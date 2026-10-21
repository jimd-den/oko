//! Wire mapping for the surface protocol over a Unix socket.
//!
//! The byte layout lives in `format`: a fixed 26-byte header followed by a
//! payload. This module maps that layout to `ClientMessage`/`ServerMessage`,
//! and refuses a commit whose payload is not exactly `width * height * 4`.

use super::format::{read_frame, write_frame, Kind};
use super::protocol::{ClientMessage, ServerMessage};
use super::surface::{Frame, SurfaceId};
use std::io::{Read, Write};

/// Read one client frame, or `None` on a clean EOF before a header.
pub fn read_client<R: Read>(reader: &mut R) -> Result<Option<ClientMessage>, String> {
    let Some(raw) = read_frame(reader)? else {
        return Ok(None);
    };
    let id = SurfaceId(raw.id);
    match raw.kind {
        Kind::Create => Ok(Some(ClientMessage::Create {
            id,
            width: raw.width,
            height: raw.height,
        })),
        Kind::Commit => {
            let expected = raw.width as usize * raw.height as usize * 4;
            if raw.payload.len() != expected {
                return Err(format!(
                    "commit payload is {} bytes, expected {} for {}x{}",
                    raw.payload.len(),
                    expected,
                    raw.width,
                    raw.height
                ));
            }
            let mut frame = Frame::new(raw.width, raw.height);
            frame.pixels.copy_from_slice(&raw.payload);
            Ok(Some(ClientMessage::Commit { id, frame }))
        }
        Kind::Destroy => Ok(Some(ClientMessage::Destroy { id })),
        other => Err(format!("client sent server kind {:?}", other)),
    }
}

pub fn write_client<W: Write>(writer: &mut W, message: &ClientMessage) -> Result<(), String> {
    match message {
        ClientMessage::Create { id, width, height } => {
            write_frame(writer, Kind::Create, id.0, *width, *height, &[])
        }
        ClientMessage::Commit { id, frame } => write_frame(
            writer,
            Kind::Commit,
            id.0,
            frame.width,
            frame.height,
            &frame.pixels,
        ),
        ClientMessage::Destroy { id } => write_frame(writer, Kind::Destroy, id.0, 0, 0, &[]),
    }
}

pub fn read_server<R: Read>(reader: &mut R) -> Result<Option<ServerMessage>, String> {
    let Some(raw) = read_frame(reader)? else {
        return Ok(None);
    };
    let id = SurfaceId(raw.id);
    match raw.kind {
        Kind::Created => Ok(Some(ServerMessage::Created { id })),
        Kind::Committed => Ok(Some(ServerMessage::Committed { id })),
        Kind::Destroyed => Ok(Some(ServerMessage::Destroyed { id })),
        Kind::Focused => Ok(Some(ServerMessage::Focused {
            id: if id.0 == 0 { None } else { Some(id) },
        })),
        Kind::Error => Ok(Some(ServerMessage::Error {
            reason: String::from_utf8_lossy(&raw.payload).into_owned(),
        })),
        other => Err(format!("server sent client kind {:?}", other)),
    }
}

pub fn write_server<W: Write>(writer: &mut W, message: &ServerMessage) -> Result<(), String> {
    match message {
        ServerMessage::Created { id } | ServerMessage::Committed { id } => {
            let kind = if matches!(message, ServerMessage::Created { .. }) {
                Kind::Created
            } else {
                Kind::Committed
            };
            write_frame(writer, kind, id.0, 0, 0, &[])
        }
        ServerMessage::Destroyed { id } => write_frame(writer, Kind::Destroyed, id.0, 0, 0, &[]),
        ServerMessage::Focused { id } => {
            write_frame(writer, Kind::Focused, id.map_or(0, |id| id.0), 0, 0, &[])
        }
        ServerMessage::Error { reason } => {
            write_frame(writer, Kind::Error, 0, 0, 0, reason.as_bytes())
        }
    }
}
