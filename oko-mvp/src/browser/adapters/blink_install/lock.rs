//! The `toolchain/versions.lock` file format: the `[blink]` section record and
//! its parser. Changes with the lock schema, not with the install flow that
//! reads the pinned checksum from it.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// The `[blink]` fields the install flow consumes. The rest of the section is
/// the inventory the release tooling reads straight from the file (SRD §14.1),
/// so it is not duplicated here.
#[derive(Debug, Clone)]
pub struct LockEntry {
    pub source_url: String,
    pub pinned_revision: String,
    pub source_checksum_sha256: String,
}

/// Parse the [blink] section of versions.lock.
pub(super) fn parse_lock_entry(path: &Path) -> Result<LockEntry, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read versions.lock: {}", e))?;
    let mut in_blink = false;
    let mut fields: HashMap<String, String> = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "[blink]" {
            in_blink = true;
            continue;
        }
        if in_blink {
            if trimmed.starts_with('[') && trimmed != "[blink]" {
                break;
            }
            if let Some(eq) = trimmed.find('=') {
                let key = trimmed[..eq].trim().to_string();
                let val = trimmed[eq + 1..].trim().trim_matches('"').to_string();
                fields.insert(key, val);
            }
        }
    }
    Ok(LockEntry {
        source_url: fields.get("source_url").cloned().unwrap_or_default(),
        pinned_revision: fields.get("pinned_revision").cloned().unwrap_or_default(),
        source_checksum_sha256: fields
            .get("source_checksum_sha256")
            .cloned()
            .unwrap_or_default(),
    })
}
