//! Loopback HTTP transport for the desktop UI.
//!
//! The stdio transport serves the terminal and automation. A browser engine
//! cannot speak stdio, so this one serves the same core over HTTP on loopback:
//! the desktop shell is fetched like any page and calls the host API with
//! `POST /api`, carrying the same typed, versioned messages (SRD UI-008).
//!
//! This is what makes the engine interchangeable: the shell talks HTTP, so any
//! engine that renders HTML and runs JavaScript can host it.

mod assets;
mod request;
mod response;
mod server;

pub use server::HttpUiServer;

#[cfg(test)]
mod tests;
