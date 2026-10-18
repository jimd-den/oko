//! Application: terminal session service (SRD §7.1).

use crate::domain::session::{Session, SessionId, SessionState};
use crate::ports::terminal::{PtySession, TerminalPort};

/// Supervises PTY-backed terminal sessions independently of the desktop UI.
/// Session persistence across UI reload is owned here, not by the UI (TERM-003).
pub struct SessionService {
    sessions: std::collections::HashMap<SessionId, Session>,
    pty_handles: std::collections::HashMap<SessionId, PtySession>,
}

impl Default for SessionService {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionService {
    pub fn new() -> Self {
        Self {
            sessions: std::collections::HashMap::new(),
            pty_handles: std::collections::HashMap::new(),
        }
    }

    /// Create a terminal session backed by a real PTY via the port adapter.
    pub fn create_terminal(
        &mut self,
        terminal: &dyn TerminalPort,
        id: SessionId,
    ) -> Result<Session, crate::ports::terminal::PtyError> {
        let pty = terminal.create_session()?;
        let session = Session::new(id.clone(), 0).running();
        self.sessions.insert(id.clone(), session.clone());
        self.pty_handles.insert(id, pty);
        Ok(session)
    }

    pub fn list(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }

    pub fn get(&self, id: &SessionId) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn stop(&mut self, id: &SessionId) -> Option<SessionState> {
        self.pty_handles.remove(id);
        self.sessions.remove(id).map(|s| s.state)
    }

    pub fn pty(&self, id: &SessionId) -> Option<&PtySession> {
        self.pty_handles.get(id)
    }
}
