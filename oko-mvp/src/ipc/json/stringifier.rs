//! JSON stringifier — small composable functions.

use crate::ipc::json::JsonValue;

pub fn stringify(v: &JsonValue) -> String {
    match v {
        JsonValue::Null => "null".into(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => format_number(*n),
        JsonValue::String(s) => format_string(s),
        JsonValue::Array(items) => format_array(items),
        JsonValue::Object(entries) => format_object(entries),
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

fn format_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        let escaped = match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\t' => "\\t".to_string(),
            '\r' => "\\r".to_string(),
            '\u{0008}' => "\\b".to_string(),
            '\u{000c}' => "\\f".to_string(),
            c if (c as u32) < 0x20 => format!("\\u{:04x}", c as u32),
            c => c.to_string(),
        };
        out.push_str(&escaped);
    }
    out.push('"');
    out
}

fn format_array(items: &[JsonValue]) -> String {
    let inner: Vec<String> = items.iter().map(stringify).collect();
    format!("[{}]", inner.join(","))
}

fn format_object(entries: &[(String, JsonValue)]) -> String {
    let inner: Vec<String> = entries
        .iter()
        .map(|(k, v)| format!("{}:{}", format_string(k), stringify(v)))
        .collect();
    format!("{{{}}}", inner.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::json::{object, str, num, bool, arr, get, as_str};

    #[test]
    fn stringify_null() {
        assert_eq!(stringify(&JsonValue::Null), "null");
    }

    #[test]
    fn stringify_bool() {
        assert_eq!(stringify(&bool(true)), "true");
        assert_eq!(stringify(&bool(false)), "false");
    }

    #[test]
    fn stringify_number() {
        assert_eq!(stringify(&num(42.0)), "42");
        assert_eq!(stringify(&num(3.14)), "3.14");
    }

    #[test]
    fn stringify_string() {
        assert_eq!(stringify(&str("hello")), "\"hello\"");
    }

    #[test]
    fn stringify_array() {
        assert_eq!(stringify(&arr(vec![num(1.0), num(2.0)])), "[1,2]");
    }

    #[test]
    fn stringify_object() {
        assert_eq!(stringify(&object(vec![("a".into(), num(1.0))])), "{\"a\":1}");
    }

    #[test]
    fn roundtrip() {
        let v = object(vec![
            ("v".into(), num(1.0)),
            ("route".into(), str("host.terminal.create")),
            ("payload".into(), object(vec![])),
        ]);
        let s = stringify(&v);
        let back = crate::ipc::json::parser::parse(&s).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn stringify_nested() {
        let v = object(vec![
            ("a".into(), num(1.0)),
            ("b".into(), str("hi")),
            ("c".into(), arr(vec![num(1.0), num(2.0)])),
            ("d".into(), object(vec![("e".into(), bool(true))])),
        ]);
        let s = stringify(&v);
        let back = crate::ipc::json::parser::parse(&s).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn stringify_escape() {
        let v = object(vec![("k".into(), str("a\"b\\c"))]);
        let s = stringify(&v);
        let back = crate::ipc::json::parser::parse(&s).unwrap();
        assert_eq!(as_str(get(&back, "k").unwrap()), Some("a\"b\\c"));
    }
}
