//! Envelope round-trips: what the host writes must be what it can read back.

use super::*;

#[test]
fn request_roundtrip() {
    let m = Message::request("1", "host.terminal.create", object(vec![]));
    let line = m.to_line();
    let back = Message::from_line(&line).unwrap();
    assert_eq!(back.version, 1);
    assert_eq!(back.id, "1");
    assert_eq!(back.route, "host.terminal.create");
}

#[test]
fn error_response_roundtrip() {
    let m = Message::error_response(
        "2",
        "host.app.launch",
        "permission_denied",
        "missing process.spawn",
    );
    let line = m.to_line();
    let back = Message::from_line(&line).unwrap();
    assert!(back.error.is_some());
    assert_eq!(back.route, "host.app.launch");
}

#[test]
fn rejects_wrong_version() {
    let mut m = Message::request("1", "host.status", object(vec![]));
    m.version = 999;
    let line = m.to_line();
    assert!(Message::from_line(&line).is_err());
}

#[test]
fn notification() {
    let m = Message::notification("ui.reload", object(vec![]));
    assert!(matches!(m.direction, Direction::Notification));
    let back = Message::from_line(&m.to_line()).unwrap();
    assert_eq!(back.route, "ui.reload");
}
