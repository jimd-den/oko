//! How the Blink adapter satisfies `BrowserEnginePort` (SRD §5.2).
//!
//! Kept apart from the adapter's own state and construction so the whole
//! engine-neutral contract for this backend is in one place, and so a change to
//! the port is visible in one file per engine.

use super::{BlinkBrowserEngineAdapter, BlinkSession};
use crate::browser::ports::engine::{
    BackendCapabilities, BrowserConfig, BrowserEnginePort, BrowserError, BrowserSession,
};
use crate::domain::session::SessionId;

impl BrowserEnginePort for BlinkBrowserEngineAdapter {
    fn initialize(&self) -> Result<(), BrowserError> {
        if let Some(reason) = self.crash_reason.lock().unwrap().as_ref() {
            return Err(BrowserError::Crash(format!("engine crashed: {}", reason)));
        }
        if self.embedder.is_linked() {
            // In a real build, call oko_blink_embedder_init here.
            Ok(())
        } else {
            Err(BrowserError::EngineUnavailable(
                "blink embedder not linked (feature `blink` requires a \
                 separately built Chromium)"
                    .into(),
            ))
        }
    }

    fn shutdown(&self) -> Result<(), BrowserError> {
        if self.embedder.is_linked() {
            // In a real build, call oko_blink_embedder_shutdown here.
        }
        Ok(())
    }

    fn engine_name(&self) -> &str {
        "blink"
    }

    fn engine_version(&self) -> &str {
        // Pinned at build time from the Chromium DEPS/version file (SRD §14).
        "chromium-pinned"
    }

    fn is_experimental(&self) -> bool {
        false // Blink is the production candidate (SRD §5.2 table)
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_js: true,
            supports_downloads: true,
            supports_permissions: true,
            is_experimental: false,
        }
    }

    fn create_session(&self, config: BrowserConfig) -> Result<BrowserSession, BrowserError> {
        if let Some(reason) = self.crash_reason.lock().unwrap().as_ref() {
            return Err(BrowserError::Crash(format!("engine crashed: {}", reason)));
        }
        self.require_linked()?;

        let mut sessions = self.sessions.lock().unwrap();
        let mut next = self.next_id.lock().unwrap();
        let id = BrowserSession(SessionId(format!("blink-{}", *next)));
        *next += 1;
        sessions.insert(
            id.clone(),
            BlinkSession {
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
            "script execution requires a linked V8 embedder".into(),
        ))
    }

    fn last_crash_reason(&self) -> Option<String> {
        self.crash_reason.lock().unwrap().clone()
    }
}
