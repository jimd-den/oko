//! MockBrowserEngineAdapter — dev/stub browser engine (SRD §5.2).
//!
//! Simulates a browser engine behind `BrowserEnginePort`. Engine-neutral: no
//! engine objects leak through. This adapter is a dev stub, not a production
//! fallback; Servo is the engine the host actually serves.

use crate::browser::ports::engine::{
    BackendCapabilities, BrowserConfig, BrowserEnginePort, BrowserError, BrowserSession,
};
use crate::domain::session::SessionId;
use std::collections::HashMap;
use std::sync::Mutex;

struct SessionEntry {
    url: Option<String>,
    closed: bool,
}

pub struct MockBrowserEngineAdapter {
    sessions: Mutex<HashMap<BrowserSession, SessionEntry>>,
    next_id: Mutex<u64>,
    crash_reason: Mutex<Option<String>>,
}

impl Default for MockBrowserEngineAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBrowserEngineAdapter {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            crash_reason: Mutex::new(None),
        }
    }

    /// Simulate an engine crash (for ENG-004 testing: engine crash must not
    /// take down the terminal core).
    pub fn simulate_crash(&self, reason: &str) {
        *self.crash_reason.lock().unwrap() = Some(reason.into());
    }

    pub fn clear_crash(&self) {
        *self.crash_reason.lock().unwrap() = None;
    }
}

impl BrowserEnginePort for MockBrowserEngineAdapter {
    fn initialize(&self) -> Result<(), BrowserError> {
        if self.crash_reason.lock().unwrap().is_some() {
            return Err(BrowserError::Crash("engine failed to initialize".into()));
        }
        Ok(())
    }

    fn shutdown(&self) -> Result<(), BrowserError> {
        Ok(())
    }

    fn engine_name(&self) -> &str {
        "mock"
    }

    fn engine_version(&self) -> &str {
        "0.1.0-dev"
    }

    fn is_experimental(&self) -> bool {
        true // dev stub is visibly experimental (SRD ENG-009)
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
        if self.crash_reason.lock().unwrap().is_some() {
            return Err(BrowserError::Crash("engine crashed".into()));
        }
        let mut sessions = self.sessions.lock().unwrap();
        let mut next = self.next_id.lock().unwrap();
        let id = BrowserSession(SessionId(format!("browser-{}", *next)));
        *next += 1;
        sessions.insert(
            id.clone(),
            SessionEntry {
                url: config.url.clone(),
                closed: false,
            },
        );
        Ok(id)
    }

    fn navigate(&self, session: &BrowserSession, url: &str) -> Result<(), BrowserError> {
        if self.crash_reason.lock().unwrap().is_some() {
            return Err(BrowserError::Crash(
                "engine crashed during navigation".into(),
            ));
        }
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
        script: &str,
    ) -> Result<String, BrowserError> {
        if self.crash_reason.lock().unwrap().is_some() {
            return Err(BrowserError::Crash("engine crashed during script".into()));
        }
        Ok(format!("mock script result for: {}", script))
    }

    fn last_crash_reason(&self) -> Option<String> {
        self.crash_reason.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_engine_identity() {
        let engine = MockBrowserEngineAdapter::new();
        assert_eq!(engine.engine_name(), "mock");
        assert!(engine.is_experimental());
        assert!(engine.capabilities().supports_js);
    }

    #[test]
    fn create_and_navigate_session() {
        let engine = MockBrowserEngineAdapter::new();
        engine.initialize().unwrap();
        let session = engine
            .create_session(BrowserConfig {
                profile: "default".into(),
                url: None,
                off_the_record: false,
            })
            .unwrap();
        engine.navigate(&session, "https://example.com").unwrap();
        assert_eq!(engine.last_crash_reason(), None);
    }

    #[test]
    fn engine_crash_is_isolated() {
        let engine = MockBrowserEngineAdapter::new();
        engine.simulate_crash("renderer crashed");
        assert!(engine.initialize().is_err());
        assert_eq!(engine.last_crash_reason(), Some("renderer crashed".into()));
        // The mock adapter itself still exists — the *core* is not torn down.
        engine.clear_crash();
        engine.initialize().unwrap();
    }

    #[test]
    fn one_engine_per_instance() {
        let a = MockBrowserEngineAdapter::new();
        let b = MockBrowserEngineAdapter::new();
        assert_eq!(a.engine_name(), "mock");
        assert_eq!(b.engine_name(), "mock");
        // Two separate adapter instances — no shared internal state (SRD §2.2).
    }
}
