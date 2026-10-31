//! In-app updates: looks for a newer release, downloads its setup, checks the
//! minisign signature against the key built into this program and starts the
//! setup. The release publishes `latest.json` (same format as the Tauri
//! updater, so older versions can update to this one) next to the setup.

use std::path::PathBuf;

use serde::Deserialize;

use crate::proc::hidden;
use crate::remote::is_newer;

/// Where the latest release lists its setup and signature.
const ENDPOINT: &str = "https://github.com/Niklas970/faderust/releases/latest/download/latest.json";

/// The release signing key, as the Tauri updater stores it: the public key
/// file, base64 encoded.
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDg2QkYyMEVDMUZBNjkzNkEKUldScWs2WWY3Q0MvaHBVK25RbzlRMXBaUXN0NTJPVTcvTW5YTWt4aWNtOFRhdU1vT0pCU0N0R20K";

const PLATFORM: &str = "windows-x86_64";

/// A newer release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub version: String,
    url: String,
    /// The `.sig` file's content (base64 of the minisign signature).
    signature: String,
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    platforms: std::collections::HashMap<String, Platform>,
}

#[derive(Deserialize)]
struct Platform {
    url: String,
    signature: String,
}

/// The update `json` offers over `current`, if it is newer and has a setup
/// for this platform.
pub fn parse_manifest(json: &str, current: &str) -> Result<Option<Update>, String> {
    let m: Manifest = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if !is_newer(m.version.trim_start_matches('v'), current) {
        return Ok(None);
    }
    let p = m
        .platforms
        .get(PLATFORM)
        .ok_or_else(|| format!("no setup for {PLATFORM}"))?;
    Ok(Some(Update {
        version: m.version.trim_start_matches('v').to_string(),
        url: p.url.clone(),
        signature: p.signature.clone(),
    }))
}

fn fetch(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = hidden("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--max-time",
            "600",
        ])
        .args(args)
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Asks GitHub for the latest release; `Ok(None)` when this one is current.
pub fn check(current: &str) -> Result<Option<Update>, String> {
    let body = fetch(&["--max-time", "20", ENDPOINT])?;
    parse_manifest(&String::from_utf8_lossy(&body), current)
}

fn b64_text(s: &str) -> Result<String, String> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s.trim())
        .map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn verify_with(
    key: &minisign_verify::PublicKey,
    data: &[u8],
    signature: &str,
) -> Result<(), String> {
    let sig =
        minisign_verify::Signature::decode(&b64_text(signature)?).map_err(|e| e.to_string())?;
    key.verify(data, &sig, true)
        .map_err(|e| format!("Signatur ungültig / bad signature: {e}"))
}

/// Whether `data` carries a valid signature of the release key (`signature`
/// as in the manifest).
pub fn verify(data: &[u8], signature: &str) -> Result<(), String> {
    let key = minisign_verify::PublicKey::decode(&b64_text(PUBKEY)?).map_err(|e| e.to_string())?;
    verify_with(&key, data, signature)
}

fn setup_path(version: &str) -> PathBuf {
    std::env::temp_dir()
        .join("faderust-update")
        .join(format!("Faderust_{version}_x64-setup.exe"))
}

/// Downloads the setup, checks its signature and starts it silently; it
/// replaces the files and starts Faderust again. The caller quits right after
/// (the setup stops a still running Faderust anyway).
pub fn install(update: &Update) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Updates gibt es nur unter Windows / Updates are Windows only".into());
    }
    let data = fetch(&[&update.url])?;
    verify(&data, &update.signature)?;
    let path = setup_path(&update.version);
    std::fs::create_dir_all(path.parent().unwrap_or(&path)).map_err(|e| e.to_string())?;
    std::fs::write(&path, &data).map_err(|e| e.to_string())?;
    hidden(&path)
        .args(["/S", "/UPDATE"])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{"version":"0.7.0","pub_date":"x","notes":"","platforms":{"windows-x86_64":{"signature":"c2ln","url":"https://example.invalid/setup.exe"}}}"#;

    #[test]
    fn a_newer_manifest_is_an_update() {
        let u = parse_manifest(MANIFEST, "0.6.4").unwrap().unwrap();
        assert_eq!(u.version, "0.7.0");
        assert_eq!(u.url, "https://example.invalid/setup.exe");
    }

    #[test]
    fn the_same_or_an_older_version_is_not() {
        assert!(parse_manifest(MANIFEST, "0.7.0").unwrap().is_none());
        assert!(parse_manifest(MANIFEST, "0.8.1").unwrap().is_none());
    }

    #[test]
    fn a_manifest_without_this_platform_is_an_error() {
        let json = MANIFEST.replace("windows-x86_64", "linux-x86_64");
        assert!(parse_manifest(&json, "0.6.4").is_err());
    }

    #[test]
    fn a_good_signature_verifies_and_a_changed_file_does_not() {
        use base64::Engine as _;
        // Test vector of the minisign-verify crate (pre-hashed, file "test").
        let key = minisign_verify::PublicKey::from_base64(
            "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3",
        )
        .unwrap();
        let sig = "untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1633700835\tfile:test\tprehashed\nwLMDjy9FLAuxZ3q4NlEvkgtyhrr0gtTu6KC4KBJdITbbOeAi1zBIYo0v4iTgt8jJpIidRJnp94ABQkJAgAooBQ==";
        let encoded = base64::engine::general_purpose::STANDARD.encode(sig);
        assert!(verify_with(&key, b"test", &encoded).is_ok());
        assert!(verify_with(&key, b"tampered", &encoded).is_err());
        // Signed by another key: the release key refuses it.
        assert!(verify(b"test", &encoded).is_err());
    }

    #[test]
    fn garbage_does_not_verify() {
        assert!(verify(b"setup", "bm90IGEgc2lnbmF0dXJl").is_err());
        assert!(verify(b"setup", "!!!").is_err());
    }
}
