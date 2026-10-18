//! Servo browser engine adapter (SRD §2.1, §5.2 — experimental engine stack).
//!
//! Servo is a Rust-based, memory-safe, modular browser engine with a WebView
//! embedding API. It is gated behind the `servo` feature flag because it
//! requires a separately built Servo checkout to link.
//!
//! Per SRD §5.2, Servo is the experimental alternative engine backend. Its
//! embedder interface and script-engine compatibility must be verified before
//! locking the build. It is visibly marked experimental (SRD ENG-009).
//!
//! Exactly one engine backend serves one browser instance (SRD ENG-001).
//! Multiple engines must not share undocumented internal objects, heaps,
//! DOM trees or rendering state (SRD §2.2).
//!
//! Package layout: `ffi` is the Servo embedder ABI this adapter links against,
//! `port` is how it satisfies `BrowserEnginePort`, and this file owns the
//! adapter's own state and construction.

mod ffi;
mod port;

#[cfg(test)]
mod tests;

use super::embedder::EmbedderHandle;
use crate::browser::ports::engine::{BrowserError, BrowserSession};
use std::collections::HashMap;
use std::sync::Mutex;

/// Session tracking for the Servo backend. Engine-neutral on this side of
/// the boundary; the Servo side owns the real WebView/compositing state.
#[derive(Debug, Clone)]
struct ServoSession {
    pub url: Option<String>,
    pub closed: bool,
}

pub struct ServoBrowserEngineAdapter {
    embedder: EmbedderHandle,
    sessions: Mutex<HashMap<BrowserSession, ServoSession>>,
    next_id: Mutex<u64>,
    crash_reason: Mutex<Option<String>>,
}

impl Default for ServoBrowserEngineAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ServoBrowserEngineAdapter {
    pub fn new() -> Self {
        Self {
            embedder: EmbedderHandle::unlinked(),
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            crash_reason: Mutex::new(None),
        }
    }

    /// Construct with a pre-linked Servo embedder handle (production).
    pub fn with_embedder(handle: *mut std::ffi::c_void) -> Self {
        Self {
            embedder: EmbedderHandle::from_address(handle),
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
        if !self.embedder.is_linked() {
            return Err(BrowserError::EngineUnavailable(
                "servo adapter not linked against a Servo build; \
                 enable the `servo` feature and link a pinned Servo \
                 checkout (SRD §5.2, §14)"
                    .into(),
            ));
        }
        Ok(())
    }
}
