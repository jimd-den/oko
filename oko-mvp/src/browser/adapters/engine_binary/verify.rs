//! Checksum verification for a downloaded engine archive.
//!
//! Reuses the crate's pure-Rust SHA-256 (`engine_binary::sha256`) so the check
//! stays offline and testable. A second digest would put two implementations
//! of one algorithm in the tree to keep in sync.

use super::sha256::sha256;
use std::fs;
use std::path::Path;

/// The placeholder a pinned record carries until a real digest has been
/// recorded.
const UNSET_SENTINEL: &str = "PLACEHOLDER-PENDING-FIRST-FETCH";

/// A pinned checksum that has not been filled in yet.
pub(super) fn is_unset(sha256: &str) -> bool {
    let trimmed = sha256.trim();
    trimmed.is_empty() || trimmed == UNSET_SENTINEL
}

/// Verify `path` against the hex `expected` digest.
///
/// Contract: compares case-insensitively after trimming; a mismatch reports
/// both digests and is never silently accepted.
pub(super) fn verify(path: &Path, expected: &str) -> Result<(), String> {
    let data = fs::read(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let actual = sha256(&data).hex();
    if actual.eq_ignore_ascii_case(expected.trim()) {
        return Ok(());
    }
    Err(format!(
        "checksum mismatch for {}: computed {}, expected {}",
        path.display(),
        actual,
        expected.trim()
    ))
}
