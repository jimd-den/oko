//! Unpack a downloaded engine archive with the system `tar`.
//!
//! The only other process I/O in the package besides the download. Changes with
//! the archive tool, not with the install flow that calls it.
//!
//! Hazard: the upstream prebuilt archives wrap everything in one top-level
//! directory (`servo/servoshell`), so unpacking strips that level and the
//! binary lands directly in the engine directory, which is where `binary_in`
//! looks for it. An archive with a different shape unpacks without the binary
//! appearing, and `install` reports that rather than pretending it worked.

use super::lock::ArchiveKind;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Unpack `archive` into `engine_dir`.
pub(super) fn unpack(archive: &Path, engine_dir: &Path, kind: ArchiveKind) -> Result<(), String> {
    fs::create_dir_all(engine_dir)
        .map_err(|e| format!("create {}: {}", engine_dir.display(), e))?;
    let status = Command::new("tar")
        .arg(kind.tar_mode())
        .arg(archive)
        .arg("-C")
        .arg(engine_dir)
        .arg("--strip-components=1")
        .status()
        .map_err(|e| format!("run tar: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("tar failed unpacking {}", archive.display()))
    }
}
