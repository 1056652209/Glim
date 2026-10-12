//! Update preparation and one-time session recovery. Run on the background executor.
use anyhow::{Context, Result, ensure};
use glim_core::update::UpdateSession;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub fn state_directory() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(
        std::env::var_os("APPDATA").context("Application data directory is unavailable")?,
    );
    #[cfg(not(target_os = "windows"))]
    let base = PathBuf::from(std::env::var_os("HOME").context("Home directory is unavailable")?)
        .join("Library/Application Support");
    Ok(base.join("Glim"))
}
fn digest(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
#[cfg(not(target_os = "windows"))]
fn run(command: &mut std::process::Command) -> Result<()> {
    let output = command.output()?;
    ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// Preserve the signed complete bundle, never user documents or preferences.
/// Returns an immutable, verified archive; installation remains Sparkle's job.
pub fn prepare_backup(bundle: &Path) -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    return crate::windows_updates::install::prepare_backup(
        bundle,
        &state_directory()?.join("UpdateBackups"),
    );
    #[cfg(not(target_os = "windows"))]
    prepare_backup_at(bundle, &state_directory()?.join("UpdateBackups"))
}
#[cfg(not(target_os = "windows"))]
fn prepare_backup_at(bundle: &Path, backups: &Path) -> Result<PathBuf> {
    ensure!(
        cfg!(target_os = "macos"),
        "Automatic installation requires macOS"
    );
    let bundle = bundle.canonicalize()?;
    ensure!(
        bundle.extension().is_some_and(|v| v == "app"),
        "Run an installed Glim.app before updating"
    );
    ensure!(
        !bundle.starts_with("/Volumes")
            && !bundle
                .components()
                .any(|p| p.as_os_str() == "AppTranslocation"),
        "Install Glim in Applications before updating"
    );
    let info = bundle.join("Contents/Info.plist");
    let identity = std::process::Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleIdentifier"])
        .arg(&info)
        .output()?;
    ensure!(
        identity.status.success()
            && String::from_utf8_lossy(&identity.stdout).trim() == "io.github.jony4.glimpse",
        "Unexpected application identity"
    );
    run(std::process::Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&bundle))?;
    let fingerprint = format!(
        "{}:{}:{}",
        digest(&bundle.join("Contents/MacOS/glim"))?,
        digest(&info)?,
        digest(&bundle.join("Contents/_CodeSignature/CodeResources"))?
    );
    let identifier = format!("{:x}", Sha256::digest(fingerprint.as_bytes()));
    fs::create_dir_all(backups)?;
    let archive = backups.join(format!("Glim-{identifier}.zip"));
    let receipt = archive.with_extension("sha256");
    if archive.is_file()
        && fs::read_to_string(&receipt)
            .ok()
            .is_some_and(|expected| digest(&archive).is_ok_and(|actual| actual == expected))
    {
        return Ok(archive);
    }
    let temp = tempfile::Builder::new()
        .prefix(".glim-backup-")
        .tempdir_in(backups)?;
    let copy = temp.path().join("Glim.app");
    run(std::process::Command::new("/usr/bin/ditto")
        .arg(&bundle)
        .arg(&copy))?;
    run(std::process::Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&copy))?;
    let prepared = temp.path().join("Glim.zip");
    run(std::process::Command::new("/usr/bin/ditto")
        .args(["-c", "-k", "--sequesterRsrc", "--keepParent"])
        .arg(&copy)
        .arg(&prepared))?;
    let hash = digest(&prepared)?;
    fs::rename(&prepared, &archive)?;
    fs::write(&receipt, hash)?;
    Ok(archive)
}

pub fn verify_backup(archive: &Path) -> Result<()> {
    let expected = fs::read_to_string(archive.with_extension("sha256"))
        .context("The recovery backup receipt is missing")?;
    ensure!(
        digest(archive)? == expected,
        "The recovery backup has changed; prepare a new backup before updating"
    );
    Ok(())
}

fn write_session_at(directory: &Path, session: &UpdateSession) -> Result<()> {
    validate_session(session)?;
    fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer(&mut file, session)?;
    ensure!(
        file.as_file().metadata()?.len() <= 8 * 1024 * 1024,
        "Update session exceeds size limit"
    );
    file.flush()?;
    file.as_file().sync_all()?;
    file.persist(directory.join("update-session.json"))?;
    Ok(())
}
fn validate_session(session: &UpdateSession) -> Result<()> {
    ensure!(session.schema == 1, "Unsupported update session format");
    ensure!(
        session.workspaces.len() <= 64,
        "Too many windows to restore"
    );
    for workspace in &session.workspaces {
        ensure!(
            workspace.roots.len() <= 256 && workspace.tabs.len() <= 2000,
            "Too many items to restore"
        );
        ensure!(
            workspace.active.is_none_or(|i| i < workspace.tabs.len()),
            "Invalid active tab"
        );
    }
    Ok(())
}
pub fn save_session(session: &UpdateSession) -> Result<()> {
    write_session_at(&state_directory()?, session)
}
pub fn clear_session() -> Result<()> {
    match fs::remove_file(state_directory()?.join("update-session.json")) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
/// Consume or quarantine by rename: a bad session cannot loop on every launch.
/// Restore even after an installer failure left the old build in place. Explicit
/// cancellation clears the checkpoint; loading always consumes it at most once.
pub fn take_session(current_build: &str) -> Result<Option<UpdateSession>> {
    take_session_at(&state_directory()?, current_build)
}
fn take_session_at(directory: &Path, current_build: &str) -> Result<Option<UpdateSession>> {
    let path = directory.join("update-session.json");
    if !path.is_file() {
        return Ok(None);
    }
    let read = || -> Result<UpdateSession> {
        ensure!(
            fs::metadata(&path)?.len() <= 8 * 1024 * 1024,
            "Update session exceeds size limit"
        );
        let session: UpdateSession = serde_json::from_slice(&fs::read(&path)?)?;
        validate_session(&session)?;
        Ok(session)
    };
    let session = match read() {
        Ok(session) => session,
        Err(error) => {
            fs::rename(&path, directory.join("rejected-update-session.json"))?;
            return Err(error);
        }
    };
    ensure!(
        !current_build.is_empty(),
        "Cannot identify the running build"
    );
    // The version may skip ahead if another release was installed manually.
    fs::rename(&path, directory.join("last-update-session.json"))?;
    Ok(Some(session))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glim_core::update::{SessionTab, WorkspaceSession};
    #[test]
    fn checkpoint_recovers_failed_installs_and_is_consumed_once_after_upgrade() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let session = UpdateSession {
            schema: 1,
            source_build: "12".into(),
            target_build: "13".into(),
            workspaces: vec![WorkspaceSession {
                roots: vec!["/tmp/project".into()],
                tabs: vec![SessionTab::File {
                    path: "/tmp/project/notes.md".into(),
                    preview: false,
                }],
                active: Some(0),
            }],
        };
        write_session_at(directory.path(), &session)?;
        assert_eq!(
            take_session_at(directory.path(), "12")?,
            Some(session.clone())
        );
        write_session_at(directory.path(), &session)?;
        assert_eq!(take_session_at(directory.path(), "13")?, Some(session));
        assert!(take_session_at(directory.path(), "13")?.is_none());
        Ok(())
    }
    #[test]
    fn invalid_active_tab_never_replaces_a_saved_checkpoint() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let session = UpdateSession {
            schema: 1,
            source_build: "12".into(),
            target_build: "13".into(),
            workspaces: vec![WorkspaceSession {
                active: Some(3),
                ..Default::default()
            }],
        };
        assert!(write_session_at(directory.path(), &session).is_err());
        assert!(!directory.path().join("update-session.json").exists());
        Ok(())
    }
    #[test]
    fn corrupted_checkpoint_is_quarantined_instead_of_repeating_on_launch() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("update-session.json"), "not json")?;
        assert!(take_session_at(dir.path(), "13").is_err());
        assert!(take_session_at(dir.path(), "13")?.is_none());
        assert!(dir.path().join("rejected-update-session.json").exists());
        Ok(())
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn signed_backup_survives_source_replacement_and_detects_tampering() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let bundle = dir.path().join("Glim.app");
        let contents = bundle.join("Contents");
        fs::create_dir_all(contents.join("MacOS"))?;
        let source = dir.path().join("main.c");
        fs::write(&source, "int main(void) { return 0; }")?;
        let binary = contents.join("MacOS/glim");
        run(std::process::Command::new("cc")
            .arg(&source)
            .arg("-o")
            .arg(&binary))?;
        fs::write(
            contents.join("Info.plist"),
            r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>io.github.jony4.glimpse</string><key>CFBundleExecutable</key><string>glim</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleVersion</key><string>1</string></dict></plist>"#,
        )?;
        run(std::process::Command::new("codesign")
            .args(["--force", "--sign", "-", "--timestamp=none"])
            .arg(&bundle))?;
        let backups = dir.path().join("backups");
        let archive = prepare_backup_at(&bundle, &backups)?;
        assert_eq!(prepare_backup_at(&bundle, &backups)?, archive);
        verify_backup(&archive)?;
        fs::write(&binary, "damaged application")?;
        assert!(prepare_backup_at(&bundle, &backups).is_err());
        let restored = dir.path().join("restored");
        run(std::process::Command::new("ditto")
            .args(["-x", "-k"])
            .arg(&archive)
            .arg(&restored))?;
        run(std::process::Command::new("codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(restored.join("Glim.app")))?;
        fs::OpenOptions::new()
            .append(true)
            .open(&archive)?
            .write_all(b"tampered")?;
        assert!(verify_backup(&archive).is_err());
        Ok(())
    }
}
