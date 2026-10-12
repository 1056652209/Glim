//! Portable Windows updates: a signed feed, cancellable OS HTTPS transport and a separate installer.
pub mod install;
pub mod manifest;

use anyhow::{Context, Result, ensure};
use glim_core::update::{UpdateMode, UpdateSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    mode: UpdateMode,
    last_check: u64,
    skipped_build: u64,
    pending: Option<PathBuf>,
    install_on_quit: bool,
}
struct Shared {
    snapshot: Mutex<UpdateSnapshot>,
    generation: AtomicU64,
    stopped: AtomicBool,
    committed: AtomicBool,
    network: Mutex<Option<Child>>,
    installer: Mutex<Option<Child>>,
}
#[derive(Clone)]
pub struct Client {
    shared: Arc<Shared>,
    sender: mpsc::Sender<(String, u64)>,
}
pub struct Runner {
    client: Client,
    receiver: mpsc::Receiver<(String, u64)>,
    executable: PathBuf,
    key: String,
    data: PathBuf,
    preferences: Preferences,
    candidate: Option<(manifest::Manifest, Vec<u8>, String)>,
    pending: Option<PathBuf>,
}
pub fn create(
    executable: PathBuf,
    key: String,
    version: String,
    build: u64,
) -> Result<(Client, Runner)> {
    manifest::public_key(&key)?;
    let directory = executable
        .parent()
        .context("Missing installation directory")?
        .to_path_buf();
    let snapshot = UpdateSnapshot {
        enabled: true,
        phase: "idle".into(),
        current_version: version,
        current_build: build.to_string(),
        bundle_path: directory,
        ..Default::default()
    };
    let shared = Arc::new(Shared {
        snapshot: Mutex::new(snapshot),
        generation: AtomicU64::new(0),
        stopped: AtomicBool::new(false),
        committed: AtomicBool::new(false),
        network: Mutex::new(None),
        installer: Mutex::new(None),
    });
    let (sender, receiver) = mpsc::channel();
    let client = Client { shared, sender };
    let runner = Runner {
        client: client.clone(),
        receiver,
        executable,
        key,
        data: PathBuf::new(),
        preferences: Preferences::default(),
        candidate: None,
        pending: None,
    };
    Ok((client, runner))
}
impl Client {
    pub fn snapshot(&self) -> UpdateSnapshot {
        self.shared.snapshot.lock().unwrap().clone()
    }
    fn change(&self, update: impl FnOnce(&mut UpdateSnapshot)) {
        update(&mut self.shared.snapshot.lock().unwrap());
    }
    fn kill(slot: &Mutex<Option<Child>>) {
        if let Some(child) = slot.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
    }
    pub fn command(&self, command: &str) {
        if command == "commit-exit" {
            self.shared.committed.store(true, Ordering::SeqCst);
            return;
        }
        if matches!(
            command,
            "cancel" | "later" | "skip" | "approval-denied" | "withdraw-approval" | "check"
        ) {
            self.shared.generation.fetch_add(1, Ordering::SeqCst);
            Self::kill(&self.shared.network);
            if !self.shared.committed.load(Ordering::SeqCst) {
                Self::kill(&self.shared.installer);
            }
            self.change(|s| {
                s.exit_requested = false;
                s.needs_install_approval = false;
            });
        }
        if matches!(command, "cancel" | "later" | "skip") {
            self.change(|s| {
                s.phase = "cancelled".into();
                s.can_cancel = false;
                s.can_install = false;
                s.can_download = false;
                s.install_on_quit = false;
            });
        }
        if matches!(command, "manual" | "notify" | "automatic") {
            self.change(|s| {
                s.mode = match command {
                    "manual" => UpdateMode::Manual,
                    "automatic" => UpdateMode::Automatic,
                    _ => UpdateMode::Notify,
                };
                if s.mode != UpdateMode::Automatic {
                    s.install_on_quit = false;
                }
            });
        }
        if command == "validate-exit" {
            self.change(|s| s.exit_permitted = s.can_install);
            return;
        }
        let _ = self.sender.send((
            command.into(),
            self.shared.generation.load(Ordering::SeqCst),
        ));
    }
    pub fn stop(&self) {
        self.shared.stopped.store(true, Ordering::SeqCst);
        self.shared.generation.fetch_add(1, Ordering::SeqCst);
        Self::kill(&self.shared.network);
        if !self.shared.committed.load(Ordering::SeqCst) {
            Self::kill(&self.shared.installer);
        }
        let _ = self
            .sender
            .send(("stop".into(), self.shared.generation.load(Ordering::SeqCst)));
    }
    fn active(&self, generation: u64) -> bool {
        !self.shared.stopped.load(Ordering::SeqCst)
            && self.shared.generation.load(Ordering::SeqCst) == generation
    }
}
fn quiet(command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    #[cfg(not(target_os = "windows"))]
    let _ = command;
}
fn curl() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        Ok(PathBuf::from(
            std::env::var_os("SystemRoot").context("Windows system directory is unavailable")?,
        )
        .join("System32/curl.exe"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(PathBuf::from("/usr/bin/curl"))
    }
}
#[cfg(target_os = "windows")]
fn powershell() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("SystemRoot").context("Windows system directory is unavailable")?,
    )
    .join("System32/WindowsPowerShell/v1.0/powershell.exe"))
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
impl Runner {
    pub fn run(mut self) {
        if let Err(error) = self.initialize() {
            self.client.change(|s| {
                s.enabled = false;
                s.phase = "disabled".into();
                s.message = format!("Cannot enable Windows updates: {error:#}");
            });
            return;
        }
        while !self.client.shared.stopped.load(Ordering::SeqCst) {
            let (command, generation) = match self.receiver.recv_timeout(Duration::from_secs(60)) {
                Ok(command) => command,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if self.client.snapshot().mode == UpdateMode::Manual
                        || now().saturating_sub(self.preferences.last_check) < 86400
                    {
                        continue;
                    }
                    (
                        "daily".into(),
                        self.client.shared.generation.load(Ordering::SeqCst),
                    )
                }
            };
            if command == "stop" {
                break;
            }
            if matches!(
                command.as_str(),
                "start" | "check" | "daily" | "download" | "install-approved" | "quit-approved"
            ) && !self.client.active(generation)
            {
                continue;
            }
            let result = self.dispatch(&command, generation);
            if let Err(error) = result
                && self.client.active(generation)
            {
                if matches!(command.as_str(), "install-approved" | "quit-approved") {
                    Client::kill(&self.client.shared.installer);
                }
                self.client.change(|s| {
                    s.phase = "error".into();
                    s.message = format!("{error:#}");
                    s.can_cancel = false;
                    s.can_install = false;
                    s.install_on_quit = false;
                    s.exit_requested = false;
                });
            }
        }
    }
    fn initialize(&mut self) -> Result<()> {
        ensure!(
            self.executable
                .file_name()
                .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("Glim.exe")),
            "Use the packaged Glim.exe before enabling automatic updates"
        );
        let directory = install::validate_directory(
            self.executable
                .parent()
                .context("Missing installation directory")?,
        )?;
        self.executable = directory.join("Glim.exe");
        let id = format!(
            "{:x}",
            sha2::Sha256::digest(directory.to_string_lossy().as_bytes())
        );
        self.data = crate::updates::state_directory()?
            .join("WindowsUpdates")
            .join(id);
        fs::create_dir_all(self.data.join("staging"))?;
        let preferences = self.data.join("preferences.json");
        if preferences.exists() {
            ensure!(
                fs::metadata(&preferences)?.len() <= 16 * 1024,
                "Update preferences exceed size limit"
            );
            self.preferences = serde_json::from_slice(&fs::read(preferences)?)?;
        }
        self.client.change(|s| {
            s.bundle_path = directory;
            s.mode = self.preferences.mode;
        });
        if let Some(stage) = self.preferences.pending.clone()
            && stage.parent() == Some(self.data.join("staging").as_path())
        {
            if stage.join("result.json").exists() {
                let path = stage.join("result.json");
                ensure!(
                    fs::metadata(&path)?.len() <= 16 * 1024,
                    "Update result exceeds size limit"
                );
                let result: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
                if result["success"] == false {
                    self.client.change(|s| {
                        s.message = result["message"]
                            .as_str()
                            .unwrap_or("Update installation failed")
                            .into()
                    });
                }
                self.preferences.pending = None;
                self.save_preferences()?;
            } else if let Ok(candidate) = self.read_stage(&stage) {
                let snapshot = self.client.snapshot();
                if manifest::is_newer(
                    &candidate.0,
                    &snapshot.current_version,
                    snapshot.current_build.parse()?,
                )? {
                    self.candidate = Some(candidate);
                    self.pending = Some(stage);
                    self.ready();
                }
            }
        }
        Ok(())
    }
    fn save_preferences(&self) -> Result<()> {
        install::write_json(&self.data.join("preferences.json"), &self.preferences)
    }
    fn dispatch(&mut self, command: &str, generation: u64) -> Result<()> {
        match command {
            "start"
                if self.client.snapshot().mode != UpdateMode::Manual && self.pending.is_none() =>
            {
                self.check(false, generation)
            }
            "check" => self.check(true, generation),
            "daily" => self.check(false, generation),
            "download" => self.download(generation),
            "manual" | "notify" | "automatic" => {
                self.preferences.mode = self.client.snapshot().mode;
                self.preferences.install_on_quit = self.client.snapshot().install_on_quit;
                self.save_preferences()
            }
            "install-on-quit" if self.pending.is_some() => {
                self.preferences.install_on_quit = true;
                self.client.change(|s| s.install_on_quit = true);
                self.save_preferences()
            }
            "cancel" | "skip" | "later" => {
                if command == "skip" {
                    self.preferences.skipped_build = self
                        .candidate
                        .as_ref()
                        .map(|v| v.0.build)
                        .unwrap_or_default();
                }
                if command != "later" {
                    self.pending = None;
                    self.preferences.pending = None;
                    self.preferences.install_on_quit = false;
                }
                self.save_preferences()
            }
            "install-approved" => self.prepare_installer(true, generation),
            "quit-approved" => self.prepare_installer(false, generation),
            "approval-denied" | "withdraw-approval" => {
                self.client.change(|s| {
                    s.phase = if self.pending.is_some() {
                        "ready"
                    } else {
                        "idle"
                    }
                    .into();
                    s.exit_requested = false;
                    s.can_install = self.pending.is_some();
                });
                Ok(())
            }
            _ => Ok(()),
        }
    }
    fn transfer(
        &self,
        url: &str,
        path: &Path,
        limit: u64,
        generation: u64,
        progress: bool,
    ) -> Result<()> {
        ensure!(self.client.active(generation), "Update cancelled");
        let mut command = Command::new(curl()?);
        command
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--max-redirs",
                "5",
                "--connect-timeout",
                "15",
                "--max-time",
                "600",
                "--speed-time",
                "15",
                "--speed-limit",
                "1",
                "--header",
                "Cache-Control: no-cache",
                "--max-filesize",
                &limit.to_string(),
                "--output",
            ])
            .arg(path)
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        quiet(&mut command);
        let mut child = command
            .spawn()
            .context("Windows HTTPS downloader is unavailable")?;
        {
            let mut slot = self.client.shared.network.lock().unwrap();
            if !self.client.active(generation) {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("Update cancelled");
            }
            *slot = Some(child);
        }
        let status = loop {
            ensure!(self.client.active(generation), "Update cancelled");
            let length = fs::metadata(path).map(|m| m.len()).unwrap_or_default();
            if length > limit {
                Client::kill(&self.client.shared.network);
                anyhow::bail!("Update response exceeds size limit");
            }
            if progress {
                self.client.change(|s| s.received = length);
            }
            if let Some(status) = self
                .client
                .shared
                .network
                .lock()
                .unwrap()
                .as_mut()
                .context("Missing downloader process")?
                .try_wait()?
            {
                break status;
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        let mut child = self
            .client
            .shared
            .network
            .lock()
            .unwrap()
            .take()
            .context("Missing downloader process")?;
        let mut error = String::new();
        use std::io::Read;
        if let Some(stderr) = child.stderr.take() {
            stderr.take(4096).read_to_string(&mut error)?;
        }
        ensure!(status.success(), "Update download failed: {}", error.trim());
        ensure!(
            self.client.active(generation) && fs::metadata(path)?.len() <= limit,
            "Update cancelled or response exceeds size limit"
        );
        Ok(())
    }
    fn check(&mut self, explicit: bool, generation: u64) -> Result<()> {
        self.client.change(|s| {
            s.phase = "checking".into();
            s.message.clear();
            s.can_cancel = true;
            s.can_download = false;
        });
        let temp = tempfile::tempdir_in(&self.data)?;
        let path = temp.path().join("manifest.json");
        self.transfer(
            manifest::FEED_URL,
            &path,
            manifest::MANIFEST_LIMIT,
            generation,
            false,
        )?;
        let bytes = fs::read(path)?;
        let tentative: manifest::Manifest = serde_json::from_slice(&bytes)?;
        manifest::validate(&tentative)?;
        let signature_path = temp.path().join("manifest.sig");
        self.transfer(
            &manifest::signature_url(&tentative),
            &signature_path,
            1024,
            generation,
            false,
        )?;
        let signature = fs::read_to_string(signature_path)?;
        let candidate = manifest::verify(&bytes, &signature, &self.key)?;
        let snapshot = self.client.snapshot();
        self.preferences.last_check = now();
        self.save_preferences()?;
        let timestamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)?;
        if !manifest::is_newer(
            &candidate,
            &snapshot.current_version,
            snapshot.current_build.parse()?,
        )? || (!explicit && candidate.build == self.preferences.skipped_build)
        {
            self.client.change(|s| {
                s.phase = "current".into();
                s.can_cancel = false;
                s.can_download = false;
                s.last_check = timestamp;
                s.message = "You’re up to date".into();
            });
            return Ok(());
        }
        if let Some(stage) = &self.pending
            && self
                .read_stage(stage)
                .is_ok_and(|old| old.0.build == candidate.build && old.0.sha256 == candidate.sha256)
        {
            self.candidate = Some((candidate, bytes, signature));
            self.ready();
            return Ok(());
        }
        self.pending = None;
        self.preferences.pending = None;
        self.preferences.install_on_quit = false;
        self.save_preferences()?;
        self.client.change(|s| {
            s.version = candidate.version.clone();
            s.build = candidate.build.to_string();
            s.notes = candidate.notes.clone();
            s.published = candidate.published.clone();
            s.phase = "available".into();
            s.can_cancel = false;
            s.can_download = true;
            s.can_install = false;
            s.install_on_quit = false;
            s.last_check = timestamp;
            s.attention += 1;
        });
        self.candidate = Some((candidate, bytes, signature));
        if self.client.snapshot().mode == UpdateMode::Automatic {
            self.download(generation)?;
        }
        Ok(())
    }
    fn read_stage(&self, stage: &Path) -> Result<(manifest::Manifest, Vec<u8>, String)> {
        ensure!(
            fs::metadata(stage.join("manifest.json"))?.len() <= manifest::MANIFEST_LIMIT
                && fs::metadata(stage.join("manifest.sig"))?.len() <= 1024,
            "Staged manifest exceeds size limit"
        );
        let bytes = fs::read(stage.join("manifest.json"))?;
        let signature = fs::read_to_string(stage.join("manifest.sig"))?;
        let candidate = manifest::verify(&bytes, &signature, &self.key)?;
        ensure!(
            fs::metadata(stage.join("update.zip"))?.len() == candidate.size
                && install::digest(&stage.join("update.zip"))? == candidate.sha256,
            "Staged update archive has changed"
        );
        Ok((candidate, bytes, signature))
    }
    fn download(&mut self, generation: u64) -> Result<()> {
        let (candidate, bytes, signature) = self
            .candidate
            .as_ref()
            .context("Check for updates before downloading")?
            .clone();
        let stage = tempfile::Builder::new()
            .prefix(".glim-download-")
            .tempdir_in(self.data.join("staging"))?;
        self.client.change(|s| {
            s.phase = "downloading".into();
            s.can_cancel = true;
            s.can_download = false;
            s.progress_known = true;
            s.received = 0;
            s.expected = candidate.size;
        });
        self.transfer(
            &candidate.url,
            &stage.path().join("update.zip"),
            candidate.size,
            generation,
            true,
        )?;
        self.client.change(|s| s.phase = "extracting".into());
        let payload = tempfile::tempdir_in(stage.path())?;
        install::extract(
            &stage.path().join("update.zip"),
            payload.path(),
            &candidate,
            &self.key,
        )?;
        ensure!(self.client.active(generation), "Update cancelled");
        fs::write(stage.path().join("manifest.json"), bytes)?;
        fs::write(stage.path().join("manifest.sig"), signature)?;
        drop(payload);
        let path = stage.keep();
        self.preferences.pending = Some(path.clone());
        self.preferences.install_on_quit = self.client.snapshot().mode == UpdateMode::Automatic;
        self.save_preferences()?;
        self.pending = Some(path);
        self.ready();
        Ok(())
    }
    fn ready(&self) {
        if let Some((candidate, _, _)) = &self.candidate {
            self.client.change(|s| {
                s.phase = "ready".into();
                s.version = candidate.version.clone();
                s.build = candidate.build.to_string();
                s.notes = candidate.notes.clone();
                s.published = candidate.published.clone();
                s.can_cancel = true;
                s.can_download = false;
                s.can_install = true;
                s.install_on_quit = self.preferences.install_on_quit;
            });
        }
    }
    fn prepare_installer(&self, relaunch: bool, generation: u64) -> Result<()> {
        let stage = self
            .pending
            .as_ref()
            .context("Download a verified update before installing")?;
        self.read_stage(stage)?;
        let directory = self
            .executable
            .parent()
            .context("Missing installation directory")?;
        install::validate_directory(directory)?;
        #[cfg(target_os = "windows")]
        {
            let path = self.executable.to_string_lossy();
            let normalized = if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
                format!(r"\\{unc}")
            } else {
                path.trim_start_matches(r"\\?\").to_owned()
            };
            let script = format!(
                "$ErrorActionPreference = 'Stop'; $glimOthers = Get-CimInstance Win32_Process | Where-Object {{ $_.ExecutablePath -eq $env:GLIM_UPDATE_EXE -and $_.ProcessId -ne {} }}; if ($glimOthers) {{ exit 1 }}",
                std::process::id()
            );
            let mut command = Command::new(powershell()?);
            command.env("GLIM_UPDATE_EXE", normalized).args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &script,
            ]);
            quiet(&mut command);
            ensure!(
                command.output()?.status.success(),
                "Quit other running copies of Glim before updating."
            );
        }
        ensure!(
            self.client.active(generation),
            "Update approval was withdrawn"
        );
        let snapshot = self.client.snapshot();
        let helper = stage.join("GlimUpdater.exe");
        fs::copy(&self.executable, &helper)?;
        let job = install::InstallJob {
            schema: 1,
            parent_pid: std::process::id(),
            target: directory.to_path_buf(),
            source_hash: install::digest(&self.executable)?,
            source_version: snapshot.current_version,
            source_build: snapshot.current_build.parse()?,
            relaunch,
            nonce: format!(
                "{}-{}",
                std::process::id(),
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
            ),
        };
        let ready = stage.join("ready.json");
        if ready.exists() {
            fs::remove_file(&ready)?;
        }
        let job_path = stage.join("job.json");
        install::write_json(&job_path, &job)?;
        let mut command = Command::new(&helper);
        command.arg("--install-update").arg(job_path);
        quiet(&mut command);
        let mut child = command
            .spawn()
            .context("Cannot start the update installer")?;
        let mut slot = self.client.shared.installer.lock().unwrap();
        if !self.client.active(generation) {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("Update approval was withdrawn");
        }
        *slot = Some(child);
        drop(slot);
        let started = std::time::Instant::now();
        loop {
            ensure!(
                self.client.active(generation),
                "Update approval was withdrawn"
            );
            if ready.is_file() {
                ensure!(
                    fs::metadata(&ready)?.len() <= 1024,
                    "Installer readiness exceeds size limit"
                );
                let value: serde_json::Value = serde_json::from_slice(&fs::read(&ready)?)?;
                if value["nonce"].as_str() == Some(job.nonce.as_str()) {
                    break;
                }
            }
            let status = self
                .client
                .shared
                .installer
                .lock()
                .unwrap()
                .as_mut()
                .context("Missing installer process")?
                .try_wait()?;
            ensure!(
                status.is_none(),
                "Update installer exited before verifying its package; Glim remains open"
            );
            if started.elapsed() > Duration::from_secs(20) {
                Client::kill(&self.client.shared.installer);
                anyhow::bail!("Update installer did not become ready; Glim remains open");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        self.client.change(|s| {
            s.phase = "installing".into();
            s.exit_requested = true;
            s.exit_permitted = true;
            s.can_cancel = false;
        });
        Ok(())
    }
}
use sha2::Digest;
