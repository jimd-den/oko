//! Presenter port: put a composed frame on an output device.
//!
//! The display core produces `Frame`s. A presenter is the only device-specific
//! piece: a terminal, a framebuffer, a DRM/KMS plane, an X11/Wayland window, or
//! a browser canvas. The contract is one call, so a new output does not touch
//! the compositor or any app.

use super::surface::Frame;

/// The units a presenter natively works in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputSize {
    /// A pixel-addressed device, such as a framebuffer.
    Pixels(u32, u32),
    /// A character-addressed device, such as a terminal.
    Cells(u32, u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenterError {
    /// The frame has no pixels; there is nothing to show.
    EmptyFrame,
    /// The device cannot accept this frame size.
    WrongSize {
        expected: (u32, u32),
        got: (u32, u32),
    },
    /// The output device failed while writing.
    Io(String),
}

/// One output device.
pub trait Presenter {
    fn output_size(&self) -> OutputSize;

    /// Show `frame` now. Contract: the presenter does not retain the frame
    /// unless its implementation says so; a caller may reuse the buffer.
    fn present(&mut self, frame: &Frame) -> Result<(), PresenterError>;
}

/// A presenter with no device: it keeps the last frame for tests and tools.
pub struct HeadlessPresenter {
    width: u32,
    height: u32,
    last: Option<Frame>,
}

impl HeadlessPresenter {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            last: None,
        }
    }

    pub fn last_frame(&self) -> Option<&Frame> {
        self.last.as_ref()
    }
}

impl Presenter for HeadlessPresenter {
    fn output_size(&self) -> OutputSize {
        OutputSize::Pixels(self.width, self.height)
    }

    fn present(&mut self, frame: &Frame) -> Result<(), PresenterError> {
        if frame.width == 0 || frame.height == 0 {
            return Err(PresenterError::EmptyFrame);
        }
        if frame.width != self.width || frame.height != self.height {
            return Err(PresenterError::WrongSize {
                expected: (self.width, self.height),
                got: (frame.width, frame.height),
            });
        }
        self.last = Some(frame.clone());
        Ok(())
    }
}
