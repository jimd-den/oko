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
    /// The screen the session is showing.
    ///
    /// Contract: returns the rendered screen, not the bytes behind it —
    /// cursor movement applied, erases applied, control sequences gone, lines
    /// trailing-blank-trimmed. It is the same screen on every read until the
    /// session draws something, so a caller that wants a log appends a diff,
    /// not the string.
    ///
    /// A real shell draws the echo of what it was given before it draws the
    /// answer, so an implementation waits for the screen to settle rather than
    /// returning at the first byte. It must never wait without bound.
    fn read_output(&self, session: &PtySession) -> Result<String, PtyError>;

    /// The row the session's cursor is on, counted from the top of the screen.
    ///
    /// Contract: a shell draws its prompt and then waits there, so this is the
    /// line a command's output stops before — which is how a caller that asked
    /// a question tells the answer from the question.
    fn cursor_line(&self, session: &PtySession) -> Result<usize, PtyError>;
    fn resize(&self, _session: &PtySession, _cols: u16, _rows: u16) -> Result<(), PtyError> {
        Ok(())
    }
    /// Report the process exit code/signal when the session ends (TERM-008).
    fn exit_status(&self, session: &PtySession) -> Result<Option<i32>, PtyError>;
}
