//! The typed surface protocol between an app and the display server.
//!
//! These are the messages the server understands. They are engine-neutral and
//! carry no DOM, no scripting handle and no device object: an app sends pixels,
//! receives input, and owns nothing else. A socket transport serializes this
//! vocabulary; the in-process session tests drive the same messages directly.
//!
//! Versioning note: this is the v1 vocabulary. A transport must carry the
//! version and refuse a mismatch; do not grow a field and assume old clients
//! will ignore it.

use super::surface::{Frame, SurfaceId};

/// The largest surface the server will allocate, in pixels per side.
pub const MAX_SURFACE_DIMENSION: u32 = 8192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientMessage {
    Create {
        id: SurfaceId,
        width: u32,
        height: u32,
    },
    Commit {
        id: SurfaceId,
        frame: Frame,
    },
    Destroy {
        id: SurfaceId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMessage {
    Created { id: SurfaceId },
    Committed { id: SurfaceId },
    Destroyed { id: SurfaceId },
    Focused { id: Option<SurfaceId> },
    Input { id: SurfaceId, event: InputEvent },
    Error { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Key { code: u32, pressed: bool },
    Pointer { x: i32, y: i32, buttons: u32 },
    Focus { focused: bool },
    Resize { width: u32, height: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayError {
    UnknownSurface(SurfaceId),
    SurfaceExists(SurfaceId),
    SizeMismatch {
        expected: (u32, u32),
        got: (u32, u32),
    },
    TooLarge {
        width: u32,
        height: u32,
    },
    EmptyFrame,
}

impl DisplayError {
    pub fn reason(&self) -> String {
        match self {
            Self::UnknownSurface(id) => format!("unknown surface {}", id.0),
            Self::SurfaceExists(id) => format!("surface {} already exists", id.0),
            Self::SizeMismatch { expected, got } => {
                format!("frame is {:?}, surface is {:?}", got, expected)
            }
            Self::TooLarge { width, height } => {
                format!("surface {}x{} exceeds the {} px limit", width, height, MAX_SURFACE_DIMENSION)
            }
            Self::EmptyFrame => "frame has no pixels".into(),
        }
    }
}
