//! Software renderer (SRD GFX-008: software rendering fallback).
//!
//! A dependency-free software renderer that produces a PPM image. Used as the
//! graphics fallback when no GPU backend is available, and to render the
//! `render demo` output without Blink/Vulkan/Direct3D.

/// RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// A software framebuffer.
#[derive(Debug, Clone)]
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Rgba>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![Rgba::new(0, 0, 0); (width * height) as usize],
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: Rgba) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Render an ỌKỌ demo: gradient background, logo block, status text line.
    pub fn render_demo(&mut self, message: &str) {
        // Gradient background: dark blue -> teal.
        for y in 0..self.height {
            for x in 0..self.width {
                let t = x as f32 / self.width.max(1) as f32;
                let u = y as f32 / self.height.max(1) as f32;
                self.set_pixel(
                    x,
                    y,
                    Rgba::new(
                        (10.0 + 40.0 * t) as u8,
                        (30.0 + 90.0 * u) as u8,
                        (60.0 + 120.0 * (t + u) / 2.0) as u8,
                    ),
                );
            }
        }
        // Logo block (Ọ).
        let cx = self.width / 2;
        let cy = self.height / 2;
        let r = (self.width.min(self.height) / 4) as i32;
        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x as i32 - cx as i32;
                let dy = y as i32 - cy as i32;
                if dx * dx + dy * dy <= r * r {
                    self.set_pixel(x, y, Rgba::new(77, 166, 255));
                }
            }
        }
        // Border.
        for x in 0..self.width {
            self.set_pixel(x, 0, Rgba::new(255, 255, 255));
            self.set_pixel(x, self.height - 1, Rgba::new(255, 255, 255));
        }
        for y in 0..self.height {
            self.set_pixel(0, y, Rgba::new(255, 255, 255));
            self.set_pixel(self.width - 1, y, Rgba::new(255, 255, 255));
        }
        let _ = message; // text rendering deferred to a font rasterizer
    }

    /// Export as PPM (P6 binary).
    pub fn to_ppm(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(format!("P6\n{} {}\n255\n", self.width, self.height).as_bytes());
        for p in &self.pixels {
            out.push(p.r);
            out.push(p.g);
            out.push(p.b);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framebuffer_render_demo_produces_ppm() {
        let mut fb = Framebuffer::new(64, 48);
        fb.render_demo("oko demo");
        let ppm = fb.to_ppm();
        assert!(ppm.starts_with(b"P6\n64 48\n255\n"));
        assert_eq!(ppm.len(), 13 + 64 * 48 * 3); // 13-byte header + RGB pixels
    }

    #[test]
    fn set_pixel_out_of_bounds_is_noop() {
        let mut fb = Framebuffer::new(8, 8);
        fb.set_pixel(100, 100, Rgba::new(255, 0, 0));
        assert_eq!(fb.pixels[0], Rgba::new(0, 0, 0));
    }
}
