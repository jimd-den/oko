//! Platform adapters — Linux implementations of the ports.
//!
//! The domain imports nothing from here. Adapters implement port traits and
//! are wired in `oko-host`. Swap an adapter to change platform/engine without
//! touching domain or application code (SRD §4.2, §4.3).

pub mod file_storage;
pub mod linux_pty;
pub mod linux_process;
pub mod system_clock;
