//! Minimal JSON codec (dependency-free).
//!
//! IPC messages are typed, versioned and validated (SRD §4.2). A small
//! in-house JSON subset keeps the core free of arbitrary crates while the
//! browser engine remains behind a port (SRD §14 — approved allowlist only).

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsonError(pub String);

type ParseResult = Result<(JsonValue, usize), JsonError>;

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn parse_value(s: &[u8], i: usize) -> ParseResult {
    let mut i = i;
    while i < s.len() && is_ws(s[i]) {
        i += 1;
    }
    if i >= s.len() {
        return Err(JsonError("unexpected end of input".into()));
    }
    match s[i] {
        b'{' => parse_object(s, i),
        b'[' => parse_array(s, i),
        b'"' => parse_string(s, i),
        b't' => parse_literal(s, i, b"true", JsonValue::Bool(true)),
        b'f' => parse_literal(s, i, b"false", JsonValue::Bool(false)),
        b'n' => parse_literal(s, i, b"null", JsonValue::Null),
        b'-' | b'0'..=b'9' => parse_number(s, i),
        _ => Err(JsonError(format!("unexpected char: {}", s[i] as char))),
    }
}

fn parse_literal(s: &[u8], i: usize, lit: &[u8], v: JsonValue) -> ParseResult {
    if s.get(i..i + lit.len()) == Some(lit) {
        Ok((v, i + lit.len()))
    } else {
        Err(JsonError("invalid literal".into()))
    }
}

fn parse_string(s: &[u8], i: usize) -> ParseResult {
    assert_eq!(s[i], b'"');
    let mut i = i + 1;
    let mut out = String::new();
    while i < s.len() {
        let c = s[i];
        if c == b'"' {
            return Ok((JsonValue::String(out), i + 1));
        }
        if c == b'\\' {
            i += 1;
            if i >= s.len() {
                return Err(JsonError("unterminated escape".into()));
            }
            match s[i] {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'/' => out.push('/'),
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'b' => out.push('\u{0008}'),
                b'f' => out.push('\u{000c}'),
                b'u' => {
                    let hex = std::str::from_utf8(&s[i + 1..i + 5])
                        .map_err(|_| JsonError("invalid unicode escape".into()))?;
                    let cp = u32::from_str_radix(hex, 16).map_err(|_| JsonError("invalid unicode".into()))?;
                    out.push(std::char::from_u32(cp).ok_or_else(|| JsonError("invalid codepoint".into()))?);
                    i += 4;
                }
                _ => return Err(JsonError("invalid escape".into())),
            }
        } else {
            out.push(c as char);
        }
        i += 1;
    }
    Err(JsonError("unterminated string".into()))
}

fn parse_number(s: &[u8], i: usize) -> ParseResult {
    let mut i = i;
    let start = i;
    if s[i] == b'-' {
        i += 1;
    }
    while i < s.len() && (s[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() && (s[i] as char).is_ascii_digit() {
            i += 1;
        }
    }
    if i < s.len() && (s[i] == b'e' || s[i] == b'E') {
        i += 1;
        if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
            i += 1;
        }
        while i < s.len() && (s[i] as char).is_ascii_digit() {
            i += 1;
        }
    }
    let text = std::str::from_utf8(&s[start..i]).map_err(|_| JsonError("invalid number utf8".into()))?;
    let n: f64 = text.parse().map_err(|_| JsonError("invalid number".into()))?;
    Ok((JsonValue::Number(n), i))
}

fn parse_object(s: &[u8], i: usize) -> ParseResult {
    assert_eq!(s[i], b'{');
    let mut i = i + 1;
    let mut entries = Vec::new();
    while i < s.len() {
        while i < s.len() && is_ws(s[i]) {
            i += 1;
        }
        if i >= s.len() {
            return Err(JsonError("unterminated object".into()));
        }
        if s[i] == b'}' {
            return Ok((JsonValue::Object(entries), i + 1));
        }
        let (key, ni) = parse_string(s, i)?;
        i = ni;
        while i < s.len() && is_ws(s[i]) {
            i += 1;
        }
        if i >= s.len() || s[i] != b':' {
            return Err(JsonError("expected ':'".into()));
        }
        i += 1;
        let (val, ni) = parse_value(s, i)?;
        i = ni;
        entries.push((match key {
            JsonValue::String(k) => k,
            _ => return Err(JsonError("object key must be string".into())),
        }, val));
        while i < s.len() && is_ws(s[i]) {
            i += 1;
        }
        if i < s.len() && s[i] == b',' {
            i += 1;
            continue;
        }
        if i < s.len() && s[i] == b'}' {
            return Ok((JsonValue::Object(entries), i + 1));
        }
        return Err(JsonError("expected ',' or '}'".into()));
    }
    Err(JsonError("unterminated object".into()))
}

fn parse_array(s: &[u8], i: usize) -> ParseResult {
    assert_eq!(s[i], b'[');
    let mut i = i + 1;
    let mut items = Vec::new();
    while i < s.len() {
        while i < s.len() && is_ws(s[i]) {
            i += 1;
        }
        if i >= s.len() {
            return Err(JsonError("unexpected end of input".into()));
        }
        if s[i] == b']' {
            return Ok((JsonValue::Array(items), i + 1));
        }
        let (val, ni) = parse_value(s, i)?;
        i = ni;
        items.push(val);
        while i < s.len() && is_ws(s[i]) {
            i += 1;
        }
        if i < s.len() && s[i] == b',' {
            i += 1;
            continue;
        }
        if i < s.len() && s[i] == b']' {
            return Ok((JsonValue::Array(items), i + 1));
        }
        return Err(JsonError("expected ',' or ']'".into()));
    }
    Err(JsonError("unterminated array".into()))
}

pub fn parse(s: &str) -> Result<JsonValue, JsonError> {
    let (val, end) = parse_value(s.as_bytes(), 0)?;
    let _ = end;
    Ok(val)
}

pub fn stringify(v: &JsonValue) -> String {
    match v {
        JsonValue::Null => "null".into(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => {
            if n.fract() == 0.0 && n.is_finite() {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        JsonValue::String(s) => {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    '\r' => out.push_str("\\r"),
                    '\u{0008}' => out.push_str("\\b"),
                    '\u{000c}' => out.push_str("\\f"),
                    c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        }
        JsonValue::Array(items) => {
            let inner: Vec<String> = items.iter().map(stringify).collect();
            format!("[{}]", inner.join(","))
        }
        JsonValue::Object(entries) => {
            let inner: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{}:{}", stringify(&JsonValue::String(k.clone())), stringify(v)))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
    }
}

pub fn object(entries: Vec<(String, JsonValue)>) -> JsonValue {
    JsonValue::Object(entries)
}

pub fn str(v: &str) -> JsonValue {
    JsonValue::String(v.into())
}

pub fn num(n: f64) -> JsonValue {
    JsonValue::Number(n)
}

pub fn bool(b: bool) -> JsonValue {
    JsonValue::Bool(b)
}

pub fn arr(items: Vec<JsonValue>) -> JsonValue {
    JsonValue::Array(items)
}

pub fn get<'a>(obj: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    match obj {
        JsonValue::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

pub fn as_str<'a>(v: &'a JsonValue) -> Option<&'a str> {
    match v {
        JsonValue::String(s) => Some(s),
        _ => None,
    }
}

pub fn as_bool(v: &JsonValue) -> Option<bool> {
    match v {
        JsonValue::Bool(b) => Some(*b),
        _ => None,
    }
}

pub fn as_num(v: &JsonValue) -> Option<f64> {
    match v {
        JsonValue::Number(n) => Some(*n),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_object() {
        let v = object(vec![
            ("v".into(), num(1.0)),
            ("route".into(), str("host.terminal.create")),
            ("payload".into(), object(vec![])),
        ]);
        let s = stringify(&v);
        let back = parse(&s).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn parse_nested() {
        let s = r#"{"a":1,"b":"hi","c":[1,2],"d":{"e":true}}"#;
        let v = parse(s).unwrap();
        assert_eq!(get(&v, "a"), Some(&num(1.0)));
        assert_eq!(as_str(get(&v, "b").unwrap()), Some("hi"));
        assert_eq!(as_bool(get(&v, "d").and_then(|d| get(d, "e")).unwrap()), Some(true));
    }

    #[test]
    fn string_escape() {
        let s = r#"{"k":"a\"b\\c"}"#;
        let v = parse(s).unwrap();
        assert_eq!(as_str(get(&v, "k").unwrap()), Some("a\"b\\c"));
    }
}
