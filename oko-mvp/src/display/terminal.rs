//! Terminal presenter: a composed frame as truecolor ANSI half-blocks.
//!
//! Each terminal cell carries two vertical pixels: the upper half block with
//! the top pixel as foreground and the bottom pixel as background. That is the
//! smallest presenter that can show a pixel surface on a bare TTY with no
//! display server, and it works over a serial console.
//!
//! The frame is scaled to the cell grid by averaging each region. A cell is
//! therefore one "display pixel"; the resolution is 2 x rows.

use super::presenter::{OutputSize, Presenter, PresenterError};
use super::surface::Frame;
use std::io::Write;

pub struct TerminalPresenter<W: Write> {
    out: W,
    cols: u32,
    rows: u32,
}

impl<W: Write> TerminalPresenter<W> {
    pub fn new(out: W, cols: u32, rows: u32) -> Self {
        Self { out, cols, rows }
    }

    pub fn into_inner(self) -> W {
        self.out
    }
}

impl<W: Write> Presenter for TerminalPresenter<W> {
    fn output_size(&self) -> OutputSize {
        OutputSize::Cells(self.cols, self.rows)
    }

    fn present(&mut self, frame: &Frame) -> Result<(), PresenterError> {
        if frame.width == 0 || frame.height == 0 {
            return Err(PresenterError::EmptyFrame);
        }
        if self.cols == 0 || self.rows == 0 {
            return Err(PresenterError::WrongSize {
                expected: (self.cols, self.rows),
                got: (frame.width, frame.height),
            });
        }
        // Home the cursor so a live server overwrites its last frame instead of
        // appending to it. The last row gets no newline, which would scroll.
        write!(self.out, "\x1b[H").map_err(|e| PresenterError::Io(e.to_string()))?;
        for row in 0..self.rows {
            for col in 0..self.cols {
                let (x0, x1) = span(col, self.cols, frame.width);
                let (t0, t1) = span(2 * row, 2 * self.rows, frame.height);
                let (b0, b1) = span(2 * row + 1, 2 * self.rows, frame.height);
                let top = average(frame, x0, t0, x1, t1);
                let bottom = average(frame, x0, b0, x1, b1);
                write!(
                    self.out,
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m\u{2580}",
                    top[0], top[1], top[2], bottom[0], bottom[1], bottom[2]
                )
                .map_err(|e| PresenterError::Io(e.to_string()))?;
            }
            let separator = if row + 1 < self.rows { "\n" } else { "" };
            write!(self.out, "\x1b[0m{}", separator)
                .map_err(|e| PresenterError::Io(e.to_string()))?;
        }
        Ok(())
    }
}

/// The half-open source range for cell `i` of `n` covering `total` pixels.
fn span(i: u32, n: u32, total: u32) -> (u32, u32) {
    let start = i * total / n;
    let end = ((i + 1) * total / n).max(start + 1).min(total);
    (start, end)
}

/// Average a rectangular region. Regions are never empty after `span`.
fn average(frame: &Frame, x0: u32, y0: u32, x1: u32, y1: u32) -> [u8; 3] {
    let mut sum = [0u32; 3];
    let mut count = 0u32;
    for y in y0..y1 {
        for x in x0..x1 {
            if let Some(p) = frame.pixel(x, y) {
                sum[0] += p[0] as u32;
                sum[1] += p[1] as u32;
                sum[2] += p[2] as u32;
                count += 1;
            }
        }
    }
    if count == 0 {
        return [0, 0, 0];
    }
    [
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
    ]
}
