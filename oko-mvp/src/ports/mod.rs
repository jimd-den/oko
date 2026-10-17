//! Ports — abstract interfaces for processes, terminal sessions, storage,
//! clock, and events (SRD §4.3).
//!
//! The domain and application layers depend on these interfaces, not on
//! concrete platform classes.

pub mod clock;
pub mod process;
pub mod storage;
pub mod terminal;
