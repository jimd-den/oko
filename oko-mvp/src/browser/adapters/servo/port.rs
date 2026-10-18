//! How the Servo adapter satisfies `BrowserEnginePort` (SRD §5.2).
//!
//! Kept apart from the adapter's own state and construction so the whole
//! engine-neutral contract for this backend is in one place, and so a change to
//! the port is visible in one file per engine.

use super::{ServoBrowserEngineAdapter, ServoSession};
use crate::browser::ports::engine::{
    BackendCapabilities, BrowserConfig, BrowserEnginePort, BrowserError, BrowserSession,
};
use crate::domain::session::SessionId;

impl BrowserEnginePort for ServoBrowserEngineAdapter {
    fn initialize(&self) -> Result<(), BrowserError> {
        if let Some(reason) = self.crash_reason.lock().unwrap().as_ref() {
            return Err(BrowserError::Crash(format!("engine crashed: {}", reason)));
        }
        if self.embedder.is_linked() {
            // In a real build, call oko_servo_embedder_init here.
            Ok(())
        } else {
            Err(BrowserError::EngineUnavailable(
                "servo embedder not linked (feature `servo` requires a \
                 separately built Servo checkout)"
                    .into(),
            ))
        }
    }

    fn shutdown(&self) -> Result<(), BrowserError> {
        if self.embedder.is_linked() {
            // In a real build, call oko_servo_embedder_shutdown here.
        }
        Ok(())
    }

    fn engine_name(&self) -> &str {
        "servo"
    }

    fn engine_version(&self) -> &str {
        // Pinned at build time from the Servo source revision (SRD §14).
        "servo-pinned"
    }

    fn is_experimental(&self) -> bool {
        true // Servo is the experimental alternative engine (SRD ENG-009)
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
        if let Some(reason) = self.crash_reason.lock().unwrap().as_ref() {
            return Err(BrowserError::Crash(format!("engine crashed: {}", reason)));
        }
        self.require_linked()?;

        let mut sessions = self.sessions.lock().unwrap();
        let mut next = self.next_id.lock().unwrap();
        let id = BrowserSession(SessionId(format!("servo-{}", *next)));
        *next += 1;
        sessions.insert(
            id.clone(),
            ServoSession {
                url: config.url.clone(),
                closed: false,
            },
        );
        Ok(id)
    }

    fn navigate(&self, session: &BrowserSession, url: &str) -> Result<(), BrowserError> {
        if let Some(reason) = self.crash_reason.lock().unwrap().as_ref() {
            return Err(BrowserError::Crash(format!("engine crashed: {}", reason)));
        }
        self.require_linked()?;
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions
            .get_mut(session)
            .ok_or(BrowserError::SessionNotFound)?;
        if entry.closed {
            return Err(BrowserError::SessionNotFound);
        }
        entry.url = Some(url.into());
        Ok(())
    }

    fn close_session(&self, session: &BrowserSession) -> Result<(), BrowserError> {
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions
            .get_mut(session)
            .ok_or(BrowserError::SessionNotFound)?;
        entry.closed = true;
        Ok(())
    }

    fn execute_script(
        &self,
        _session: &BrowserSession,
        _script: &str,
    ) -> Result<String, BrowserError> {
        if let Some(reason) = self.crash_reason.lock().unwrap().as_ref() {
            return Err(BrowserError::Crash(format!("engine crashed: {}", reason)));
        }
        self.require_linked()?;
        Err(BrowserError::ScriptFailed(
            "script execution requires a linked Servo embedder".into(),
        ))
    }

    fn last_crash_reason(&self) -> Option<String> {
        self.crash_reason.lock().unwrap().clone()
    }
}
