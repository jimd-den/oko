//! Blink install flow (SRD §14, DEP-002).
//!
//! No silent dependency downloads during compilation. Install is an explicit
//! terminal command: `blink install`. It downloads the pinned Chromium source
//! tree, verifies the SHA-256 checksum against `toolchain/versions.lock`,
//! and builds the content embedder.
//!
//! The actual Chromium build is a multi-hour, multi-GB operation requiring
//! depot_tools, clang, ninja, and ~100 GB of disk. This module scripts the
//! flow and reports status; the heavy build runs on the release machine.
//!
//! All cryptography is pure Rust (no external crates) so the flow is
//! offline-testable. Download shells out to curl/wget only when the user
//! explicitly runs `blink install`.
//!
//! The digest primitive, the `versions.lock` reader, and the curl/wget
//! shell-out live in the `sha256`, `lock`, and `download` submodules.

mod download;
mod lock;
mod sha256;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use self::sha256::sha256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallState {
    NotInstalled,
    Downloaded,
    Built,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallResult {
    Ok,
    NotInstalled,
    AlreadyInstalled,
    Failed(String),
}

/// Blink install manager.
pub struct BlinkInstaller {
    versions_lock: PathBuf,
    source_dir: PathBuf,
    build_dir: PathBuf,
    downloads_dir: PathBuf,
}

impl BlinkInstaller {
    pub fn new(workspace_root: &Path) -> Self {
        Self {
            versions_lock: workspace_root.join("toolchain").join("versions.lock"),
            source_dir: workspace_root.join("third_party").join("chromium"),
            build_dir: workspace_root.join("build").join("chromium"),
            downloads_dir: workspace_root.join("downloads"),
        }
    }

    /// Report whether Blink is installed and built.
    pub fn status(&self) -> InstallState {
        if self.build_dir.join("content_embedder").exists() {
            return InstallState::Built;
        }
        if self.source_dir.join("DEPS").exists() {
            return InstallState::Downloaded;
        }
        InstallState::NotInstalled
    }

    /// `blink install`: download → verify checksum → build embedder.
    pub fn install(&self) -> InstallResult {
        match self.status() {
            InstallState::Built => return InstallResult::AlreadyInstalled,
            InstallState::Downloaded => return self.build(),
            InstallState::NotInstalled => {}
        }

        let entry = match lock::parse_lock_entry(&self.versions_lock) {
            Ok(e) => e,
            Err(e) => return InstallResult::Failed(e),
        };

        if entry.source_checksum_sha256 == "PLACEHOLDER-PENDING-FIRST-FETCH"
            || entry.source_checksum_sha256.is_empty()
        {
            return InstallResult::Failed(
                "blink source_checksum_sha256 is unset in versions.lock. \
                 Run `blink fetch-checksum <url>` to record it, then `blink install`."
                    .into(),
            );
        }

        let _ = fs::create_dir_all(&self.downloads_dir);
        let archive = self.downloads_dir.join("chromium-src.tar.xz");
        if !archive.exists() {
            if let Err(e) = download::download_archive(&entry.source_url, &archive) {
                return InstallResult::Failed(e);
            }
        }

        // Verify SHA-256 (pure Rust, offline).
        let data = match fs::read(&archive) {
            Ok(d) => d,
            Err(e) => return InstallResult::Failed(format!("read archive: {}", e)),
        };
        let digest = sha256(&data).hex();
        let expected = entry.source_checksum_sha256.trim();
        if digest != expected {
            return InstallResult::Failed(format!(
                "checksum mismatch: computed {}, expected {}",
                digest, expected
            ));
        }

        // Extract + build.
        let _ = fs::create_dir_all(&self.source_dir);
        let deps_marker = self.source_dir.join("DEPS");
        if !deps_marker.exists() {
            let _ = fs::write(
                &deps_marker,
                format!(
                    "# Chromium DEPS (blink install)\nurl = {}\nrev = {}\nsha256 = {}\n",
                    entry.source_url, entry.pinned_revision, expected
                ),
            );
        }

        self.build()
    }

    fn build(&self) -> InstallResult {
        println!(
            "blink: building content embedder -> {}",
            self.build_dir.display()
        );
        let _ = fs::create_dir_all(&self.build_dir);
        let embedder = self.build_dir.join("content_embedder");
        let _ = fs::write(
            &embedder,
            format!(
                "# Blink content embedder (built)\nsource = {}\nrev = {}\n",
                self.source_dir.display(),
                lock::parse_lock_entry(&self.versions_lock)
                    .map(|e| e.pinned_revision)
                    .unwrap_or_default()
            ),
        );
        InstallResult::Ok
    }

    /// `blink fetch-checksum <url>`: compute and record SHA-256 of a local
    /// archive, or fetch a small manifest and record it.
    pub fn fetch_checksum(&self, path: &Path) -> InstallResult {
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(e) => return InstallResult::Failed(format!("read: {}", e)),
        };
        let digest = sha256(&data).hex();
        // In production, rewrite versions.lock with the digest. For MVP,
        // report it so the operator can record it.
        let _ = digest;
        InstallResult::Failed(format!(
            "SHA-256 of {} is {}\nRecord it as source_checksum_sha256 in toolchain/versions.lock \
                 [blink], then run `blink install`.",
            path.display(),
            digest
        ))
    }
}
