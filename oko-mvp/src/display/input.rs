//! Key bytes to `InputEvent`s, with no device knowledge.
//!
//! A terminal delivers bytes, not key names. This parser turns a byte stream
//! into key presses and keeps an incomplete escape sequence until the rest
//! arrives. It does not read a device, set a mode, or know which surface has
//! focus; those are the input source and the session.
//!
//! Codes are the protocol's engine-neutral numbers. Printable characters use
//! their Unicode scalar value; control keys use their ASCII value; the keys a
//! terminal encodes with CSI use the reserved 0x100 range.

use super::protocol::InputEvent;

pub const KEY_ESC: u32 = 0x1b;
pub const KEY_ENTER: u32 = 0x0d;
pub const KEY_TAB: u32 = 0x09;
pub const KEY_BACKSPACE: u32 = 0x7f;
pub const KEY_UP: u32 = 0x100;
pub const KEY_DOWN: u32 = 0x101;
pub const KEY_RIGHT: u32 = 0x102;
pub const KEY_LEFT: u32 = 0x103;
pub const KEY_HOME: u32 = 0x104;
pub const KEY_END: u32 = 0x105;
pub const KEY_DELETE: u32 = 0x106;
pub const KEY_PAGE_UP: u32 = 0x107;
pub const KEY_PAGE_DOWN: u32 = 0x108;

#[derive(Default)]
pub struct KeyParser {
    pending: Vec<u8>,
}

impl KeyParser {
    pub fn feed(&mut self, byte: u8) -> Vec<InputEvent> {
        self.pending.push(byte);
        self.drain()
    }

    pub fn feed_bytes(&mut self, bytes: &[u8]) -> Vec<InputEvent> {
        self.pending.extend_from_slice(bytes);
        self.drain()
    }

    fn drain(&mut self) -> Vec<InputEvent> {
        let mut events = Vec::new();
        loop {
            if self.pending.is_empty() {
                break;
            }
            if self.pending[0] == 0x1b {
                if self.pending.len() == 1 {
                    break; // Could be Escape or the start of a CSI sequence.
                }
                if self.pending[1] == b'[' {
                    let Some(end) = self.pending[2..]
                        .iter()
                        .position(|b| (0x40..=0x7e).contains(b))
                    else {
                        break; // Incomplete sequence; wait for the final byte.
                    };
                    let sequence: Vec<u8> = self.pending.drain(..2 + end + 1).collect();
                    if let Some(code) = csi_key(&sequence[2..]) {
                        events.push(key(code));
                    }
                } else {
                    self.pending.remove(0);
                    events.push(key(KEY_ESC));
                }
            } else if self.pending[0] < 0x80 {
                let byte = self.pending.remove(0);
                if let Some(code) = ascii_key(byte) {
                    events.push(key(code));
                }
            } else {
                let length = utf8_len(self.pending[0]);
                if self.pending.len() < length {
                    break;
                }
                let bytes: Vec<u8> = self.pending.drain(..length).collect();
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    if let Some(character) = text.chars().next() {
                        events.push(key(character as u32));
                    }
                }
            }
        }
        events
    }
}

fn key(code: u32) -> InputEvent {
    InputEvent::Key {
        code,
        pressed: true,
    }
}

fn ascii_key(byte: u8) -> Option<u32> {
    match byte {
        0x0d | 0x0a => Some(KEY_ENTER),
        0x09 => Some(KEY_TAB),
        0x08 | 0x7f => Some(KEY_BACKSPACE),
        0x01..=0x1a | 0x20..=0x7e => Some(byte as u32),
        _ => None,
    }
}

fn utf8_len(lead: u8) -> usize {
    match lead {
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => 1,
    }
}

fn csi_key(sequence: &[u8]) -> Option<u32> {
    match sequence {
        b"A" => Some(KEY_UP),
        b"B" => Some(KEY_DOWN),
        b"C" => Some(KEY_RIGHT),
        b"D" => Some(KEY_LEFT),
        b"H" => Some(KEY_HOME),
        b"F" => Some(KEY_END),
        b"3~" => Some(KEY_DELETE),
        b"5~" => Some(KEY_PAGE_UP),
        b"6~" => Some(KEY_PAGE_DOWN),
        _ => None,
    }
}
