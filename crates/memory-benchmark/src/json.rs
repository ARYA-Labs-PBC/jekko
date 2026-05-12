//! Hand-rolled JSON encoder with deterministic key ordering.
//!
//! Avoids serde so we have zero external dependencies and total control over
//! the byte-output. Encoded form is canonical: keys lexicographically sorted,
//! no extra whitespace, no trailing comma, no unicode escapes for ASCII.
//!
//! Decoder is intentionally minimal (only what `prompt_reduce` and
//! `population_report` need) — bench.rs only writes, never reads.

pub use crate::json_parser::parse;
use std::collections::BTreeMap;

/// A minimal JSON value type. BTreeMap keeps object keys sorted by Ord.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    pub fn obj() -> BTreeMap<String, Json> {
        BTreeMap::new()
    }

    /// Encode to compact, deterministic JSON.
    pub fn to_string(&self) -> String {
        let mut buf = String::with_capacity(256);
        self.write_to(&mut buf);
        buf
    }

    pub fn write_to(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => out.push_str(&n.to_string()),
            Json::Float(f) => {
                if !f.is_finite() {
                    // Canonical: non-finite floats become null. Determinism.
                    out.push_str("null");
                } else if f.fract() == 0.0 && f.abs() < 1e16 {
                    // Render as integer-shaped if exact.
                    out.push_str(&format!("{}.0", *f as i64));
                } else {
                    // Use Rust's shortest round-trip repr.
                    out.push_str(&format!("{}", f));
                }
            }
            Json::Str(s) => encode_str(s, out),
            Json::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write_to(out);
                }
                out.push(']');
            }
            Json::Object(map) => {
                out.push('{');
                let mut first = true;
                for (k, v) in map.iter() {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    encode_str(k, out);
                    out.push(':');
                    v.write_to(out);
                }
                out.push('}');
            }
        }
    }
}

fn encode_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

// ───────── helpers for builder ergonomics ─────────

pub fn s<T: Into<String>>(v: T) -> Json {
    Json::Str(v.into())
}
pub fn n(v: i64) -> Json {
    Json::Int(v)
}
pub fn f(v: f64) -> Json {
    Json::Float(v)
}
pub fn b(v: bool) -> Json {
    Json::Bool(v)
}
pub fn arr(v: Vec<Json>) -> Json {
    Json::Array(v)
}
pub fn arr_str<T: Into<String>>(v: impl IntoIterator<Item = T>) -> Json {
    Json::Array(v.into_iter().map(|x| Json::Str(x.into())).collect())
}
pub fn obj(pairs: &[(&str, Json)]) -> Json {
    let mut m = BTreeMap::new();
    for (k, v) in pairs {
        m.insert((*k).to_string(), v.clone());
    }
    Json::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_roundtrip() {
        assert_eq!(Json::Null.to_string(), "null");
        assert_eq!(Json::Bool(true).to_string(), "true");
        assert_eq!(Json::Int(42).to_string(), "42");
        assert_eq!(Json::Float(1.5).to_string(), "1.5");
        assert_eq!(s("hi").to_string(), "\"hi\"");
    }

    #[test]
    fn objects_have_sorted_keys() {
        let o = obj(&[("z", n(1)), ("a", n(2)), ("m", n(3))]);
        let encoded = o.to_string();
        assert_eq!(encoded, "{\"a\":2,\"m\":3,\"z\":1}");
    }

    #[test]
    fn string_escapes() {
        assert_eq!(s("a\"b").to_string(), "\"a\\\"b\"");
        assert_eq!(s("a\nb").to_string(), "\"a\\nb\"");
    }

    #[test]
    fn parse_then_encode_is_canonical() {
        let input = r#"{"z":1, "a":2}"#;
        let v = parse(input).unwrap();
        assert_eq!(v.to_string(), "{\"a\":2,\"z\":1}");
    }

    #[test]
    fn float_canonical_form_for_whole_numbers() {
        // 5.0 must encode as "5.0", not "5" — preserves float type round-trip.
        assert_eq!(Json::Float(5.0).to_string(), "5.0");
    }
}
