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
//!
//! Package layout: `ffi` is the Chromium embedder ABI this adapter links
//! against, `port` is how it satisfies `BrowserEnginePort`, and this file owns
//! the adapter's own state and construction.

mod ffi;
mod port;

#[cfg(test)]
mod tests;

use super::embedder::EmbedderHandle;
use crate::browser::ports::engine::{BrowserError, BrowserSession};
use std::collections::HashMap;
use std::sync::Mutex;

/// Session tracking for the Blink backend. Engine-neutral on this side of
/// the boundary; the Chromium side owns the real WebContents/RenderFrameHost.
#[derive(Debug, Clone)]
struct BlinkSession {
    pub url: Option<String>,
    pub closed: bool,
}

pub struct BlinkBrowserEngineAdapter {
    embedder: EmbedderHandle,
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
            embedder: EmbedderHandle::unlinked(),
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            crash_reason: Mutex::new(None),
        }
    }

    /// Construct with a pre-linked Chromium embedder handle (production).
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
                "blink adapter not linked against a Chromium build; \
                 enable the `blink` feature and link a pinned Chromium \
                 embedder (SRD §5.1, §14)"
                    .into(),
            ));
        }
        Ok(())
    }
}
