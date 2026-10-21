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

pub mod bmp;
pub mod compositor;
pub mod fb;
#[cfg(target_os = "linux")]
pub mod fbdev;
pub mod presenter;
pub mod protocol;
pub mod session;
pub mod surface;
pub mod terminal;

pub use bmp::{decode_bmp, BmpError};
pub use compositor::Compositor;
pub use fb::{FramebufferDevice, FramebufferInfo, FramebufferPresenter, MemoryFramebuffer, PixelFormat};
#[cfg(target_os = "linux")]
pub use fbdev::LinuxFbdev;
pub use presenter::{HeadlessPresenter, OutputSize, Presenter, PresenterError};
pub use protocol::{ClientMessage, DisplayError, InputEvent, ServerMessage};
pub use session::DisplaySession;
pub use surface::{Frame, Surface, SurfaceId};
pub use terminal::TerminalPresenter;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod fb_tests;
#[cfg(all(test, target_os = "linux"))]
mod fbdev_tests;
