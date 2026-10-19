//! Servo browser engine adapter (SRD §2.1, §5.2).
//!
//! Servo is the engine this project supports. It ships prebuilt release
//! archives rather than a linkable library, so the host embeds it the way a
//! browser embeds a renderer process: one real Servo process per browser
//! session, started on the URL that session is showing. `third_party/engines/
//! servo/servoshell` is the binary; `engine_binary` installs it.
//!
//! This is a process embedder, and the honest limits are written down rather
//! than hidden:
//!   - navigating a live session means restarting its process, because driving
//!     a running Servo needs its DevTools protocol (a later step);
//!   - `execute_script` reports failure for the same reason.
//! What it does not fake: the process is real, it renders the real page, and
//! its version is read from the binary rather than asserted.
//!
//! Exactly one engine backend serves one browser instance (SRD ENG-001), and
//! two adapters share no state (SRD §2.2).

mod port;

#[cfg(test)]
mod tests;

use crate::browser::ports::engine::{BrowserError, BrowserSession};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Session tracking. The Servo side owns the real page; this side owns the
/// process that is showing it. Not `Debug`: a live process has no useful
/// representation and printing one would tempt a log to try.
struct ServoSession {
    pub url: Option<String>,
    pub closed: bool,
    pub process: Option<crate::browser::adapters::engine_binary::EngineProcess>,
}

pub struct ServoBrowserEngineAdapter {
    /// The installed engine binary, if one was found. `None` means this build
    /// can name the engine but cannot run it.
    binary: Option<PathBuf>,
    /// Read from the binary itself, so the reported version is the one running.
    version: String,
    sessions: Mutex<HashMap<BrowserSession, ServoSession>>,
    next_id: Mutex<u64>,
    crash_reason: Mutex<Option<String>>,
}

impl Default for ServoBrowserEngineAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ServoBrowserEngineAdapter {
    /// An adapter with no engine behind it: it can report its identity and
    /// nothing else.
    pub fn new() -> Self {
        Self {
            binary: None,
            version: "unavailable".into(),
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            crash_reason: Mutex::new(None),
        }
    }

    /// An adapter bound to the installed engine under `workspace_root`.
    ///
    /// Contract: never downloads. A missing binary yields an unbound adapter
    /// that reports itself unavailable, so a machine without the engine still
    /// starts the host (SRD ENG-008).
    pub fn located(workspace_root: &Path) -> Self {
        let binary = crate::browser::adapters::engine_binary::locate(workspace_root, "servo");
        let version = binary
            .as_ref()
            .and_then(|path| read_version(path))
            .unwrap_or_else(|| "unavailable".into());
        Self {
            binary,
            version,
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            crash_reason: Mutex::new(None),
        }
    }

    /// An adapter bound to a chosen binary, for a test or a packaged engine.
    pub fn with_binary(binary: &Path) -> Self {
        let version = read_version(binary).unwrap_or_else(|| "unknown".into());
        Self {
            binary: Some(binary.to_path_buf()),
            version,
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            crash_reason: Mutex::new(None),
        }
    }

    /// Simulate an engine crash (for ENG-004 testing).
    pub fn simulate_crash(&self, reason: &str) {
        *self.crash_reason.lock().unwrap() = Some(reason.into());
    }

    pub fn clear_crash(&self) {
        *self.crash_reason.lock().unwrap() = None;
    }

    /// Whether a real engine is behind this adapter.
    pub fn is_bound(&self) -> bool {
        self.binary.is_some()
    }

    fn require_bound(&self) -> Result<&Path, BrowserError> {
        self.binary.as_deref().ok_or_else(|| {
            BrowserError::EngineUnavailable(
                "no servo engine installed; run `oko-host --install-engine servo`".into(),
            )
        })
    }
}

/// Ask the binary what it is. A version read from the program beats a version
/// written in this file, which would drift the first time the pin moves.
fn read_version(binary: &Path) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(text.trim().trim_start_matches("Version: ").to_string())
}
