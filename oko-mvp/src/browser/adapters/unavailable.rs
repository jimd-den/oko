//! An engine name this build does not contain.
//!
//! Selection is by name (SRD ENG-002). A name the build cannot serve must be
//! reported as unavailable instead of being quietly replaced by another engine,
//! because a browser that falls back to a different engine makes every
//! capability and security claim about it false (SRD ENG-008).
//!
//! The adapter keeps the requested identity, so the caller sees exactly what it
//! asked for and why it is not there.

use crate::browser::ports::engine::{
    BackendCapabilities, BrowserConfig, BrowserEnginePort, BrowserError, BrowserSession,
};

pub struct UnavailableBrowserEngine {
    name: String,
    reason: String,
}

impl UnavailableBrowserEngine {
    pub fn new(name: &str, reason: &str) -> Self {
        Self {
            name: name.into(),
            reason: reason.into(),
        }
    }

    fn unavailable(&self) -> BrowserError {
        BrowserError::EngineUnavailable(self.reason.clone())
    }
}

impl BrowserEnginePort for UnavailableBrowserEngine {
    fn initialize(&self) -> Result<(), BrowserError> {
        Err(self.unavailable())
    }
    fn shutdown(&self) -> Result<(), BrowserError> {
        Ok(())
    }
    fn engine_name(&self) -> &str {
        &self.name
    }
    fn engine_version(&self) -> &str {
        "unavailable"
    }
    /// Not a claim about the upstream project: an engine this build cannot
    /// provide is unproven here, which is what the experimental marking means
    /// to a caller (SRD ENG-009).
    fn is_experimental(&self) -> bool {
        true
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }
    fn create_session(&self, _config: BrowserConfig) -> Result<BrowserSession, BrowserError> {
        Err(self.unavailable())
    }
    fn navigate(&self, _session: &BrowserSession, _url: &str) -> Result<(), BrowserError> {
        Err(self.unavailable())
    }
    fn close_session(&self, _session: &BrowserSession) -> Result<(), BrowserError> {
        Err(self.unavailable())
    }
    fn execute_script(
        &self,
        _session: &BrowserSession,
        _script: &str,
    ) -> Result<String, BrowserError> {
        Err(self.unavailable())
    }
    fn last_crash_reason(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_requested_identity_and_says_why_it_is_absent() {
        let engine = UnavailableBrowserEngine::new("servo", "not enabled in this build");
        assert_eq!(engine.engine_name(), "servo");
        assert!(matches!(
            engine.initialize(),
            Err(BrowserError::EngineUnavailable(reason)) if reason.contains("not enabled")
        ));
    }
}
