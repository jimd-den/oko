//! Shared test doubles for the host's IPC and browser-engine boundaries.
//!
//! Integration tests are separate crates, so they cannot share a module the
//! ordinary way. A test file loads this package with
//! `#[path = "../support/mod.rs"] mod support;`.
//!
//! Every test crate compiles this package separately, so a helper one test file
//! uses looks dead to the others. That is the reason for the crate-level allow
//! rather than a pile of per-item attributes.

#![allow(dead_code)]

pub mod engine_conformance;
pub mod fake_engines;
pub mod host_binary;
pub mod scripted_transport;
