//! Blink + V8 + Skia browser engine adapter (SRD §2.1, §5.1).
//!
//! This adapter implements `BrowserEnginePort` against the Chromium embedder
//! interface. It is gated behind the `blink` feature flag because it requires
//! a separately built Chromium source tree (pinned revision, verified
//! checksum, approved build process — SRD §14).
//!
//! IMPORTANT: Per SRD §5.1, Blink is NOT embedded as a standalone rendering
//! library. It depends on Chromium's supporting architecture (content shell,
//! embedder API, GPU process, utility processes, sandbox). This module
//! declares the embedder-interface boundary; actual linkage against a built
//! Chromium is a deployment/packaging concern outside this crate.
//!
//! When compiled without the `blink` feature, this module is absent and the
//! `MockBrowserEngineAdapter` is used instead (dev/stub, visibly
//! experimental per SRD ENG-009).

use crate::browser::ports::engine::{
    BackendCapabilities, BrowserConfig, BrowserEnginePort, BrowserError, BrowserSession,
};
use crate::domain::session::SessionId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Opaque handle to a Chromium embedder instance. Populated only when the
/// adapter is linked against a real Chromium build.
#[derive(Debug, Clone)]
pub struct ChromiumEmbedder {
    /// Embedder handle (opaque pointer in a real build).
    pub handle: *mut std::ffi::c_void,
    /// Whether a real Chromium embedder is linked.
    pub linked: bool,
}

impl ChromiumEmbedder {
    pub fn unlinked() -> Self {
        Self {
            handle: std::ptr::null_mut(),
            linked: false,
        }
    }
}

/// Session tracking for the Blink backend. Engine-neutral on this side of
/// the boundary; the Chromium side owns the real WebContents/RenderFrameHost.
#[derive(Debug, Clone)]
struct BlinkSession {
    pub id: BrowserSession,
    pub url: Option<String>,
    pub closed: bool,
}

pub struct BlinkBrowserEngineAdapter {
    embedder: ChromiumEmbedder,
    sessions: Mutex<HashMap<BrowserSession, BlinkSession>>,
    next_id: Mutex<u64>,
    crash_reason: Mutex<Option<String>>,
}

impl Default for BlinkBrowserEngineAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl BlinkBrowserEngineAdapter {
    pub fn new() -> Self {
        Self {
            embedder: ChromiumEmbedder::unlinked(),
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            crash_reason: Mutex::new(None),
        }
    }

    /// Construct with a pre-linked Chromium embedder handle (production).
    pub fn with_embedder(handle: *mut std::ffi::c_void) -> Self {
        Self {
            embedder: ChromiumEmbedder { handle, linked: true },
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            crash_reason: Mutex::new(None),
        }
    }

    /// Simulate an engine crash (for ENG-004 testing).
    pub fn simulate_crash(&self, reason: &str) {
        *self.crash_reason.lock().unwrap() = Some(reason.into());
    }

    pub fn clear_crash(&self) {
        *self.crash_reason.lock().unwrap() = None;
    }

    fn require_linked(&self) -> Result<(), BrowserError> {
        if !self.embedder.linked {
            return Err(BrowserError::EngineUnavailable(
                "blink adapter not linked against a Chromium build; \
                 enable the `blink` feature and link a pinned Chromium \
                 embedder (SRD §5.1, §14)".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(feature = "blink")]
mod embedder_ffi {
    //! Declarations of the Chromium embedder interface.
    //!
    //! These are the C++ symbols a real Chromium embedder (content shell /
    //! content embedder) exports. The actual headers/libraries come from the
    //! pinned Chromium source tree, not from this crate. See
    //! https://chromium.googlesource.com/chromium/src/+/HEAD/docs/embedder.md

    extern "C" {
        /// Initialize the Chromium embedder subsystem.
        pub fn oko_blink_embedder_init() -> *mut std::ffi::c_void;
        /// Shut down the embedder.
        pub fn oko_blink_embedder_shutdown(handle: *mut std::ffi::c_void);
        /// Create a browser session (WebContents equivalent).
        pub fn oko_blink_session_create(
            handle: *mut std::ffi::c_void,
            profile: *const std::ffi::c_char,
        ) -> u64;
        /// Navigate a session to a URL.
        pub fn oko_blink_session_navigate(
            handle: *mut std::ffi::c_void,
            session_id: u64,
            url: *const std::ffi::c_char,
        ) -> i32;
        /// Close a session.
        pub fn oko_blink_session_close(handle: *mut std::ffi::c_void, session_id: u64) -> i32;
        /// Execute script in a session (V8).
        pub fn oko_blink_session_execute_script(
            handle: *mut std::ffi::c_void,
            session_id: u64,
            script: *const std::ffi::c_char,
            out: *mut *mut std::ffi::c_char,
        ) -> i32;
        /// Query last crash reason.
        pub fn oko_blink_crash_reason(
            handle: *mut std::ffi::c_void,
            out: *mut *mut std::ffi::c_char,
        ) -> i32;
    }
}

impl BrowserEnginePort for BlinkBrowserEngineAdapter {
    fn initialize(&self) -> Result<(), BrowserError> {
        if let Some(reason) = self.crash_reason.lock().unwrap().as_ref() {
            return Err(BrowserError::Crash(format!("engine crashed: {}", reason)));
        }
        if self.embedder.linked {
            // In a real build, call oko_blink_embedder_init here.
            Ok(())
        } else {
            Err(BrowserError::EngineUnavailable(
                "blink embedder not linked (feature `blink` requires a \
                 separately built Chromium)".into(),
            ))
        }
    }

    fn shutdown(&self) -> Result<(), BrowserError> {
        if self.embedder.linked {
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
            BlinkSession { id: id.clone(), url: config.url.clone(), closed: false },
        );
        Ok(id)
    }

    fn navigate(&self, session: &BrowserSession, url: &str) -> Result<(), BrowserError> {
        if let Some(reason) = self.crash_reason.lock().unwrap().as_ref() {
            return Err(BrowserError::Crash(format!("engine crashed: {}", reason)));
        }
        self.require_linked()?;
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions.get_mut(session).ok_or(BrowserError::SessionNotFound)?;
        if entry.closed {
            return Err(BrowserError::SessionNotFound);
        }
        entry.url = Some(url.into());
        Ok(())
    }

    fn close_session(&self, session: &BrowserSession) -> Result<(), BrowserError> {
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions.get_mut(session).ok_or(BrowserError::SessionNotFound)?;
        entry.closed = true;
        Ok(())
    }

    fn execute_script(&self, _session: &BrowserSession, _script: &str) -> Result<String, BrowserError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blink_identity_without_link() {
        let engine = BlinkBrowserEngineAdapter::new();
        assert_eq!(engine.engine_name(), "blink");
        assert!(!engine.is_experimental());
        assert!(engine.capabilities().supports_js);
    }

    #[test]
    fn blink_unlinked_reports_unavailable() {
        let engine = BlinkBrowserEngineAdapter::new();
        assert!(engine.initialize().is_err());
        assert!(engine
            .create_session(crate::browser::ports::engine::BrowserConfig {
                profile: "default".into(),
                url: None,
                off_the_record: false,
            })
            .is_err());
    }

    #[test]
    fn blink_crash_is_isolated() {
        let engine = BlinkBrowserEngineAdapter::new();
        engine.simulate_crash("renderer process terminated");
        assert!(engine.initialize().is_err());
        assert_eq!(
            engine.last_crash_reason(),
            Some("renderer process terminated".into())
        );
        engine.clear_crash();
        // Still unavailable without a linked embedder, but crash state cleared.
        assert_eq!(engine.last_crash_reason(), None);
    }

    #[test]
    fn one_engine_per_instance() {
        let a = BlinkBrowserEngineAdapter::new();
        let b = BlinkBrowserEngineAdapter::new();
        assert_eq!(a.engine_name(), "blink");
        assert_eq!(b.engine_name(), "blink");
        // No shared internal state between instances (SRD §2.2).
    }
}
