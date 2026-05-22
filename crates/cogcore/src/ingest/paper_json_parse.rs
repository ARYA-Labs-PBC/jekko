use std::collections::BTreeMap;

use crate::core::SourceRef;

pub(crate) fn parse_object(s: &str) -> Option<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    let s = s.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return None;
    }
    let inner = &s[1..s.len() - 1];
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        skip_ignored(bytes, &mut i, b",");
        if i >= bytes.len() {
            break;
        }
        if bytes[i] != b'"' {
            return None;
        }
        let (key, after_key) = read_string(inner, i)?;
        i = after_key;
        skip_ignored(bytes, &mut i, b":");
        if i >= bytes.len() {
            return None;
        }
        let (val, after_val) = read_value(inner, i)?;
        map.insert(key, val);
        i = after_val;
    }
    Some(map)
}

pub(crate) fn read_string(s: &str, start: usize) -> Option<(String, usize)> {
    let bytes = s.as_bytes();
    if start >= bytes.len() || bytes[start] != b'"' {
        return None;
    }
    let mut i = start + 1;
    let mut out = String::new();
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                other => out.push(other as char),
            }
            i += 2;
        } else if bytes[i] == b'"' {
            return Some((out, i + 1));
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    None
}

pub(crate) fn read_value(s: &str, start: usize) -> Option<(String, usize)> {
    let bytes = s.as_bytes();
    let mut i = start;
    skip_whitespace(bytes, &mut i);
    if i >= bytes.len() {
        return None;
    }

    match bytes[i] {
        b'"' => {
            let (s_val, end) = read_string(s, i)?;
            Some((s_val, end))
        }
        b'n' => {
            if i + 4 <= bytes.len() && &s[i..i + 4] == "null" {
                Some(("null".to_string(), i + 4))
            } else {
                None
            }
        }
        b't' => {
            if i + 4 <= bytes.len() && &s[i..i + 4] == "true" {
                Some(("true".to_string(), i + 4))
            } else {
                None
            }
        }
        b'f' => {
            if i + 5 <= bytes.len() && &s[i..i + 5] == "false" {
                Some(("false".to_string(), i + 5))
            } else {
                None
            }
        }
        b'[' | b'{' => {
            let open = bytes[i] as char;
            let close = if open == '[' { ']' } else { '}' };
            let mut depth = 1;
            let mut j = i + 1;
            let mut in_str = false;
            while j < bytes.len() && depth > 0 {
                let c = bytes[j] as char;
                if in_str {
                    if c == '\\' {
                        j += 2;
                        continue;
                    }
                    if c == '"' {
                        in_str = false;
                    }
                } else if c == '"' {
                    in_str = true;
                } else if c == open {
                    depth += 1;
                } else if c == close {
                    depth -= 1;
                }
                j += 1;
            }
            if depth != 0 {
                return None;
            }
            Some((s[i..j].to_string(), j))
        }
        _ => {
            let mut j = i;
            while j < bytes.len() {
                let c = bytes[j] as char;
                if c == ',' || c == '}' || c == ']' || c.is_whitespace() {
                    break;
                }
                j += 1;
            }
            Some((s[i..j].to_string(), j))
        }
    }
}

pub(crate) fn get_string(map: &BTreeMap<String, String>, key: &str) -> Option<String> {
    let v = map.get(key)?;
    if v == "null" {
        return None;
    }
    Some(v.clone())
}

pub(crate) fn get_string_array(map: &BTreeMap<String, String>, key: &str) -> Option<Vec<String>> {
    let raw = map.get(key)?;
    if raw == "null" {
        return None;
    }
    if !raw.starts_with('[') || !raw.ends_with(']') {
        return None;
    }
    parse_json_array_items(&raw[1..raw.len() - 1], |inner, index| {
        let bytes = inner.as_bytes();
        if index >= bytes.len() || bytes[index] != b'"' {
            return None;
        }
        read_string(inner, index)
    })
}

pub(crate) fn get_source_array(
    map: &BTreeMap<String, String>,
    key: &str,
) -> Option<Vec<SourceRef>> {
    let raw = map.get(key)?;
    if raw == "null" {
        return None;
    }
    if !raw.starts_with('[') || !raw.ends_with(']') {
        return None;
    }
    parse_json_array_items(&raw[1..raw.len() - 1], |inner, index| {
        let bytes = inner.as_bytes();
        if index >= bytes.len() || bytes[index] != b'{' {
            return None;
        }
        let (obj_str, end) = read_value(inner, index)?;
        let obj_map = parse_object(&obj_str)?;
        Some((parse_source_ref(&obj_map)?, end))
    })
}

pub(crate) fn parse_source_ref(raw: &BTreeMap<String, String>) -> Option<SourceRef> {
    Some(SourceRef {
        uri: parse_source_or_empty(raw, "uri"),
        citation: parse_source_or_empty(raw, "citation"),
        quality: raw
            .get("quality")
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(0.9),
    })
}

fn parse_source_or_empty(raw: &BTreeMap<String, String>, key: &str) -> String {
    match raw.get(key) {
        Some(value) => value.clone(),
        None => String::new(),
    }
}

pub(crate) fn parse_json_array_items<T, F>(inner: &str, mut parser: F) -> Option<Vec<T>>
where
    F: FnMut(&str, usize) -> Option<(T, usize)>,
{
    let bytes = inner.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        skip_ignored(bytes, &mut i, b",");
        if i >= bytes.len() {
            break;
        }
        let (item, end) = parser(inner, i)?;
        out.push(item);
        i = end;
    }
    Some(out)
}

fn skip_ignored(bytes: &[u8], index: &mut usize, tokens: &[u8]) {
    while *index < bytes.len() {
        let current = bytes[*index];
        if current.is_ascii_whitespace() || tokens.contains(&current) {
            *index += 1;
        } else {
            break;
        }
    }
}

fn skip_whitespace(bytes: &[u8], index: &mut usize) {
    while *index < bytes.len() && bytes[*index].is_ascii_whitespace() {
        *index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_object_and_array_items_round_trip() {
        let map = parse_object(
            r#"{"tags":["a","b"],"sources":[{"uri":"u","citation":"c","quality":0.5}]}"#,
        )
        .expect("object parses");
        assert_eq!(get_string_array(&map, "tags").unwrap(), vec!["a", "b"]);
        let sources = get_source_array(&map, "sources").unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].uri, "u");
    }
}
