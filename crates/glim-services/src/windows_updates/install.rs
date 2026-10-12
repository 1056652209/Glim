//! Install only the four files owned by the portable distribution, keeping user files intact.
use super::manifest::{APPLICATION_ID, ARCHIVE_LIMIT, Manifest, PackageMetadata};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MANAGED_FILES: [&str; 4] = [
    "Glim.exe",
    "LICENSE.txt",
    "Read Me.txt",
    "glim-release.json",
];
pub fn digest(path: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    let mut file = fs::File::open(path)?;
    let mut bytes = [0; 64 * 1024];
    loop {
        let count = file.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn real_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "Managed application files cannot be links or directories"
    );
    Ok(())
}
pub fn validate_directory(directory: &Path) -> Result<PathBuf> {
    ensure!(
        !fs::symlink_metadata(directory)?.file_type().is_symlink(),
        "Update directory cannot be a symbolic link"
    );
    let directory = directory.canonicalize()?;
    real_file(&directory.join("Glim.exe"))?;
    // Keep changes on the same volume and fail before shutdown for protected folders.
    let _probe = tempfile::NamedTempFile::new_in(&directory)
        .context("Move Glim to a writable folder before updating")?;
    for name in MANAGED_FILES {
        let path = directory.join(name);
        if path.exists() {
            real_file(&path)?;
        }
    }
    Ok(directory)
}
pub fn prepare_backup(directory: &Path, backups: &Path) -> Result<PathBuf> {
    let directory = validate_directory(directory)?;
    fs::create_dir_all(backups)?;
    let id = digest(&directory.join("Glim.exe"))?;
    let archive = backups.join(format!("Glim-windows-{id}.zip"));
    let receipt = archive.with_extension("sha256");
    if archive.is_file()
        && fs::read_to_string(&receipt).is_ok_and(|v| digest(&archive).is_ok_and(|d| d == v))
    {
        return Ok(archive);
    }
    let mut temporary = tempfile::NamedTempFile::new_in(backups)?;
    {
        let mut zip = zip::ZipWriter::new(temporary.as_file_mut());
        for name in MANAGED_FILES {
            let path = directory.join(name);
            if !path.exists() {
                continue;
            }
            ensure!(
                fs::metadata(&path)?.len() <= ARCHIVE_LIMIT,
                "Application file exceeds backup limit"
            );
            zip.start_file(
                name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )?;
            std::io::copy(&mut fs::File::open(path)?, &mut zip)?;
        }
        zip.finish()?;
    }
    temporary.as_file().sync_all()?;
    let hash = digest(temporary.path())?;
    temporary.persist(&archive)?;
    fs::write(receipt, hash)?;
    Ok(archive)
}
pub fn extract(archive: &Path, directory: &Path, manifest: &Manifest, key: &str) -> Result<()> {
    ensure!(
        fs::metadata(archive)?.len() == manifest.size && digest(archive)? == manifest.sha256,
        "Update archive failed size or digest verification"
    );
    let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
    ensure!(
        zip.len() <= MANAGED_FILES.len() + 1,
        "Unexpected files in update archive"
    );
    let mut seen = std::collections::HashSet::new();
    let mut total = 0;
    fs::create_dir_all(directory)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if entry.is_dir() {
            ensure!(entry.name() == "Glim/", "Unexpected archive directory");
            continue;
        }
        let name = entry
            .name()
            .strip_prefix("Glim/")
            .context("Invalid portable archive root")?
            .to_owned();
        ensure!(
            MANAGED_FILES.contains(&name.as_str()) && seen.insert(name.clone()),
            "Unexpected or duplicate archive file"
        );
        ensure!(
            entry
                .unix_mode()
                .is_none_or(|mode| mode & 0o170000 != 0o120000),
            "Update archive cannot contain symbolic links"
        );
        total += entry.size();
        ensure!(
            total <= 512 * 1024 * 1024,
            "Extracted update exceeds size limit"
        );
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(&name))?;
        let count = std::io::copy(&mut (&mut entry).take(ARCHIVE_LIMIT + 1), &mut file)?;
        ensure!(
            count <= ARCHIVE_LIMIT && count == entry.size(),
            "Archive entry exceeds size limit"
        );
        file.sync_all()?;
    }
    ensure!(
        seen.len() == MANAGED_FILES.len(),
        "Update archive lacks required application files"
    );
    let metadata_path = directory.join("glim-release.json");
    ensure!(
        fs::metadata(&metadata_path)?.len() <= 16 * 1024,
        "Package metadata exceeds size limit"
    );
    let metadata: PackageMetadata = serde_json::from_slice(&fs::read(metadata_path)?)?;
    ensure!(
        metadata.schema == 1
            && metadata.application == APPLICATION_ID
            && metadata.platform == "windows-x64"
            && metadata.version == manifest.version
            && metadata.build == manifest.build
            && metadata.public_key.trim() == key.trim()
            && metadata.updates_enabled,
        "Package identity or update configuration differs from signed manifest"
    );
    let mut exe = fs::File::open(directory.join("Glim.exe"))?;
    let mut header = [0; 64];
    exe.read_exact(&mut header)?;
    ensure!(
        &header[..2] == b"MZ",
        "Update does not contain a Windows executable"
    );
    use std::io::{Seek, SeekFrom};
    exe.seek(SeekFrom::Start(
        u32::from_le_bytes(header[60..64].try_into().unwrap()) as u64,
    ))?;
    let mut pe = [0; 6];
    exe.read_exact(&mut pe)?;
    ensure!(
        &pe[..4] == b"PE\0\0" && u16::from_le_bytes(pe[4..6].try_into().unwrap()) == 0x8664,
        "Update executable is not Windows x64"
    );
    Ok(())
}
/// Stage on the destination volume, then replace managed files with rollback on any error.
pub fn replace(directory: &Path, payload: &Path, source_hash: &str) -> Result<()> {
    replace_with_hook(directory, payload, source_hash, |_| Ok(()))
}
fn replace_with_hook(
    directory: &Path,
    payload: &Path,
    source_hash: &str,
    mut after_replace: impl FnMut(&str) -> Result<()>,
) -> Result<()> {
    let directory = validate_directory(directory)?;
    ensure!(
        digest(&directory.join("Glim.exe"))? == source_hash,
        "Installed application changed while preparing the update"
    );
    let transaction = tempfile::Builder::new()
        .prefix(".glim-update-")
        .tempdir_in(&directory)?;
    let new = transaction.path().join("new");
    let old = transaction.path().join("old");
    fs::create_dir(&new)?;
    fs::create_dir(&old)?;
    for name in MANAGED_FILES {
        fs::copy(payload.join(name), new.join(name))?;
    }
    let mut applied = Vec::new();
    let result = (|| {
        for name in MANAGED_FILES {
            let destination = directory.join(name);
            let existed = destination.exists();
            if existed {
                fs::rename(&destination, old.join(name))?;
            }
            applied.push((name, existed));
            fs::rename(new.join(name), destination)?;
            after_replace(name)?;
        }
        Ok::<_, anyhow::Error>(())
    })();
    if let Err(error) = result {
        let mut recovery_error = None;
        for (name, existed) in applied.into_iter().rev() {
            let destination = directory.join(name);
            if destination.exists()
                && let Err(e) = fs::remove_file(&destination)
            {
                recovery_error = Some(e);
                continue;
            }
            if existed && let Err(e) = fs::rename(old.join(name), destination) {
                recovery_error = Some(e);
            }
        }
        if let Some(recovery_error) = recovery_error {
            let recovery = transaction.keep();
            anyhow::bail!(
                "Update failed: {error:#}. Recovery failed: {recovery_error}; original files are retained at {}",
                recovery.display()
            );
        }
        return Err(error.context("Update failed; original application files were restored"));
    }
    Ok(())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallJob {
    pub schema: u32,
    pub parent_pid: u32,
    pub target: PathBuf,
    pub source_hash: String,
    pub source_version: String,
    pub source_build: u64,
    pub relaunch: bool,
    pub nonce: String,
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().context("Missing state directory")?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(&mut file, value)?;
    file.flush()?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
pub fn run_installer(job_path: &Path, key: &str, version: &str, build: u64) -> Result<()> {
    let stage = job_path
        .parent()
        .context("Missing update staging directory")?
        .canonicalize()?;
    ensure!(
        std::env::current_exe()?
            .parent()
            .context("Missing helper directory")?
            .canonicalize()?
            == stage,
        "Update helper and job must share their private staging directory"
    );
    ensure!(
        fs::metadata(job_path)?.len() <= 16 * 1024,
        "Update job exceeds size limit"
    );
    let job: InstallJob = serde_json::from_slice(&fs::read(job_path)?)?;
    ensure!(
        job.schema == 1
            && job.source_version == version
            && job.source_build == build
            && job.parent_pid != std::process::id(),
        "Unexpected update helper identity"
    );
    let manifest = super::manifest::verify(
        &fs::read(stage.join("manifest.json"))?,
        &fs::read_to_string(stage.join("manifest.sig"))?,
        key,
    )?;
    ensure!(
        super::manifest::is_newer(&manifest, version, build)?,
        "Update must increase both application version and build"
    );
    let state = stage
        .parent()
        .and_then(Path::parent)
        .context("Missing installer state directory")?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(state.join("install.lock"))?;
    lock.try_lock()
        .context("Another Glim update is already installing")?;
    let payload = tempfile::tempdir_in(&stage)?;
    extract(&stage.join("update.zip"), payload.path(), &manifest, key)?;
    write_json(
        &stage.join("ready.json"),
        &serde_json::json!({"nonce": job.nonce}),
    )?;
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "$glimOldProcess = Get-Process -Id {} -ErrorAction SilentlyContinue; if ($glimOldProcess -and !$glimOldProcess.WaitForExit(120000)) {{ exit 1 }}; exit 0",
            job.parent_pid
        );
        let mut command = std::process::Command::new(super::powershell()?);
        command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        super::quiet(&mut command);
        let output = command.output()?;
        ensure!(
            output.status.success(),
            "The previous Glim process did not quit; no files were changed"
        );
    }
    #[cfg(not(target_os = "windows"))]
    anyhow::bail!("The install helper requires Windows");
    #[cfg(target_os = "windows")]
    {
        let result = replace(&job.target, payload.path(), &job.source_hash);
        write_json(
            &stage.join("result.json"),
            &serde_json::json!({"success": result.is_ok(), "message": result.as_ref().err().map(|e| format!("{e:#}")), "version": manifest.version}),
        )?;
        if job.relaunch {
            std::process::Command::new(job.target.join("Glim.exe"))
                .current_dir(&job.target)
                .spawn()
                .context("Cannot relaunch Glim; open it from its installation folder")?;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use ed25519_dalek::SigningKey;
    pub(super) fn fixture(root: &Path) -> (Manifest, String, PathBuf) {
        let key = STANDARD.encode(SigningKey::from_bytes(&[7; 32]).verifying_key().as_bytes());
        let metadata = PackageMetadata {
            schema: 1,
            application: APPLICATION_ID.into(),
            platform: "windows-x64".into(),
            version: "0.1.15".into(),
            build: 16,
            public_key: key.clone(),
            updates_enabled: true,
        };
        let mut exe = vec![0; 80];
        exe[..2].copy_from_slice(b"MZ");
        exe[60..64].copy_from_slice(&64u32.to_le_bytes());
        exe[64..70].copy_from_slice(b"PE\0\0\x64\x86");
        let archive = root.join("update.zip");
        let mut writer = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        for (name, bytes) in [
            ("Glim.exe", exe),
            ("LICENSE.txt", b"license".to_vec()),
            ("Read Me.txt", b"instructions".to_vec()),
            ("glim-release.json", serde_json::to_vec(&metadata).unwrap()),
        ] {
            writer
                .start_file(
                    format!("Glim/{name}"),
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap();
        let manifest = Manifest { schema: 1, application: APPLICATION_ID.into(), platform: "windows-x64".into(), version: "0.1.15".into(), build: 16, url: "https://github.com/NeoPanic/Glim/releases/download/v0.1.15/Glim-0.1.15-windows-x64.zip".into(), size: fs::metadata(&archive).unwrap().len(), sha256: digest(&archive).unwrap(), published: "2026-10-12T00:00:00Z".into(), notes: String::new() };
        (manifest, key, archive)
    }
    #[test]
    fn verified_package_replaces_only_managed_files_and_backup_is_recoverable() {
        let root = tempfile::tempdir().unwrap();
        let (manifest, key, archive) = fixture(root.path());
        let payload = root.path().join("payload");
        extract(&archive, &payload, &manifest, &key).unwrap();
        let target = root.path().join("portable");
        fs::create_dir(&target).unwrap();
        for name in MANAGED_FILES {
            fs::write(target.join(name), b"original").unwrap();
        }
        fs::write(target.join("my-book.epub"), b"user book").unwrap();
        let backup = prepare_backup(&target, &root.path().join("backups")).unwrap();
        crate::updates::verify_backup(&backup).unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(&backup).unwrap()).unwrap();
        let mut old = String::new();
        zip.by_name("Glim.exe")
            .unwrap()
            .read_to_string(&mut old)
            .unwrap();
        assert_eq!(old, "original");
        replace(
            &target,
            &payload,
            &digest(&target.join("Glim.exe")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            digest(&target.join("Glim.exe")).unwrap(),
            digest(&payload.join("Glim.exe")).unwrap()
        );
        assert_eq!(fs::read(target.join("my-book.epub")).unwrap(), b"user book");
        fs::write(&backup, b"tampered").unwrap();
        assert!(crate::updates::verify_backup(&backup).is_err());
    }
    #[test]
    fn rejects_tampered_archives_foreign_keys_and_unsafe_archive_paths() {
        let root = tempfile::tempdir().unwrap();
        let (mut manifest, key, archive) = fixture(root.path());
        let foreign = STANDARD.encode(SigningKey::from_bytes(&[8; 32]).verifying_key().as_bytes());
        assert!(extract(&archive, &root.path().join("foreign"), &manifest, &foreign).is_err());
        let mut bytes = fs::read(&archive).unwrap();
        bytes[0] ^= 1;
        fs::write(&archive, bytes).unwrap();
        assert!(extract(&archive, &root.path().join("tampered"), &manifest, &key).is_err());
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        zip.start_file(
            "Glim/../outside.txt",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"bad").unwrap();
        zip.finish().unwrap();
        manifest.size = fs::metadata(&archive).unwrap().len();
        manifest.sha256 = digest(&archive).unwrap();
        assert!(extract(&archive, &root.path().join("unsafe"), &manifest, &key).is_err());
        assert!(!root.path().join("outside.txt").exists());
    }
    #[test]
    fn replacement_failure_restores_original_files() {
        let root = tempfile::tempdir().unwrap();
        let (manifest, key, archive) = fixture(root.path());
        let payload = root.path().join("payload");
        extract(&archive, &payload, &manifest, &key).unwrap();
        let target = root.path().join("portable");
        fs::create_dir(&target).unwrap();
        for name in MANAGED_FILES {
            fs::write(target.join(name), b"original").unwrap();
        }
        let hash = digest(&target.join("Glim.exe")).unwrap();
        assert!(
            replace_with_hook(&target, &payload, &hash, |name| {
                if name == "LICENSE.txt" {
                    anyhow::bail!("Injected failure after two replacements");
                }
                Ok(())
            })
            .is_err()
        );
        for name in MANAGED_FILES {
            assert_eq!(fs::read(target.join(name)).unwrap(), b"original");
        }
        // A source change invalidates the transaction before the first rename.
        fs::write(target.join("Glim.exe"), b"changed by another installer").unwrap();
        assert!(replace(&target, &payload, &hash).is_err());
        assert_eq!(fs::read(target.join("LICENSE.txt")).unwrap(), b"original");
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let _locked = fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(target.join("Read Me.txt"))
                .unwrap();
            let hash = digest(&target.join("Glim.exe")).unwrap();
            assert!(replace(&target, &payload, &hash).is_err());
            assert_eq!(
                fs::read(target.join("Glim.exe")).unwrap(),
                b"changed by another installer"
            );
            assert_eq!(fs::read(target.join("LICENSE.txt")).unwrap(), b"original");
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod process_tests {
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signer, SigningKey};
    use std::{
        process::{Child, Command, Stdio},
        time::{Duration, Instant},
    };

    struct Host(Child);
    impl Drop for Host {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn compile(directory: &Path, name: &str, new: bool) -> PathBuf {
        let source = directory.join(format!("{name}.c"));
        let body = if new {
            "FILE *f = fopen(\"relaunch-marker.txt\", \"w\"); if (f) { fputs(\"new\", f); fclose(f); } return 0;"
        } else {
            "Sleep(60000); return 0;"
        };
        fs::write(&source, format!("#include <windows.h>\n#include <stdio.h>\n#include <string.h>\nint main(int argc, char **argv) {{ if (argc > 1 && !strcmp(argv[1], \"--help\")) {{ puts(\"{name}\"); return 0; }} {body} }}\n")).unwrap();
        let executable = directory.join(format!("{name}.exe"));
        let output = Command::new("cl.exe")
            .args(["/nologo", "/O1"])
            .arg(format!("/Fe{}", executable.display()))
            .arg(source)
            .current_dir(directory)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }
    #[test]
    fn helper_process() {
        let Some(path) = std::env::var_os("GLIM_TEST_UPDATE_JOB") else {
            return;
        };
        let key = std::env::var("GLIM_TEST_UPDATE_KEY").unwrap();
        run_installer(Path::new(&path), &key, "0.1.14", 15).unwrap();
    }
    #[test]
    fn real_windows_process_wait_install_relaunch_and_tamper_rejection() {
        let root = tempfile::tempdir().unwrap();
        let old_exe = compile(root.path(), "old", false);
        let new_exe = compile(root.path(), "new", true);
        for (index, relaunch, tamper) in [(0, true, false), (1, false, false), (2, false, true)] {
            let case = root.path().join(format!("case-{index}"));
            fs::create_dir(&case).unwrap();
            let target = case.join("便携 Glim");
            fs::create_dir(&target).unwrap();
            fs::copy(&old_exe, target.join("Glim.exe")).unwrap();
            fs::write(target.join("my-book.epub"), b"user content").unwrap();
            let source_hash = digest(&target.join("Glim.exe")).unwrap();
            let host = Command::new(target.join("Glim.exe"))
                .current_dir(&target)
                .stdout(Stdio::null())
                .spawn()
                .unwrap();
            let mut host = Host(host);
            let stages = case.join("staging");
            fs::create_dir(&stages).unwrap();
            let stage = stages.join("signed");
            fs::create_dir(&stage).unwrap();
            let (mut manifest, key, archive) = super::tests::fixture(&stage);
            let metadata = PackageMetadata {
                schema: 1,
                application: APPLICATION_ID.into(),
                platform: "windows-x64".into(),
                version: manifest.version.clone(),
                build: manifest.build,
                public_key: key.clone(),
                updates_enabled: true,
            };
            let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
            for (name, bytes) in [
                ("Glim.exe", fs::read(&new_exe).unwrap()),
                ("LICENSE.txt", b"license".to_vec()),
                ("Read Me.txt", b"instructions".to_vec()),
                ("glim-release.json", serde_json::to_vec(&metadata).unwrap()),
            ] {
                zip.start_file(
                    format!("Glim/{name}"),
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
                zip.write_all(&bytes).unwrap();
            }
            zip.finish().unwrap();
            manifest.size = fs::metadata(&archive).unwrap().len();
            manifest.sha256 = digest(&archive).unwrap();
            let bytes = serde_json::to_vec(&manifest).unwrap();
            fs::write(stage.join("manifest.json"), &bytes).unwrap();
            fs::write(
                stage.join("manifest.sig"),
                STANDARD.encode(SigningKey::from_bytes(&[7; 32]).sign(&bytes).to_bytes()),
            )
            .unwrap();
            if tamper {
                let mut file = fs::OpenOptions::new().append(true).open(&archive).unwrap();
                file.write_all(b"tampered").unwrap();
            }
            let job = InstallJob {
                schema: 1,
                parent_pid: host.0.id(),
                target: target.clone(),
                source_hash: source_hash.clone(),
                source_version: "0.1.14".into(),
                source_build: 15,
                relaunch,
                nonce: format!("test-{index}"),
            };
            let job_path = stage.join("job.json");
            write_json(&job_path, &job).unwrap();
            let helper = stage.join("GlimUpdater.exe");
            fs::copy(std::env::current_exe().unwrap(), &helper).unwrap();
            let mut child = Command::new(&helper)
                .args([
                    "--exact",
                    "windows_updates::install::process_tests::helper_process",
                    "--nocapture",
                ])
                .env("GLIM_TEST_UPDATE_JOB", &job_path)
                .env("GLIM_TEST_UPDATE_KEY", &key)
                .stdout(Stdio::from(
                    fs::File::create(stage.join("helper-output.txt")).unwrap(),
                ))
                .stderr(Stdio::from(
                    fs::File::create(stage.join("helper-error.txt")).unwrap(),
                ))
                .spawn()
                .unwrap();
            let started = Instant::now();
            while !stage.join("ready.json").exists() && child.try_wait().unwrap().is_none() {
                assert!(
                    started.elapsed() < Duration::from_secs(30),
                    "Installer did not verify its package"
                );
                std::thread::sleep(Duration::from_millis(50));
            }
            assert_eq!(digest(&target.join("Glim.exe")).unwrap(), source_hash);
            if !tamper {
                assert!(stage.join("ready.json").exists());
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "Installer did not wait for the old process"
                );
                host.0.kill().unwrap();
                host.0.wait().unwrap();
            }
            let started = Instant::now();
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(30),
                    "Installer did not finish"
                );
                std::thread::sleep(Duration::from_millis(50));
            };
            assert_eq!(
                status.success(),
                !tamper,
                "helper output: {}\nhelper error: {}\nresult: {}",
                fs::read_to_string(stage.join("helper-output.txt")).unwrap_or_default(),
                fs::read_to_string(stage.join("helper-error.txt")).unwrap_or_default(),
                fs::read_to_string(stage.join("result.json")).unwrap_or_default()
            );
            assert_eq!(
                fs::read(target.join("my-book.epub")).unwrap(),
                b"user content"
            );
            if !tamper {
                assert_eq!(
                    digest(&target.join("Glim.exe")).unwrap(),
                    digest(&new_exe).unwrap()
                );
                let started = Instant::now();
                while relaunch && !target.join("relaunch-marker.txt").exists() {
                    assert!(started.elapsed() < Duration::from_secs(10));
                    std::thread::sleep(Duration::from_millis(50));
                }
                assert_eq!(target.join("relaunch-marker.txt").exists(), relaunch);
                assert!(
                    Command::new(target.join("Glim.exe"))
                        .arg("--help")
                        .output()
                        .unwrap()
                        .status
                        .success()
                );
            }
        }
    }
}
