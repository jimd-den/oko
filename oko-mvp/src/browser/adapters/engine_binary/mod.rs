//! Engine binary: locate, install and launch a prebuilt browser engine.
//!
//! Three operations on the host's own engine tree, `third_party/engines/<name>/`:
//! locate an installed binary, install one from the pinned release recorded in
//! `toolchain/versions.lock`, and spawn it — headless for a screenshot, or
//! windowed for a session. Nothing here runs at build time: install is an
//! explicit host action (DEP-002), and every operation is testable offline
//! against a throwaway workspace.
//!
//! The lock reader, the checksum check, the unpack, the download and the
//! process spawn live in the `lock`, `verify`, `extract`, `download` and
//! `launch` submodules. This file owns the engine-tree layout and the flow.
//!
//! Deliberately not here: satisfying `BrowserEnginePort`. That port models an
//! in-process embedder session; a prebuilt engine is a separate process driven
//! by its own command line, which is what `launch` implements.

mod download;
mod extract;
mod launch;
mod lock;
mod verify;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

pub use launch::{spawn_render, spawn_session, EngineProcess, RenderJob};
pub use lock::{parse_servo_entry, ArchiveKind, PinnedEngine};

use verify::is_unset;

/// The engine tree, `third_party/engines/`.
pub const ENGINE_DIR: &str = "engines";

/// The engine archive cache, `downloads/`.
pub const DOWNLOADS_DIR: &str = "downloads";

/// The binary file names upstream prebuilt archives place directly in the
/// engine directory. A lookup table of real artifacts: Servo ships `servoshell`,
/// Chromium ships `headless_shell`/`content_shell`/`chrome`.
pub const BINARIES: &[&str] = &["servoshell", "headless_shell", "content_shell", "chrome"];

/// `third_party/engines`.
pub fn engines_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join("third_party").join(ENGINE_DIR)
}

/// `third_party/engines/<name>`.
pub fn engine_dir(workspace_root: &Path, name: &str) -> PathBuf {
    engines_root(workspace_root).join(name)
}

/// The installed engine binary in `dir`, if one is there.
pub fn binary_in(dir: &Path) -> Option<PathBuf> {
    BINARIES
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// The installed `name` engine under `workspace_root`, if it is present.
pub fn locate(workspace_root: &Path, name: &str) -> Option<PathBuf> {
    binary_in(&engine_dir(workspace_root, name))
}

/// What the install flow did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallOutcome {
    /// The binary was already in the engine tree; nothing was fetched.
    AlreadyInstalled(PathBuf),
    /// The archive was fetched, verified and unpacked; here is the binary.
    Installed(PathBuf),
    /// The flow stopped, and this is why.
    Failed(String),
}

/// `engine install`: refuse when unset, else fetch → verify → unpack.
///
/// Contract:
/// - Never downloads when the binary is already present.
/// - Refuses when the pinned checksum is unset, rather than skipping the check.
/// - Fails loudly on a checksum mismatch, reporting both digests.
pub fn install(workspace_root: &Path, entry: &PinnedEngine) -> InstallOutcome {
    let dir = engine_dir(workspace_root, &entry.name);
    if let Some(binary) = binary_in(&dir) {
        println!(
            "engine: {} already installed at {}",
            entry.name,
            binary.display()
        );
        return InstallOutcome::AlreadyInstalled(binary);
    }
    if is_unset(&entry.sha256) {
        return InstallOutcome::Failed(format!(
            "{} source_checksum_sha256 is unset in versions.lock; \
             record the pinned release digest first",
            entry.name
        ));
    }

    let downloads = workspace_root.join(DOWNLOADS_DIR);
    let _ = fs::create_dir_all(&downloads);
    let archive = downloads.join(entry.archive.file_name(&entry.name));
    if !archive.exists() {
        if let Err(why) = download::download_archive(&entry.url, &archive) {
            return InstallOutcome::Failed(why);
        }
    }

    if let Err(why) = verify::verify(&archive, &entry.sha256) {
        return InstallOutcome::Failed(why);
    }
    if let Err(why) = extract::unpack(&archive, &dir, entry.archive) {
        return InstallOutcome::Failed(why);
    }

    match binary_in(&dir) {
        Some(binary) => InstallOutcome::Installed(binary),
        None => InstallOutcome::Failed(format!(
            "{} unpacked but no engine binary appeared in {}",
            entry.name,
            dir.display()
        )),
    }
}
