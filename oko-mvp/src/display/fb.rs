//! Framebuffer presenter: a composed frame on a pixel-addressed device.
//!
//! The device is hidden behind `FramebufferDevice`, so the conversion and the
//! presenter are testable without `/dev/fb0`. The Linux fbdev adapter is in
//! `fbdev`; a FreeBSD vt(4) or OpenBSD wscons adapter implements the same
//! three methods.
//!
//! Formats are explicit because a framebuffer's byte order is not negotiable.
//! The presenter converts once per frame into a reusable buffer.

use super::presenter::{OutputSize, Presenter, PresenterError};
use super::surface::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba8888,
    Bgra8888,
    Rgb565,
}

impl PixelFormat {
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba8888 | Self::Bgra8888 => 4,
            Self::Rgb565 => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferInfo {
    pub width: u32,
    pub height: u32,
    /// Bytes between the start of one row and the next. May exceed
    /// `width * bytes_per_pixel`.
    pub stride: usize,
    pub format: PixelFormat,
}

/// A pixel-addressed device the presenter can write.
pub trait FramebufferDevice {
    fn info(&self) -> FramebufferInfo;
    /// Write one full frame in the device's format and stride.
    fn write(&mut self, bytes: &[u8]) -> Result<(), String>;
}

pub struct FramebufferPresenter<D: FramebufferDevice> {
    device: D,
    buffer: Vec<u8>,
}

impl<D: FramebufferDevice> FramebufferPresenter<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            buffer: Vec::new(),
        }
    }

    pub fn device(&self) -> &D {
        &self.device
    }
}

impl<D: FramebufferDevice> Presenter for FramebufferPresenter<D> {
    fn output_size(&self) -> OutputSize {
        let info = self.device.info();
        OutputSize::Pixels(info.width, info.height)
    }

    fn present(&mut self, frame: &Frame) -> Result<(), PresenterError> {
        let info = self.device.info();
        if frame.width == 0 || frame.height == 0 {
            return Err(PresenterError::EmptyFrame);
        }
        if frame.width != info.width || frame.height != info.height {
            return Err(PresenterError::WrongSize {
                expected: (info.width, info.height),
                got: (frame.width, frame.height),
            });
        }
        let need = info.stride * info.height as usize;
        if self.buffer.len() != need {
            self.buffer.resize(need, 0);
        }
        encode(frame, info, &mut self.buffer);
        self.device.write(&self.buffer).map_err(PresenterError::Io)
    }
}

fn encode(frame: &Frame, info: FramebufferInfo, out: &mut [u8]) {
    let bpp = info.format.bytes_per_pixel();
    for y in 0..info.height {
        let row = y as usize * info.stride;
        for x in 0..info.width {
            let [r, g, b, _] = frame.pixel(x, y).unwrap_or([0, 0, 0, 255]);
            let i = row + x as usize * bpp;
            match info.format {
                PixelFormat::Rgba8888 => out[i..i + 4].copy_from_slice(&[r, g, b, 255]),
                PixelFormat::Bgra8888 => out[i..i + 4].copy_from_slice(&[b, g, r, 255]),
                PixelFormat::Rgb565 => {
                    let value = (((r as u16) >> 3) << 11)
                        | (((g as u16) >> 2) << 5)
                        | ((b as u16) >> 3);
                    out[i..i + 2].copy_from_slice(&value.to_le_bytes());
                }
            }
        }
    }
}

/// A framebuffer in memory, used by tests and by file-backed devices.
pub struct MemoryFramebuffer {
    info: FramebufferInfo,
    bytes: Vec<u8>,
}

impl MemoryFramebuffer {
    pub fn new(info: FramebufferInfo) -> Self {
        Self {
            info,
            bytes: vec![0; info.stride * info.height as usize],
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl FramebufferDevice for MemoryFramebuffer {
    fn info(&self) -> FramebufferInfo {
        self.info
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() != self.bytes.len() {
            return Err(format!(
                "frame is {} bytes, device is {}",
                bytes.len(),
                self.bytes.len()
            ));
        }
        self.bytes.copy_from_slice(bytes);
        Ok(())
    }
}
