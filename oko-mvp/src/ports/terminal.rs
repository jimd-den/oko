//! Port: PTY-backed terminal sessions (SRD §7.1).
//!
//! Render terminal output and process input independently of the desktop UI.
//! This adapter is swapped per-platform; the domain never touches a raw fd.

use crate::domain::session::SessionId;

/// An opaque PTY session handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PtySession(pub SessionId);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyError {
    OpenFailed(String),
    ReadFailed(String),
    WriteFailed(String),
    SessionGone,
}

pub trait TerminalPort: Send + Sync {
    /// Create a real OS-backed PTY session (TERM-001).
    fn create_session(&self) -> Result<PtySession, PtyError>;
    /// Write user input into the PTY.
    fn write_input(&self, session: &PtySession, bytes: &[u8]) -> Result<(), PtyError>;
    /// Read terminal output.
    ///
    /// Contract: returns what the session has produced since the last read. A
    /// real shell answers when it answers, so an implementation may wait
    /// briefly for output rather than returning an empty screen to a caller
    /// that has just written to it. It must never wait without bound.
    fn read_output(&self, session: &PtySession) -> Result<String, PtyError>;
    fn resize(&self, _session: &PtySession, _cols: u16, _rows: u16) -> Result<(), PtyError> {
        Ok(())
    }
    /// Report the process exit code/signal when the session ends (TERM-008).
    fn exit_status(&self, session: &PtySession) -> Result<Option<i32>, PtyError>;
}
