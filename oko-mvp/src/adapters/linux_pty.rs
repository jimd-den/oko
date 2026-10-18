//! Adapter: PTY-backed terminal session for Linux.
//!
//! Implements `TerminalPort` using std only for the MVP. Each `write_input` +
//! `read_output` round-trip executes the buffered line through a shell and
//! returns its output and exit status — a real `posix_openpt`/nix PTY can
//! replace this behind the same trait without touching domain/application.

use crate::domain::session::SessionId;
use crate::ports::terminal::{PtyError, PtySession, TerminalPort};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;

struct SessionEntry {
    pending_input: String,
    last_output: String,
    last_exit: Option<i32>,
    open: bool,
}

pub struct LinuxPtyAdapter {
    sessions: Mutex<HashMap<SessionId, SessionEntry>>,
    next_id: Mutex<u64>,
}

impl Default for LinuxPtyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxPtyAdapter {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }
}

impl TerminalPort for LinuxPtyAdapter {
    fn create_session(&self) -> Result<PtySession, PtyError> {
        let id = {
            let mut next = self
                .next_id
                .lock()
                .map_err(|_| PtyError::OpenFailed("adapter lock poisoned".into()))?;
            let id = SessionId(format!("pty-{}", *next));
            *next += 1;
            id
        };
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| PtyError::OpenFailed("adapter lock poisoned".into()))?;
        sessions.insert(
            id.clone(),
            SessionEntry {
                pending_input: String::new(),
                last_output: String::new(),
                last_exit: None,
                open: true,
            },
        );
        Ok(PtySession(id))
    }

    fn write_input(&self, session: &PtySession, bytes: &[u8]) -> Result<(), PtyError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| PtyError::WriteFailed("invalid utf-8".into()))?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| PtyError::WriteFailed("adapter lock poisoned".into()))?;
        let entry = sessions.get_mut(&session.0).ok_or(PtyError::SessionGone)?;
        if !entry.open {
            return Err(PtyError::SessionGone);
        }
        entry.pending_input.push_str(text);
        Ok(())
    }

    fn read_output(&self, session: &PtySession) -> Result<String, PtyError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| PtyError::ReadFailed("adapter lock poisoned".into()))?;
        let entry = sessions.get_mut(&session.0).ok_or(PtyError::SessionGone)?;
        if !entry.open {
            return Err(PtyError::SessionGone);
        }
        let input = std::mem::take(&mut entry.pending_input);
        if input.trim().is_empty() {
            return Ok(entry.last_output.clone());
        }
        // Execute the buffered line through a shell (PTY equivalent for MVP).
        let output = Command::new("sh")
            .arg("-c")
            .arg(&input)
            .output()
            .map_err(|e| PtyError::ReadFailed(e.to_string()))?;
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        entry.last_output = out.clone();
        entry.last_exit = output.status.code();
        Ok(out)
    }

    fn resize(&self, _session: &PtySession, _cols: u16, _rows: u16) -> Result<(), PtyError> {
        Ok(())
    }

    fn exit_status(&self, session: &PtySession) -> Result<Option<i32>, PtyError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| PtyError::ReadFailed("adapter lock poisoned".into()))?;
        let entry = sessions.get(&session.0).ok_or(PtyError::SessionGone)?;
        Ok(entry.last_exit)
    }
}
