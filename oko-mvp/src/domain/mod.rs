//! Domain layer — command definitions, sessions, capability policy,
//! configuration, and lifecycle state.
//!
//! The domain imports nothing outside this crate.

pub mod capability;
pub mod command;
pub mod config;
pub mod lifecycle;
pub mod session;
