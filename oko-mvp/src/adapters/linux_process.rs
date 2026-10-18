//! Adapter: Linux process spawn via std::process::Command.
//!
//! Implements `ProcessPort` for the MVP. `spawn` runs a command to completion
//! and caches its output; `wait`/`stdout` read the cache. A real PTY/nix
//! integration can replace this behind the same trait without touching
//! domain/application.

use crate::ports::process::{ExitStatus, ProcessError, ProcessHandle, ProcessPort, SpawnRequest};
use std::collections::HashMap;
use std::os::unix::process::ExitStatusExt;
use std::process::Stdio;
use std::sync::Mutex;

/// Cached result of a spawned command.
#[derive(Debug, Clone)]
struct CachedResult {
    stdout: String,
    exit: ExitStatus,
}

/// Linux process adapter. Owns cached command results.
pub struct LinuxProcessAdapter {
    inner: Mutex<Inner>,
}

struct Inner {
    results: HashMap<ProcessHandle, CachedResult>,
    next_id: u64,
}

impl Default for LinuxProcessAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxProcessAdapter {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                results: HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl ProcessPort for LinuxProcessAdapter {
    fn spawn(&self, req: SpawnRequest) -> Result<ProcessHandle, ProcessError> {
        let output = std::process::Command::new(&req.executable)
            .args(&req.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| ProcessError::SpawnFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let exit = ExitStatus {
            code: output.status.code(),
            signal: if output.status.signal().is_some() {
                Some("signaled".into())
            } else {
                None
            },
        };
        let handle = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| ProcessError::SpawnFailed("adapter lock poisoned".into()))?;
            let id = ProcessHandle(inner.next_id);
            inner.next_id += 1;
            inner.results.insert(
                id,
                CachedResult {
                    stdout,
                    exit: exit.clone(),
                },
            );
            id
        };
        Ok(handle)
    }

    fn kill(&self, _handle: ProcessHandle) -> Result<(), ProcessError> {
        // Commands run to completion in this MVP; kill is a no-op success.
        Ok(())
    }

    fn wait(&self, handle: ProcessHandle) -> Result<ExitStatus, ProcessError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ProcessError::SpawnFailed("adapter lock poisoned".into()))?;
        inner
            .results
            .get(&handle)
            .map(|c| c.exit.clone())
            .ok_or(ProcessError::NotFound)
    }

    fn stdout(&self, handle: ProcessHandle) -> Result<String, ProcessError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| ProcessError::SpawnFailed("adapter lock poisoned".into()))?;
        inner
            .results
            .get(&handle)
            .map(|c| c.stdout.clone())
            .ok_or(ProcessError::NotFound)
    }
}
