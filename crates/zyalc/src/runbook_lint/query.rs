use serde_yaml::Value;

pub(super) fn recursive_numeric_keys(value: &Value) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    collect_numeric_keys(value, &mut out);
    out
}

fn collect_numeric_keys(value: &Value, out: &mut Vec<(String, usize)>) {
    match value {
        Value::Mapping(map) => {
            for (key, value) in map {
                if let Some(key) = key.as_str() {
                    if let Some(number) = yaml_usize(value) {
                        out.push((key.to_ascii_lowercase(), number));
                    }
                }
                collect_numeric_keys(value, out);
            }
        }
        Value::Sequence(items) => {
            for item in items {
                collect_numeric_keys(item, out);
            }
        }
        _ => {}
    }
}

pub(super) fn recursive_key_exists(value: &Value, needle: &str) -> bool {
    match value {
        Value::Mapping(map) => map.iter().any(|(key, value)| {
            key.as_str()
                .map(|key| key.eq_ignore_ascii_case(needle))
                .unwrap_or(false)
                || recursive_key_exists(value, needle)
        }),
        Value::Sequence(items) => items.iter().any(|item| recursive_key_exists(item, needle)),
        _ => false,
    }
}

pub(super) fn recursive_bool_key(value: &Value, needles: &[&str], expected: bool) -> bool {
    match value {
        Value::Mapping(map) => map.iter().any(|(key, value)| {
            let key_matches = key
                .as_str()
                .map(|key| {
                    needles
                        .iter()
                        .any(|needle| key.eq_ignore_ascii_case(needle))
                })
                .unwrap_or(false);
            (key_matches && value.as_bool() == Some(expected))
                || recursive_bool_key(value, needles, expected)
        }),
        Value::Sequence(items) => items
            .iter()
            .any(|item| recursive_bool_key(item, needles, expected)),
        _ => false,
    }
}

pub(super) fn recursive_values_for_key<'a>(value: &'a Value, needle: &str) -> Vec<&'a Value> {
    let mut out = Vec::new();
    collect_values_for_key(value, needle, &mut out);
    out
}

fn collect_values_for_key<'a>(value: &'a Value, needle: &str, out: &mut Vec<&'a Value>) {
    match value {
        Value::Mapping(map) => {
            for (key, value) in map {
                if key
                    .as_str()
                    .map(|key| key.eq_ignore_ascii_case(needle))
                    .unwrap_or(false)
                {
                    out.push(value);
                }
                collect_values_for_key(value, needle, out);
            }
        }
        Value::Sequence(items) => {
            for item in items {
                collect_values_for_key(item, needle, out);
            }
        }
        _ => {}
    }
}

pub(super) fn recursive_string_contains(value: &Value, needle: &str) -> bool {
    let needle = needle.to_ascii_lowercase();
    match value {
        Value::String(text) => text.to_ascii_lowercase().contains(&needle),
        Value::Mapping(map) => map.iter().any(|(key, value)| {
            recursive_string_contains(key, &needle) || recursive_string_contains(value, &needle)
        }),
        Value::Sequence(items) => items
            .iter()
            .any(|item| recursive_string_contains(item, &needle)),
        _ => false,
    }
}

pub(super) fn yaml_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        let Value::Mapping(map) = current else {
            return None;
        };
        current = map.iter().find_map(|(key, value)| match key.as_str() {
            Some(key) if key == *segment => Some(value),
            _ => None,
        })?;
    }
    Some(current)
}

pub(super) fn yaml_sequence_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Vec<Value>> {
    yaml_path(value, path).and_then(Value::as_sequence)
}

fn yaml_usize(value: &Value) -> Option<usize> {
    match value {
        Value::Number(number) => number.as_u64().map(|value| value as usize),
        Value::String(text) => text.parse::<usize>().ok(),
        _ => None,
    }
}

pub(super) fn yaml_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.parse::<f64>().ok(),
        _ => None,
    }
}
