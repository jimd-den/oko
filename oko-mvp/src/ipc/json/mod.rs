//! JSON value type and helpers.
//! Small, composable, dependency-free JSON for IPC (SRD §4.2).

pub mod parser;
pub mod stringifier;

pub use parser::parse;
pub use stringifier::stringify;

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

pub fn as_str(v: &JsonValue) -> Option<&str> {
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
    fn object_and_get() {
        let v = object(vec![("key".into(), num(1.0))]);
        assert_eq!(get(&v, "key"), Some(&num(1.0)));
        assert_eq!(get(&v, "missing"), None);
    }

    #[test]
    fn as_str_works() {
        let v = str("hello");
        assert_eq!(as_str(&v), Some("hello"));
        assert_eq!(as_str(&num(1.0)), None);
    }

    #[test]
    fn as_bool_works() {
        assert_eq!(as_bool(&bool(true)), Some(true));
        assert_eq!(as_bool(&num(1.0)), None);
    }

    #[test]
    fn as_num_works() {
        assert_eq!(as_num(&num(42.0)), Some(42.0));
        assert_eq!(as_num(&str("x")), None);
    }
}
