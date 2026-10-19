//! Adapter: real PTY-backed terminal sessions (SRD §7.1, TERM-001..TERM-008).
//!
//! Replaces the slice-1 stub, which buffered a line and ran it through `sh -c`:
//! no interactivity, no signals, no real exit status, no window size, and a
//! `read_output` that returned a command's stdout rather than a terminal's.
//! This adapter opens a kernel PTY, spawns a shell on the slave side, and drives
//! the master side directly, so a session behaves like a session.
//!
//! Not here yet: the VT emulator. `read_output` returns what the shell wrote,
//! including its echo and control sequences, rather than a rendered screen, so
//! a caller sees a terminal rather than a screen.
//!
//! The next step is the cell grid, behind this same port — which is the point of
//! the port, and why the trait does not change for it. The API was read out of
//! `alacritty_terminal` 0.26's own source rather than guessed, so it does not
//! need re-deriving:
//!
//! ```text
//! let size = ScreenSize::new(COLS, ROWS);   // impl Dimensions: total_lines,
//!                                           // screen_lines, columns
//! let mut term = Term::new(Config::default(), &size, VoidListener);
//! let mut parser = alacritty_terminal::vte::Parser::new();
//! parser.advance(&mut term, bytes);         // the canonical feed
//! for cell in term.renderable_content().display_iter { /* cell.point, cell.c */ }
//! ```
//!
//! `read_output` then renders the grid instead of returning the raw buffer.
//! Until it does, a shell's echo and prompt are part of every reply.

mod grid;
mod session;

#[cfg(test)]
mod tests;

use crate::domain::session::SessionId;
use crate::ports::terminal::{PtyError, PtySession, TerminalPort};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use session::SessionEntry;
use std::collections::HashMap;
use std::sync::Mutex;

/// The window every session opens with, until something resizes it.
const ROWS: u16 = 24;
const COLS: u16 = 80;

pub struct PortablePtyAdapter {
    sessions: Mutex<HashMap<SessionId, SessionEntry>>,
    next_id: Mutex<u64>,
}

impl Default for PortablePtyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PortablePtyAdapter {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }

    /// The shell a session runs. `$SHELL` when the environment names one, else
    /// `/bin/sh`, because a console that cannot find a shell is not a console.
    fn shell() -> String {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    }

    fn with_session<T>(
        &self,
        session: &PtySession,
        action: impl FnOnce(&mut SessionEntry) -> Result<T, PtyError>,
    ) -> Result<T, PtyError> {
        let mut sessions = self.sessions.lock().map_err(|_| PtyError::SessionGone)?;
        let entry = sessions.get_mut(&session.0).ok_or(PtyError::SessionGone)?;
        action(entry)
    }
}

impl TerminalPort for PortablePtyAdapter {
    fn create_session(&self) -> Result<PtySession, PtyError> {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows: ROWS,
            cols: COLS,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = pty_system
            .openpty(size)
            .map_err(|e| PtyError::OpenFailed(format!("openpty: {}", e)))?;

        let child = pair
            .slave
            .spawn_command(CommandBuilder::new(Self::shell()))
            .map_err(|e| PtyError::OpenFailed(format!("spawn shell: {}", e)))?;
        // The slave must be dropped here, or the master never sees EOF.
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::OpenFailed(format!("clone reader: {}", e)))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::OpenFailed(format!("take writer: {}", e)))?;

        let id = {
            let mut next = self
                .next_id
                .lock()
                .map_err(|_| PtyError::OpenFailed("adapter lock poisoned".into()))?;
            let id = SessionId(format!("pty-{}", *next));
            *next += 1;
            id
        };

        self.sessions
            .lock()
            .map_err(|_| PtyError::OpenFailed("adapter lock poisoned".into()))?
            .insert(
                id.clone(),
                SessionEntry::new(pair.master, writer, reader, child, COLS, ROWS),
            );
        Ok(PtySession(id))
    }

    fn write_input(&self, session: &PtySession, bytes: &[u8]) -> Result<(), PtyError> {
        self.with_session(session, |entry| entry.write(bytes))
    }

    fn read_output(&self, session: &PtySession) -> Result<String, PtyError> {
        self.with_session(session, |entry| entry.read_screen())
    }

    fn cursor_line(&self, session: &PtySession) -> Result<usize, PtyError> {
        self.with_session(session, |entry| Ok(entry.cursor_line()))
    }

    fn resize(&self, session: &PtySession, cols: u16, rows: u16) -> Result<(), PtyError> {
        self.with_session(session, |entry| entry.resize(cols, rows))
    }

    fn exit_status(&self, session: &PtySession) -> Result<Option<i32>, PtyError> {
        self.with_session(session, |entry| entry.exit_status())
    }
}
