//! Wire mapping for the surface protocol over a Unix socket.
//!
//! The byte layout lives in `format`: a fixed 26-byte header followed by a
//! payload. This module maps that layout to `ClientMessage`/`ServerMessage`,
//! and refuses a commit whose payload is not exactly `width * height * 4`.

use super::format::{read_frame, write_frame, Kind};
use super::protocol::{ClientMessage, InputEvent, ServerMessage};
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
        Kind::Input => Ok(Some(ServerMessage::Input {
            id,
            event: decode_input(&raw.payload)?,
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
        ServerMessage::Input { id, event } => {
            write_frame(writer, Kind::Input, id.0, 0, 0, &encode_input(event))
        }
        ServerMessage::Error { reason } => {
            write_frame(writer, Kind::Error, 0, 0, 0, reason.as_bytes())
        }
    }
}

fn encode_input(event: &InputEvent) -> Vec<u8> {
    match event {
        InputEvent::Key { code, pressed } => {
            let mut out = vec![1u8];
            out.extend_from_slice(&code.to_le_bytes());
            out.push(*pressed as u8);
            out
        }
        InputEvent::Pointer { x, y, buttons } => {
            let mut out = vec![2u8];
            out.extend_from_slice(&x.to_le_bytes());
            out.extend_from_slice(&y.to_le_bytes());
            out.extend_from_slice(&buttons.to_le_bytes());
            out
        }
        InputEvent::Focus { focused } => vec![3u8, *focused as u8],
        InputEvent::Resize { width, height } => {
            let mut out = vec![4u8];
            out.extend_from_slice(&width.to_le_bytes());
            out.extend_from_slice(&height.to_le_bytes());
            out
        }
    }
}

fn decode_input(payload: &[u8]) -> Result<InputEvent, String> {
    match payload.first() {
        Some(1) if payload.len() == 6 => Ok(InputEvent::Key {
            code: u32::from_le_bytes(payload[1..5].try_into().unwrap()),
            pressed: payload[5] != 0,
        }),
        Some(2) if payload.len() == 13 => Ok(InputEvent::Pointer {
            x: i32::from_le_bytes(payload[1..5].try_into().unwrap()),
            y: i32::from_le_bytes(payload[5..9].try_into().unwrap()),
            buttons: u32::from_le_bytes(payload[9..13].try_into().unwrap()),
        }),
        Some(3) if payload.len() == 2 => Ok(InputEvent::Focus {
            focused: payload[1] != 0,
        }),
        Some(4) if payload.len() == 9 => Ok(InputEvent::Resize {
            width: u32::from_le_bytes(payload[1..5].try_into().unwrap()),
            height: u32::from_le_bytes(payload[5..9].try_into().unwrap()),
        }),
        Some(tag) => Err(format!("bad input event {} with {} bytes", tag, payload.len())),
        None => Err("empty input payload".into()),
    }
}
