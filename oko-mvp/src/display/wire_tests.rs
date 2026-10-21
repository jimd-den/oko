//! Wire format tests: round trips, truncation and bad headers.

use super::protocol::{ClientMessage, ServerMessage};
use super::surface::{Frame, SurfaceId};
use super::wire::{read_client, read_server, write_client, write_server};
use std::io::Cursor;

fn round_trip_client(message: ClientMessage) -> ClientMessage {
    let mut bytes = Vec::new();
    write_client(&mut bytes, &message).unwrap();
    read_client(&mut Cursor::new(bytes)).unwrap().unwrap()
}

#[test]
fn a_create_round_trips() {
    let message = ClientMessage::Create {
        id: SurfaceId(7),
        width: 2,
        height: 3,
    };
    assert_eq!(round_trip_client(message.clone()), message);
}

#[test]
fn a_commit_carries_exactly_width_times_height_times_four() {
    let mut frame = Frame::new(2, 1);
    frame.set_pixel(0, 0, [255, 0, 0, 255]);
    frame.set_pixel(1, 0, [0, 0, 255, 255]);
    let message = ClientMessage::Commit {
        id: SurfaceId(1),
        frame,
    };
    assert_eq!(round_trip_client(message.clone()), message);
}

#[test]
fn a_truncated_commit_is_refused() {
    let mut frame = Frame::new(2, 1);
    frame.set_pixel(0, 0, [1, 2, 3, 255]);
    let mut bytes = Vec::new();
    write_client(
        &mut bytes,
        &ClientMessage::Commit {
            id: SurfaceId(1),
            frame,
        },
    )
    .unwrap();
    bytes.pop();
    assert!(read_client(&mut Cursor::new(bytes)).is_err());
}

#[test]
fn a_wrong_magic_is_refused() {
    let mut bytes = Vec::new();
    write_client(
        &mut bytes,
        &ClientMessage::Destroy {
            id: SurfaceId(1),
        },
    )
    .unwrap();
    bytes[0] = b'X';
    assert!(read_client(&mut Cursor::new(bytes)).is_err());
}

#[test]
fn an_unknown_kind_is_refused() {
    let mut bytes = Vec::new();
    write_client(
        &mut bytes,
        &ClientMessage::Destroy {
            id: SurfaceId(1),
        },
    )
    .unwrap();
    bytes[5] = 99;
    assert!(read_client(&mut Cursor::new(bytes)).is_err());
}

#[test]
fn a_key_input_round_trips() {
    let message = ServerMessage::Input {
        id: SurfaceId(3),
        event: super::protocol::InputEvent::Key {
            code: 0x41,
            pressed: true,
        },
    };
    let mut bytes = Vec::new();
    write_server(&mut bytes, &message).unwrap();
    assert_eq!(
        read_server(&mut Cursor::new(bytes)).unwrap(),
        Some(message)
    );
}

#[test]
fn an_unknown_input_tag_is_refused() {
    let mut header = [0u8; 26];
    header[0..4].copy_from_slice(b"OKOD");
    header[4] = 1;
    header[5] = 9;
    header[6..14].copy_from_slice(&1u64.to_le_bytes());
    header[22..26].copy_from_slice(&2u32.to_le_bytes());
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(&[99, 0]);
    assert!(read_server(&mut Cursor::new(bytes)).is_err());
}

#[test]
fn a_server_error_carries_its_reason() {
    let message = ServerMessage::Error {
        reason: "unknown surface 4".into(),
    };
    let mut bytes = Vec::new();
    write_server(&mut bytes, &message).unwrap();
    assert_eq!(
        read_server(&mut Cursor::new(bytes)).unwrap(),
        Some(message)
    );
}

#[test]
fn a_clean_eof_is_none() {
    assert_eq!(read_client(&mut Cursor::new(Vec::new())).unwrap(), None);
}

#[test]
fn an_oversized_payload_is_refused_before_allocation() {
    let mut header = [0u8; 26];
    header[0..4].copy_from_slice(b"OKOD");
    header[4] = 1;
    header[5] = 2;
    header[22..26].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(read_client(&mut Cursor::new(header.to_vec())).is_err());
}
