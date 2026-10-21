//! The low-level surface frame layout: header, kinds and raw payload.
//!
//! Kept apart from the message mapping in `wire` so the byte layout has one
//! home. `wire` decides what a Create/Commit/Destroy means; this module only
//! moves bounded frames.

use std::io::{Read, Write};

const MAGIC: [u8; 4] = *b"OKOD";
const VERSION: u8 = 1;
const HEADER: usize = 26;
const MAX_PAYLOAD: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Create = 1,
    Commit = 2,
    Destroy = 3,
    Created = 4,
    Committed = 5,
    Destroyed = 6,
    Focused = 7,
    Error = 8,
}

impl Kind {
    fn from_u8(value: u8) -> Result<Self, String> {
        match value {
            1 => Ok(Self::Create),
            2 => Ok(Self::Commit),
            3 => Ok(Self::Destroy),
            4 => Ok(Self::Created),
            5 => Ok(Self::Committed),
            6 => Ok(Self::Destroyed),
            7 => Ok(Self::Focused),
            8 => Ok(Self::Error),
            other => Err(format!("unknown frame kind {}", other)),
        }
    }
}

pub(super) fn write_frame<W: Write>(
    writer: &mut W,
    kind: Kind,
    id: u64,
    width: u32,
    height: u32,
    payload: &[u8],
) -> Result<(), String> {
    if payload.len() > MAX_PAYLOAD {
        return Err(format!("payload {} exceeds {} bytes", payload.len(), MAX_PAYLOAD));
    }
    let mut header = [0u8; HEADER];
    header[0..4].copy_from_slice(&MAGIC);
    header[4] = VERSION;
    header[5] = kind as u8;
    header[6..14].copy_from_slice(&id.to_le_bytes());
    header[14..18].copy_from_slice(&width.to_le_bytes());
    header[18..22].copy_from_slice(&height.to_le_bytes());
    header[22..26].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    writer.write_all(&header).map_err(|e| e.to_string())?;
    writer.write_all(payload).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}

pub(super) struct RawFrame {
    pub(super) kind: Kind,
    pub(super) id: u64,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) payload: Vec<u8>,
}

pub(super) fn read_frame<R: Read>(reader: &mut R) -> Result<Option<RawFrame>, String> {
    let mut header = [0u8; HEADER];
    match reader.read(&mut header[0..1]) {
        Ok(0) => return Ok(None),
        Ok(_) => {}
        Err(e) => return Err(e.to_string()),
    }
    reader
        .read_exact(&mut header[1..])
        .map_err(|e| e.to_string())?;
    if header[0..4] != MAGIC {
        return Err("bad magic".into());
    }
    if header[4] != VERSION {
        return Err(format!("unsupported protocol version {}", header[4]));
    }
    let kind = Kind::from_u8(header[5])?;
    let id = u64::from_le_bytes(header[6..14].try_into().unwrap());
    let width = u32::from_le_bytes(header[14..18].try_into().unwrap());
    let height = u32::from_le_bytes(header[18..22].try_into().unwrap());
    let length = u32::from_le_bytes(header[22..26].try_into().unwrap()) as usize;
    if length > MAX_PAYLOAD {
        return Err(format!("payload {} exceeds {} bytes", length, MAX_PAYLOAD));
    }
    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload).map_err(|e| e.to_string())?;
    Ok(Some(RawFrame {
        kind,
        id,
        width,
        height,
        payload,
    }))
}
