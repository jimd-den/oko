//! ỌKỌ core — domain and application layer.
//!
//! This crate contains the core domain logic and application services. It
//! intentionally imports no UI, browser, GPU, or OS implementation (SRD §4.2).
//! Platform adapters live in `adapters` and are wired in `oko-host`.

pub mod application;
pub mod adapters;
pub mod browser;
pub mod domain;
pub mod ipc;
pub mod ports;

pub use domain::command::{parse_command, validate_command, Command, CommandResult};
pub use domain::session::Session;
pub use domain::capability::Capability;
