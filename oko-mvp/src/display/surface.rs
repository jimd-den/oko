//! Surfaces and frames: the two pixel containers in the display core.
//!
//! A `Surface` belongs to an app and carries a position in the scene. A
//! `Frame` is the compositor's output. Both store RGBA8888 rows, top to bottom,
//! with no padding, so a presenter can copy or scale them without a format
//! negotiation.

/// An opaque surface handle. The display core never interprets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfaceId(pub u64);

/// A rectangle of RGBA8888 pixels owned by one app.
#[derive(Debug, Clone)]
pub struct Surface {
    pub id: SurfaceId,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub visible: bool,
    pixels: Vec<u8>,
}

impl Surface {
    /// A transparent surface at the origin.
    pub fn new(id: SurfaceId, width: u32, height: u32) -> Self {
        Self {
            id,
            width,
            height,
            x: 0,
            y: 0,
            z: 0,
            visible: true,
            pixels: vec![0; pixels_len(width, height)],
        }
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn set_z(&mut self, z: i32) {
        self.z = z;
    }

    /// Write one RGBA pixel. Returns false when the point is outside.
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) -> bool {
        match self.index(x, y) {
            Some(i) => {
                self.pixels[i..i + 4].copy_from_slice(&rgba);
                true
            }
            None => false,
        }
    }

    /// Read one RGBA pixel, or `None` when the point is outside.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let i = self.index(x, y)?;
        Some([
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ])
    }

    /// Fill the whole surface with one RGBA value.
    pub fn clear(&mut self, rgba: [u8; 4]) {
        for i in (0..self.pixels.len()).step_by(4) {
            self.pixels[i..i + 4].copy_from_slice(&rgba);
        }
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(((y as usize * self.width as usize) + x as usize) * 4)
    }
}

/// The compositor's output: one RGBA8888 image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; pixels_len(width, height)],
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = ((y as usize * self.width as usize) + x as usize) * 4;
        self.pixels[i..i + 4].copy_from_slice(&rgba);
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = ((y as usize * self.width as usize) + x as usize) * 4;
        Some([
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ])
    }
}

fn pixels_len(width: u32, height: u32) -> usize {
    width as usize * height as usize * 4
}
