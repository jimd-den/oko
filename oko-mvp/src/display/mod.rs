//! Display server core: surfaces, composition, presentation.
//!
//! This is the device-independent half of ỌKỌ's display server. An app owns a
//! `Surface` of RGBA pixels; the `Compositor` stacks surfaces into one `Frame`;
//! a `Presenter` puts that frame on an output device.
//!
//! Deliberately not here: input routing, the app/IPC protocol, and any HTML or
//! browser knowledge. Those are separate adapters. This is the pixel contract
//! they share, so the same app can be shown on a TTY, a framebuffer, or a
//! browser canvas without changing the app.

pub mod compositor;
pub mod presenter;
pub mod surface;
pub mod terminal;

pub use compositor::Compositor;
pub use presenter::{HeadlessPresenter, OutputSize, Presenter, PresenterError};
pub use surface::{Frame, Surface, SurfaceId};
pub use terminal::TerminalPresenter;

#[cfg(test)]
mod tests;
