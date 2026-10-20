//! Browser engine adapters.
//!
//! Servo is the engine this project supports. It is a prebuilt release driven
//! as a separate process, so the adapter adds no Cargo dependency and no
//! build-time download. `mock` is a dev stub for tests, never a production
//! fallback. `select` maps a configured engine name to one adapter, and
//! `unavailable` stands in for a name this build cannot serve (SRD §5.2,
//! ENG-001, ENG-008).

pub mod engine_binary;
pub mod mock;
pub mod select;
pub mod servo;
pub mod unavailable;
