use serde::de::DeserializeOwned;

use crate::error::{StoreError, StoreResult};

pub(in crate::daemon) fn serialize_opt(
    value: &Option<serde_json::Value>,
) -> StoreResult<Option<String>> {
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(StoreError::from)
}

pub(in crate::daemon) fn parse_json<T: DeserializeOwned>(
    idx: usize,
    text: &str,
) -> rusqlite::Result<T> {
    serde_json::from_str(text).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(err))
    })
}

pub(in crate::daemon) fn parse_opt_json<T: DeserializeOwned>(
    idx: usize,
    text: Option<String>,
) -> rusqlite::Result<Option<T>> {
    text.as_deref().map(|s| parse_json(idx, s)).transpose()
}

pub(in crate::daemon) fn collect_rows<T, F>(
    rows: rusqlite::MappedRows<'_, F>,
) -> StoreResult<Vec<T>>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}
