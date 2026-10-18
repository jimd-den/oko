//! The download shell-out: probe for curl/wget, then fetch the archive.
//!
//! The only process I/O in the package; the flow logic that decides *what* to
//! download lives in the parent module. Changes with the download tool.

use std::path::Path;
use std::process::Command;

/// Select a downloader: prefer curl, then wget.
fn select_downloader() -> Option<Vec<String>> {
    if Command::new("curl").arg("--version").status().map(|s| s.success()).unwrap_or(false) {
        return Some(vec!["curl".into(), "-L".into(), "--fail".into()]);
    }
    if Command::new("wget").arg("--version").status().map(|s| s.success()).unwrap_or(false) {
        return Some(vec!["wget".into(), "-O".into(), "__tmp__".into()]);
    }
    None
}

// DEP-002: explicit download only. Shell out to curl/wget.
pub(super) fn download_archive(url: &str, dest: &Path) -> Result<(), String> {
    let cmd = match select_downloader() {
        Some(cmd) => cmd,
        None => {
            return Err(
                "no downloader (curl/wget) found; install curl or wget, or place the Chromium source archive at downloads/chromium-src.tar.xz manually".into(),
            );
        }
    };
    println!("blink: downloading {} ...", url);
    let status = Command::new(&cmd[0])
        .args(&cmd[1..])
        .arg(url)
        .arg("-o")
        .arg(dest)
        .status();
    let status = match status {
        Ok(s) => s,
        Err(e) => return Err(format!("download command failed: {}", e)),
    };
    if !status.success() {
        return Err("download failed".into());
    }
    Ok(())
}
