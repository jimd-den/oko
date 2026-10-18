//! Parser tests, moved out of `parser.rs` to keep that module under the 200-line
//! ceiling. Behaviour is unchanged: one test per behaviour, through public `parse`.
//! A child module of `parser`, so `use super::*` still reaches the private scanners.


use super::*;
use crate::ipc::json::num;

#[test]
fn parse_empty_object() {
    let v = parse("{}").unwrap();
    assert_eq!(v, JsonValue::Object(vec![]));
}

#[test]
fn parse_empty_array() {
    let v = parse("[]").unwrap();
    assert_eq!(v, JsonValue::Array(vec![]));
}

#[test]
fn parse_string_value() {
    let v = parse(r#" "hello" "#).unwrap();
    assert_eq!(v, JsonValue::String("hello".into()));
}

#[test]
fn parse_number_value() {
    let v = parse("42.5").unwrap();
    assert_eq!(v, JsonValue::Number(42.5));
}

#[test]
fn parse_bool_true() {
    assert_eq!(parse("true").unwrap(), JsonValue::Bool(true));
}

#[test]
fn parse_bool_false() {
    assert_eq!(parse("false").unwrap(), JsonValue::Bool(false));
}

#[test]
fn parse_null() {
    assert_eq!(parse("null").unwrap(), JsonValue::Null);
}

#[test]
fn parse_nested_object() {
    let v = parse(r#"{"a":1,"b":"hi"}"#).unwrap();
    assert_eq!(crate::ipc::json::get(&v, "a"), Some(&JsonValue::Number(1.0)));
    assert_eq!(crate::ipc::json::as_str(crate::ipc::json::get(&v, "b").unwrap()), Some("hi"));
}

#[test]
fn parse_array_of_numbers() {
    let v = parse("[1,2,3]").unwrap();
    assert_eq!(v, JsonValue::Array(vec![num(1.0), num(2.0), num(3.0)]));
}

#[test]
fn parse_rejects_bad_input() {
    assert!(parse("{").is_err());
    assert!(parse("[").is_err());
    assert!(parse("invalid").is_err());
}

#[test]
fn parse_string_with_escape() {
    let v = parse(r#" "a\"b" "#).unwrap();
    assert_eq!(v, JsonValue::String("a\"b".into()));
}
