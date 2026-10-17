//! Browser engine adapters.

pub mod mock;

// Blink adapter: gated behind the `blink` feature. Requires a separately
// built Chromium embedder (SRD §5.1, §14).
#[cfg(feature = "blink")]
pub mod blink;

// Blink install flow (download → verify checksum → build). Always available.
// Never runs silently at compile time; invoked via `blink install` (SRD DEP-002).
pub mod blink_install;
