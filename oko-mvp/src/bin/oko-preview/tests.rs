//! Argument parsing for oko-preview.

use super::*;
use super::*;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn defaults_to_80_by_24() {
    let options = parse_args(&args(&["https://example.com"])).unwrap();
    assert_eq!(options.url, "https://example.com");
    assert_eq!((options.cols, options.rows), (80, 24));
    assert!(options.socket.is_none());
}

#[test]
fn a_socket_and_dimensions_are_parsed() {
    let options = parse_args(&args(&[
        "https://example.com",
        "40",
        "12",
        "--socket",
        "/tmp/x.sock",
    ]))
    .unwrap();
    assert_eq!((options.cols, options.rows), (40, 12));
    assert_eq!(options.socket, Some(PathBuf::from("/tmp/x.sock")));
}

#[test]
fn a_missing_url_is_refused() {
    assert!(parse_args(&args(&[])).is_err());
}

#[test]
fn a_zero_dimension_is_refused() {
    assert!(parse_args(&args(&["x", "0"])).is_err());
}
