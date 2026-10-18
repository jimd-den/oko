//! Browser engine adapters.
//!
//! Each backend is a separate adapter and build configuration; exactly one
//! serves a browser instance (SRD §5.2, ENG-001). `select` maps a configured
//! engine name to the adapter, and `unavailable` stands in for a name this
//! build cannot serve.

pub mod embedder;
pub mod mock;
pub mod select;
pub mod unavailable;

// Blink adapter: gated behind the `blink` feature. Requires a separately
// built Chromium embedder (SRD §5.1, §14).
#[cfg(feature = "blink")]
pub mod blink;

// Servo adapter: gated behind the `servo` feature. Requires a separately
// built Servo checkout (SRD §2.1, §5.2).
#[cfg(feature = "servo")]
pub mod servo;

// Blink install flow (download → verify checksum → build). Always available.
// Never runs silently at compile time; invoked via `blink install` (SRD DEP-002).
pub mod blink_install;
