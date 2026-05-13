use crate::json::Json;
use std::collections::BTreeMap;

pub(crate) fn as_object(value: &Json) -> Result<&BTreeMap<String, Json>, String> {
    match value {
        Json::Object(obj) => Ok(obj),
        _ => Err("expected object".to_string()),
    }
}

pub(crate) fn as_object_ok(value: &Json) -> Option<&BTreeMap<String, Json>> {
    match value {
        Json::Object(obj) => Some(obj),
        _ => None,
    }
}

pub(crate) fn required<'a>(obj: &'a BTreeMap<String, Json>, key: &str) -> Result<&'a Json, String> {
    match obj.get(key) {
        Some(value) => Ok(value),
        None => Err(format!("missing {key}")),
    }
}

pub(crate) fn required_string(obj: &BTreeMap<String, Json>, key: &str) -> Result<String, String> {
    let value = required(obj, key)?;
    match as_str(value) {
        Some(s) => Ok(s.to_string()),
        None => Err(format!("{key} must be a string")),
    }
}

pub(crate) fn optional_string(obj: &BTreeMap<String, Json>, key: &str) -> Option<String> {
    obj.get(key).and_then(as_str).map(str::to_string)
}

pub(crate) fn required_f64(obj: &BTreeMap<String, Json>, key: &str) -> Result<f64, String> {
    match required(obj, key)? {
        Json::Float(value) => Ok(*value),
        Json::Int(value) => Ok(*value as f64),
        _ => Err(format!("{key} must be a number")),
    }
}

pub(crate) fn optional_i64(obj: &BTreeMap<String, Json>, key: &str) -> Option<i64> {
    match obj.get(key) {
        Some(Json::Int(value)) => Some(*value),
        Some(Json::Float(value)) => Some(*value as i64),
        _ => None,
    }
}

pub(crate) fn optional_f32(obj: &BTreeMap<String, Json>, key: &str) -> Option<f32> {
    match obj.get(key) {
        Some(Json::Int(value)) => Some(*value as f32),
        Some(Json::Float(value)) => Some(*value as f32),
        _ => None,
    }
}

pub(crate) fn required_array<'a>(
    obj: &'a BTreeMap<String, Json>,
    key: &str,
) -> Result<&'a [Json], String> {
    required_array_value(required(obj, key)?, key)
}

pub(crate) fn required_array_value<'a>(value: &'a Json, key: &str) -> Result<&'a [Json], String> {
    match value {
        Json::Array(items) => Ok(items),
        _ => Err(format!("{key} must be an array")),
    }
}

pub(crate) fn optional_string_array(obj: &BTreeMap<String, Json>, key: &str) -> Vec<String> {
    match obj
        .get(key)
        .and_then(|value| required_array_value(value, key).ok())
    {
        Some(items) => items
            .iter()
            .filter_map(as_str)
            .map(str::to_string)
            .collect(),
        None => Vec::new(),
    }
}

pub(crate) fn as_str(value: &Json) -> Option<&str> {
    match value {
        Json::Str(value) => Some(value.as_str()),
        _ => None,
    }
}

pub(crate) fn as_bool(value: &Json) -> Option<bool> {
    match value {
        Json::Bool(value) => Some(*value),
        _ => None,
    }
}
