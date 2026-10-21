//! Compositor: stack app surfaces into one frame.
//!
//! Painter's algorithm, bottom to top by `z`, with source-over alpha blending.
//! Every surface is clipped to the output; a surface outside the frame costs
//! nothing. This is deliberately a pure function of the scene: the same scene
//! always produces the same frame, which is what makes presenters swappable.

use super::surface::{Frame, Surface, SurfaceId};

/// A scene of positioned surfaces and the frame they compose into.
pub struct Compositor {
    pub width: u32,
    pub height: u32,
    background: [u8; 4],
    surfaces: Vec<Surface>,
}

impl Compositor {
    /// A black, opaque scene of `width` x `height`.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            background: [0, 0, 0, 255],
            surfaces: Vec::new(),
        }
    }

    pub fn with_background(mut self, rgba: [u8; 4]) -> Self {
        self.background = rgba;
        self
    }

    /// Add a surface, replacing one with the same id. Returns the old surface.
    pub fn add(&mut self, surface: Surface) -> Option<Surface> {
        match self.surfaces.iter().position(|s| s.id == surface.id) {
            Some(i) => Some(std::mem::replace(&mut self.surfaces[i], surface)),
            None => {
                self.surfaces.push(surface);
                None
            }
        }
    }

    pub fn remove(&mut self, id: SurfaceId) -> Option<Surface> {
        let i = self.surfaces.iter().position(|s| s.id == id)?;
        Some(self.surfaces.remove(i))
    }

    /// Put `id` above every other surface.
    pub fn raise(&mut self, id: SurfaceId) -> bool {
        let top = self.surfaces.iter().map(|s| s.z).max().unwrap_or(0);
        match self.surfaces.iter_mut().find(|s| s.id == id) {
            Some(surface) => {
                surface.z = top.saturating_add(1);
                true
            }
            None => false,
        }
    }

    pub fn surfaces(&self) -> &[Surface] {
        &self.surfaces
    }

    pub fn surfaces_mut(&mut self) -> &mut [Surface] {
        &mut self.surfaces
    }

    /// Compose the scene into a new frame.
    pub fn composite(&self) -> Frame {
        let mut frame = Frame::new(self.width, self.height);
        for i in (0..frame.pixels.len()).step_by(4) {
            frame.pixels[i..i + 4].copy_from_slice(&self.background);
        }
        let mut order: Vec<&Surface> = self.surfaces.iter().filter(|s| s.visible).collect();
        order.sort_by_key(|s| s.z);
        for surface in order {
            self.draw(&mut frame, surface);
        }
        frame
    }

    fn draw(&self, frame: &mut Frame, surface: &Surface) {
        for sy in 0..surface.height {
            let dy = surface.y + sy as i32;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            for sx in 0..surface.width {
                let dx = surface.x + sx as i32;
                if dx < 0 || dx >= self.width as i32 {
                    continue;
                }
                let src = match surface.pixel(sx, sy) {
                    Some(p) if p[3] != 0 => p,
                    _ => continue,
                };
                let di = ((dy as usize * self.width as usize) + dx as usize) * 4;
                if src[3] == 255 {
                    frame.pixels[di..di + 4].copy_from_slice(&src);
                } else {
                    blend(&mut frame.pixels[di..di + 4], src);
                }
            }
        }
    }
}

/// Source-over blend, with the destination already opaque in the normal case.
fn blend(dst: &mut [u8], src: [u8; 4]) {
    let sa = src[3] as u32;
    let da = dst[3] as u32;
    let out_a = sa + da * (255 - sa) / 255;
    if out_a == 0 {
        dst.copy_from_slice(&[0, 0, 0, 0]);
        return;
    }
    for c in 0..3 {
        let sc = src[c] as u32 * sa;
        let dc = dst[c] as u32 * da * (255 - sa) / 255;
        dst[c] = ((sc + dc) / out_a) as u8;
    }
    dst[3] = out_a as u8;
}
