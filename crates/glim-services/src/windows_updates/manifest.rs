//! The signing key is embedded in the application, never supplied by the feed.
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const FEED_URL: &str =
    "https://github.com/NeoPanic/Glim/releases/latest/download/windows-update.json";
pub const MANIFEST_LIMIT: u64 = 256 * 1024;
pub const ARCHIVE_LIMIT: u64 = 256 * 1024 * 1024;
pub const APPLICATION_ID: &str = "io.github.neopanic.glim";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub application: String,
    pub platform: String,
    pub version: String,
    pub build: u64,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub published: String,
    pub notes: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageMetadata {
    pub schema: u32,
    pub application: String,
    pub platform: String,
    pub version: String,
    pub build: u64,
    pub public_key: String,
    pub updates_enabled: bool,
}
pub fn public_key(value: &str) -> Result<VerifyingKey> {
    let bytes: [u8; 32] = STANDARD
        .decode(value.trim())
        .context("Invalid update public key")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("Update public key must contain 32 bytes"))?;
    VerifyingKey::from_bytes(&bytes).context("Invalid Ed25519 public key")
}
pub fn verify(bytes: &[u8], signature: &str, key: &str) -> Result<Manifest> {
    ensure!(
        bytes.len() as u64 <= MANIFEST_LIMIT,
        "Update manifest exceeds size limit"
    );
    let signature = Signature::from_slice(
        &STANDARD
            .decode(signature.trim())
            .context("Invalid update signature")?,
    )?;
    public_key(key)?
        .verify_strict(bytes, &signature)
        .context("Update manifest signature is invalid")?;
    let manifest: Manifest = serde_json::from_slice(bytes).context("Invalid update manifest")?;
    validate(&manifest)?;
    Ok(manifest)
}
pub fn validate(manifest: &Manifest) -> Result<()> {
    ensure!(
        manifest.schema == 1
            && manifest.application == APPLICATION_ID
            && manifest.platform == "windows-x64",
        "Update application or platform differs"
    );
    let version = semver::Version::parse(&manifest.version)?;
    ensure!(
        version.pre.is_empty() && version.build.is_empty(),
        "Only stable update versions are supported"
    );
    ensure!(
        manifest.build > 0 && manifest.size > 0 && manifest.size <= ARCHIVE_LIMIT,
        "Invalid update build or archive size"
    );
    let expected = format!(
        "https://github.com/NeoPanic/Glim/releases/download/v{}/Glim-{}-windows-x64.zip",
        manifest.version, manifest.version
    );
    ensure!(
        manifest.url == expected,
        "Update archive must belong to its immutable Glim release"
    );
    ensure!(
        manifest.sha256.len() == 64
            && manifest
                .sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "Invalid update archive digest"
    );
    ensure!(
        manifest.notes.len() <= 64 * 1024 && manifest.published.len() <= 128,
        "Update text exceeds size limit"
    );
    Ok(())
}
pub fn is_newer(manifest: &Manifest, version: &str, build: u64) -> Result<bool> {
    Ok(manifest.build > build
        && semver::Version::parse(&manifest.version)? > semver::Version::parse(version)?)
}
pub fn signature_url(manifest: &Manifest) -> String {
    format!(
        "https://github.com/NeoPanic/Glim/releases/download/v{}/windows-update.json.sig",
        manifest.version
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    pub(super) fn fixture() -> Manifest {
        Manifest { schema: 1, application: APPLICATION_ID.into(), platform: "windows-x64".into(), version: "0.1.15".into(), build: 16, url: "https://github.com/NeoPanic/Glim/releases/download/v0.1.15/Glim-0.1.15-windows-x64.zip".into(), size: 100, sha256: "a".repeat(64), published: "2026-10-12T00:00:00Z".into(), notes: "Update notes".into() }
    }
    #[test]
    fn authenticated_manifest_binds_identity_version_platform_and_archive() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let public = STANDARD.encode(key.verifying_key().as_bytes());
        let manifest = fixture();
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let signature = STANDARD.encode(key.sign(&bytes).to_bytes());
        let verified = verify(&bytes, &signature, &public).unwrap();
        assert!(is_newer(&verified, "0.1.14", 15).unwrap());
        assert!(!is_newer(&verified, "0.1.15", 16).unwrap());
        assert!(!is_newer(&verified, "0.1.16", 17).unwrap());
        let mut changed = bytes.clone();
        changed[0] ^= 1;
        assert!(verify(&changed, &signature, &public).is_err());
        assert!(
            verify(
                &bytes,
                &signature,
                &STANDARD.encode(SigningKey::from_bytes(&[8; 32]).verifying_key().as_bytes())
            )
            .is_err()
        );
        for field in ["url", "platform", "application"] {
            let mut value = serde_json::to_value(&manifest).unwrap();
            value[field] = "foreign".into();
            let bytes = serde_json::to_vec(&value).unwrap();
            let signature = STANDARD.encode(key.sign(&bytes).to_bytes());
            assert!(verify(&bytes, &signature, &public).is_err());
        }
    }
}
