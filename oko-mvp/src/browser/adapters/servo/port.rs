//! How the Servo adapter satisfies `BrowserEnginePort` (SRD §5.2).
//!
//! One real Servo process per session. Kept apart from the adapter's own state
//! so the whole engine-neutral contract for this backend is in one place.

use super::{ServoBrowserEngineAdapter, ServoSession};
use crate::browser::adapters::engine_binary;
use crate::browser::ports::engine::{
    BackendCapabilities, BrowserConfig, BrowserEnginePort, BrowserError, BrowserSession,
};
use crate::domain::session::SessionId;

impl BrowserEnginePort for ServoBrowserEngineAdapter {
    fn initialize(&self) -> Result<(), BrowserError> {
        if let Some(reason) = self.crash_reason.lock().unwrap().as_ref() {
            return Err(BrowserError::Crash(format!("engine crashed: {}", reason)));
        }
        self.require_bound().map(|_| ())
    }

    fn shutdown(&self) -> Result<(), BrowserError> {
        let mut sessions = self.sessions.lock().unwrap();
        for session in sessions.values_mut() {
            if let Some(process) = session.process.as_mut() {
                let _ = process.stop();
            }
            session.process = None;
        }
        Ok(())
    }

    fn engine_name(&self) -> &str {
        "servo"
    }

    fn engine_version(&self) -> &str {
        &self.version
    }

    fn is_experimental(&self) -> bool {
        true // SRD ENG-009: the experimental backend stays visibly marked
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_js: true,
            supports_downloads: false,
            supports_permissions: false,
            is_experimental: true,
        }
    }

    fn create_session(&self, config: BrowserConfig) -> Result<BrowserSession, BrowserError> {
        self.check_health()?;
        let binary = self.require_bound()?;

        let mut next = self.next_id.lock().unwrap();
        let id = BrowserSession(SessionId(format!("servo-{}", *next)));
        *next += 1;
        drop(next);

        let process = start(binary, config.url.as_deref())?;
        self.sessions.lock().unwrap().insert(
            id.clone(),
            ServoSession {
                url: config.url,
                closed: false,
                process: Some(process),
            },
        );
        Ok(id)
    }

    /// Point the session at `url`.
    ///
    /// The process is restarted, because steering a running Servo needs its
    /// DevTools protocol and this adapter does not speak it yet. The session
    /// keeps its identity across the restart, so a caller sees one session.
    fn navigate(&self, session: &BrowserSession, url: &str) -> Result<(), BrowserError> {
        self.check_health()?;
        let binary = self.require_bound()?;

        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions
            .get_mut(session)
            .ok_or(BrowserError::SessionNotFound)?;
        if entry.closed {
            return Err(BrowserError::SessionNotFound);
        }
        if let Some(process) = entry.process.as_mut() {
            let _ = process.stop();
        }
        entry.process = Some(start(binary, Some(url))?);
        entry.url = Some(url.into());
        Ok(())
    }

    fn close_session(&self, session: &BrowserSession) -> Result<(), BrowserError> {
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions
            .get_mut(session)
            .ok_or(BrowserError::SessionNotFound)?;
        if entry.closed {
            return Err(BrowserError::SessionNotFound);
        }
        if let Some(process) = entry.process.as_mut() {
            let _ = process.stop();
        }
        entry.process = None;
        entry.closed = true;
        Ok(())
    }

    fn execute_script(
        &self,
        session: &BrowserSession,
        _script: &str,
    ) -> Result<String, BrowserError> {
        self.check_health()?;
        let sessions = self.sessions.lock().unwrap();
        let entry = sessions.get(session).ok_or(BrowserError::SessionNotFound)?;
        if entry.closed {
            return Err(BrowserError::SessionNotFound);
        }
        Err(BrowserError::ScriptFailed(
            "running script in a live session needs Servo's DevTools protocol, which this \
             adapter does not speak yet"
                .into(),
        ))
    }

    fn last_crash_reason(&self) -> Option<String> {
        self.crash_reason.lock().unwrap().clone()
    }
}

impl ServoBrowserEngineAdapter {
    /// A crashed engine refuses work until it is cleared (SRD ENG-004).
    fn check_health(&self) -> Result<(), BrowserError> {
        match self.crash_reason.lock().unwrap().as_ref() {
            Some(reason) => Err(BrowserError::Crash(format!("engine crashed: {}", reason))),
            None => Ok(()),
        }
    }
}

/// Start one Servo process on `url`.
fn start(
    binary: &std::path::Path,
    url: Option<&str>,
) -> Result<engine_binary::EngineProcess, BrowserError> {
    let target = url.unwrap_or("about:blank");
    engine_binary::spawn_session(binary, target).map_err(BrowserError::SessionCreateFailed)
}
