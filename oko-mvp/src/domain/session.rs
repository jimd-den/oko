//! Domain: session model (SRD §7.1, §8.1).

/// A session identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

/// Lifecycle states for applications and sessions (SRD §8.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Discovered,
    Validated,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Unavailable,
}

/// A terminal or application session.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub state: SessionState,
    pub created_at: u64,
}

impl Session {
    pub fn new(id: SessionId, created_at: u64) -> Self {
        Self {
            id,
            state: SessionState::Starting,
            created_at,
        }
    }

    pub fn running(mut self) -> Self {
        self.state = SessionState::Running;
        self
    }

    pub fn stopped(mut self) -> Self {
        self.state = SessionState::Stopped;
        self
    }

    pub fn failed(mut self) -> Self {
        self.state = SessionState::Failed;
        self
    }
}
