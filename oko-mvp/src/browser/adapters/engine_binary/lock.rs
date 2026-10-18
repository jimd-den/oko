//! The `toolchain/versions.lock` reader for one pinned engine release.
//!
//! Owns the `[servo]` section schema — the release URL, the checksum and the
//! archive container — and nothing else. Changes with the lock format, not
//! with the install flow that consumes the record.

use std::fs;
use std::path::Path;

/// The lock section this package installs, and the directory under
/// `third_party/engines/` it unpacks into. One constant for both, so the
/// directory name cannot drift from the section name.
const SERVO: &str = "servo";

/// One pinned prebuilt engine release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedEngine {
    /// The lock section name; also the engine directory name.
    pub name: String,
    /// The release archive URL, as pinned.
    pub url: String,
    /// The archive's SHA-256 hex digest, as pinned.
    pub sha256: String,
    /// The archive container the URL serves.
    pub archive: ArchiveKind,
}

/// The upstream archive container, read from the lock's `archive_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    TarGz,
    TarXz,
}

impl ArchiveKind {
    /// The file name the archive is cached under, `downloads/<name>.tar.gz`.
    pub fn file_name(&self, engine: &str) -> String {
        match self {
            ArchiveKind::TarGz => format!("{}.tar.gz", engine),
            ArchiveKind::TarXz => format!("{}.tar.xz", engine),
        }
    }

    /// The `tar` mode flags: decompress, then unpack.
    pub fn tar_mode(&self) -> &'static str {
        match self {
            ArchiveKind::TarGz => "-xzf",
            ArchiveKind::TarXz => "-xJf",
        }
    }

    fn from_lock(value: &str) -> Option<Self> {
        match value {
            "tar.gz" => Some(ArchiveKind::TarGz),
            "tar.xz" => Some(ArchiveKind::TarXz),
            _ => None,
        }
    }
}

/// Parse the `[servo]` section of a versions.lock file.
///
/// Contract: the release is only usable when it names a URL, a checksum and a
/// container this package can unpack. An unset or missing checksum is an error,
/// never a skipped check (SRD DEP-001, DEP-003).
pub fn parse_servo_entry(path: &Path) -> Result<PinnedEngine, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read versions.lock: {}", e))?;
    let fields = section_fields(&text, &format!("[{}]", SERVO))
        .ok_or_else(|| format!("versions.lock has no [{}] section", SERVO))?;

    let url = field(&fields, "source_url")?;
    let sha256 = field(&fields, "source_checksum_sha256")?;
    if super::verify::is_unset(&sha256) {
        return Err(format!(
            "[{}] source_checksum_sha256 is unset; record the pinned release digest first",
            SERVO
        ));
    }
    let kind = field(&fields, "archive_kind")?;
    let archive = ArchiveKind::from_lock(&kind).ok_or_else(|| {
        format!(
            "[{}] archive_kind '{}' is not a container this host unpacks",
            SERVO, kind
        )
    })?;

    Ok(PinnedEngine {
        name: SERVO.into(),
        url,
        sha256,
        archive,
    })
}

/// The `key = value` pairs of one section, with quotes and whitespace removed.
fn section_fields(text: &str, header: &str) -> Option<Vec<(String, String)>> {
    let mut fields = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == header;
            continue;
        }
        if !inside || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            fields.push((
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            ));
        }
    }
    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

fn field(fields: &[(String, String)], key: &str) -> Result<String, String> {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("[{}] is missing {}", SERVO, key))
}
