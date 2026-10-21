//! Key parser tests: bytes in, engine-neutral key events out.

use super::input::*;
use super::protocol::InputEvent;

fn key(code: u32) -> InputEvent {
    InputEvent::Key {
        code,
        pressed: true,
    }
}

#[test]
fn a_plain_ascii_byte_is_a_key() {
    let mut parser = KeyParser::default();
    assert_eq!(parser.feed(b'a'), vec![key(b'a' as u32)]);
}

#[test]
fn a_utf8_character_is_one_key() {
    let mut parser = KeyParser::default();
    let events = parser.feed_bytes("é".as_bytes());
    assert_eq!(events, vec![key('é' as u32)]);
}

#[test]
fn enter_tab_and_backspace_keep_their_control_codes() {
    let mut parser = KeyParser::default();
    assert_eq!(parser.feed(0x0d), vec![key(KEY_ENTER)]);
    assert_eq!(parser.feed(0x09), vec![key(KEY_TAB)]);
    assert_eq!(parser.feed(0x7f), vec![key(KEY_BACKSPACE)]);
}

#[test]
fn control_c_is_a_key_not_a_signal() {
    let mut parser = KeyParser::default();
    assert_eq!(parser.feed(0x03), vec![key(0x03)]);
}

#[test]
fn an_arrow_is_one_key() {
    let mut parser = KeyParser::default();
    assert_eq!(parser.feed_bytes(b"\x1b[A"), vec![key(KEY_UP)]);
}

#[test]
fn an_escape_sequence_split_across_feeds_waits_for_the_rest() {
    let mut parser = KeyParser::default();
    assert_eq!(parser.feed(0x1b), vec![]);
    assert_eq!(parser.feed(b'['), vec![]);
    assert_eq!(parser.feed(b'A'), vec![key(KEY_UP)]);
}

#[test]
fn escape_then_a_letter_is_two_keys() {
    let mut parser = KeyParser::default();
    assert_eq!(parser.feed(0x1b), vec![]);
    assert_eq!(parser.feed(b'x'), vec![key(KEY_ESC), key(b'x' as u32)]);
}

#[test]
fn an_unknown_csi_sequence_is_ignored() {
    let mut parser = KeyParser::default();
    assert_eq!(parser.feed_bytes(b"\x1b[99~"), vec![]);
}
