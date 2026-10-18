//! JSON parser — small composable functions.
//! Each function does one thing and is under 20 lines.

use crate::ipc::json::{JsonError, JsonValue};

type ParseResult = Result<(JsonValue, usize), JsonError>;

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn skip_ws(s: &[u8], i: usize) -> usize {
    let mut i = i;
    while i < s.len() && is_ws(s[i]) {
        i += 1;
    }
    i
}

pub fn parse_value(s: &[u8], i: usize) -> ParseResult {
    let i = skip_ws(s, i);
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
    if s[i] != b'"' {
        return Err(JsonError("expected string".into()));
    }
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
            out.push(match s[i] {
                b'"' => '"',
                b'\\' => '\\',
                b'/' => '/',
                b'n' => '\n',
                b't' => '\t',
                b'r' => '\r',
                b'b' => '\u{0008}',
                b'f' => '\u{000c}',
                b'u' => {
                    let hex = std::str::from_utf8(&s[i + 1..i + 5])
                        .map_err(|_| JsonError("invalid unicode escape".into()))?;
                    let cp = u32::from_str_radix(hex, 16)
                        .map_err(|_| JsonError("invalid unicode".into()))?;
                    std::char::from_u32(cp).ok_or_else(|| JsonError("invalid codepoint".into()))?
                }
                _ => return Err(JsonError("invalid escape".into())),
            });
            i += if s[i] == b'u' { 4 } else { 0 };
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
    let text =
        std::str::from_utf8(&s[start..i]).map_err(|_| JsonError("invalid number utf8".into()))?;
    let n: f64 = text
        .parse()
        .map_err(|_| JsonError("invalid number".into()))?;
    Ok((JsonValue::Number(n), i))
}

fn parse_object(s: &[u8], mut i: usize) -> ParseResult {
    if s[i] != b'{' {
        return Err(JsonError("expected object".into()));
    }
    i += 1;
    let mut entries = Vec::new();
    loop {
        i = skip_ws(s, i);
        if i >= s.len() {
            return Err(JsonError("unterminated object".into()));
        }
        if s[i] == b'}' {
            return Ok((JsonValue::Object(entries), i + 1));
        }
        let (key, ni) = parse_string(s, i)?;
        i = skip_ws(s, ni);
        if i >= s.len() || s[i] != b':' {
            return Err(JsonError("expected ':'".into()));
        }
        let (val, ni) = parse_value(s, i + 1)?;
        i = ni;
        entries.push((
            match key {
                JsonValue::String(k) => k,
                _ => return Err(JsonError("object key must be string".into())),
            },
            val,
        ));
        i = skip_ws(s, i);
        if i < s.len() && s[i] == b',' {
            i += 1;
            continue;
        }
        if i < s.len() && s[i] == b'}' {
            return Ok((JsonValue::Object(entries), i + 1));
        }
        return Err(JsonError("expected ',' or '}'".into()));
    }
}

fn parse_array(s: &[u8], mut i: usize) -> ParseResult {
    if s[i] != b'[' {
        return Err(JsonError("expected array".into()));
    }
    i += 1;
    let mut items = Vec::new();
    loop {
        i = skip_ws(s, i);
        if i >= s.len() {
            return Err(JsonError("unterminated array".into()));
        }
        if s[i] == b']' {
            return Ok((JsonValue::Array(items), i + 1));
        }
        let (val, ni) = parse_value(s, i)?;
        i = ni;
        items.push(val);
        i = skip_ws(s, i);
        if i < s.len() && s[i] == b',' {
            i += 1;
            continue;
        }
        if i < s.len() && s[i] == b']' {
            return Ok((JsonValue::Array(items), i + 1));
        }
        return Err(JsonError("expected ',' or ']'".into()));
    }
}

pub fn parse(s: &str) -> Result<JsonValue, JsonError> {
    let (val, _end) = parse_value(s.as_bytes(), 0)?;
    Ok(val)
}

#[cfg(test)]
mod tests;
