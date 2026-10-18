//! The download shell-out: probe for curl/wget, then fetch the engine archive.
//!
//! The only network process I/O in the package; the flow that decides *what* to
//! download lives in the parent module. Changes with the download tool.

use std::path::Path;
use std::process::Command;

/// Select a downloader: prefer curl, then wget.
fn select_downloader() -> Option<Vec<String>> {
    if Command::new("curl")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some(vec!["curl".into(), "-L".into(), "--fail".into()]);
    }
    if Command::new("wget")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some(vec!["wget".into(), "-O".into()]);
    }
    None
}

/// Fetch `url` to `dest`.
///
/// Contract: never runs unless a caller asked for an install (SRD DEP-002).
/// A missing downloader is a clear failure, not a silent skip.
pub(super) fn download_archive(url: &str, dest: &Path) -> Result<(), String> {
    let mut argv = select_downloader().ok_or(
        "no downloader (curl/wget) found; install one, or place the engine archive          in the downloads directory yourself",
    )?;

    let status = match argv.first().map(String::as_str) {
        Some("wget") => Command::new(&argv[0])
            .args(&argv[1..])
            .arg(dest)
            .arg(url)
            .status(),
        _ => {
            argv.push(url.to_string());
            argv.push("-o".to_string());
            argv.push(dest.to_string_lossy().to_string());
            Command::new(&argv[0]).args(&argv[1..]).status()
        }
    }
    .map_err(|e| format!("run downloader: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("download failed: {}", url))
    }
}
