//! Port: process management (SRD §7.2).
//!
//! Structured executable and argument fields; process identity, state, exit
//! reason, resource ownership; graceful shutdown and bounded escalation.

/// A request to spawn a process.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub executable: String,
    pub args: Vec<String>,
}

/// An opaque handle to a spawned process (engine-neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessHandle(pub u64);

/// The result of waiting on a process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitStatus {
    pub code: Option<i32>,
    pub signal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessError {
    SpawnFailed(String),
    NotFound,
    AlreadyStopped,
    Io(String),
}

pub trait ProcessPort: Send + Sync {
    fn spawn(&self, req: SpawnRequest) -> Result<ProcessHandle, ProcessError>;
    fn kill(&self, handle: ProcessHandle) -> Result<(), ProcessError>;
    fn wait(&self, handle: ProcessHandle) -> Result<ExitStatus, ProcessError>;
    /// Capture stdout of a finished process, if requested.
    fn stdout(&self, handle: ProcessHandle) -> Result<String, ProcessError>;
}
