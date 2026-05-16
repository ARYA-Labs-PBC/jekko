//! `xtask schema` — JSON Schema emitter for `jekko-core` top-level types.
//!
//! Until `schemars` is a dependency of `jekko-core` we can't derive the
//! schema automatically. This module hand-writes a minimal JSON Schema
//! for each top-level type that downstream consumers (TS client codegen,
//! editor IntelliSense, `jekko.json` validators) need.
//!
//! NOTE(schemars): once `jekko-core` opts into `schemars = "0.8"`,
//! replace each of these blobs with `schemars::schema_for!(Type)` and
//! drop the hand-written fallbacks.
//!
//! Output layout (relative to the repo root):
//!
//! ```text
//! crates/xtask/generated-schemas/
//!   config.json
//!   theme.json
//!   keybinds-table.json
//!   provider-info.json
//!   session-info.json
//!   permission-config.json
//!   project-id.json
//! ```
//!
//! Each file starts with a leading `_note` field that pins the schemars
//! migration. The rest of the body is a JSON Schema Draft 2020-12-style
//! document with `type`, `properties`, and `required` populated for the
//! known fields.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Value};

/// Compute the absolute path of the schema output dir relative to the
/// xtask crate root.
fn output_dir(root: &Path) -> PathBuf {
    root.join("crates/xtask/generated-schemas")
}

/// One emitted schema: file name (relative to the output dir) plus the
/// JSON document itself.
struct SchemaEntry {
    file: &'static str,
    schema: Value,
}

/// Build the full list of schema entries we currently emit. Each entry
/// is hand-written and tagged with a migration note pointing at the eventual
/// `schemars` migration.
fn entries() -> Vec<SchemaEntry> {
    vec![
        SchemaEntry {
            file: "config.json",
            schema: schema_config(),
        },
        SchemaEntry {
            file: "theme.json",
            schema: schema_theme(),
        },
        SchemaEntry {
            file: "keybinds-table.json",
            schema: schema_keybinds_table(),
        },
        SchemaEntry {
            file: "provider-info.json",
            schema: schema_provider_info(),
        },
        SchemaEntry {
            file: "session-info.json",
            schema: schema_session_info(),
        },
        SchemaEntry {
            file: "permission-config.json",
            schema: schema_permission_config(),
        },
        SchemaEntry {
            file: "project-id.json",
            schema: schema_project_id(),
        },
    ]
}

/// Run the schema emit step. Writes one file per [`SchemaEntry`]; logs
/// each path; returns the number of files written.
pub fn emit(root: &Path) -> Result<usize> {
    let dir = output_dir(root);
    fs::create_dir_all(&dir)
        .with_context(|| format!("create schema output dir {}", dir.display()))?;

    let entries = entries();
    let count = entries.len();
    for entry in entries {
        let path = dir.join(entry.file);
        let body =
            serde_json::to_string_pretty(&entry.schema).context("serialize schema document")?;
        let mut text = body;
        text.push('\n');
        fs::write(&path, text).with_context(|| format!("write schema file {}", path.display()))?;
        println!("wrote {}", path.display());
    }
    Ok(count)
}

/// Common header injected at the top of every schema document.
fn migration_note(target: &'static str) -> Value {
    json!(format!(
        "Pending(schemars): replace hand-written schema with `schemars::schema_for!({target})` once jekko-core gains a schemars dep."
    ))
}

fn schema_config() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "_note": migration_note("Config"),
        "title": "Config",
        "type": "object",
        "additionalProperties": true,
        "properties": {
            "$schema": { "type": "string" },
            "shell": { "type": "string" },
            "logLevel": { "type": "string", "enum": ["trace", "debug", "info", "warn", "error"] },
            "server": {},
            "command": { "type": "object", "additionalProperties": true },
            "skills": {},
            "watcher": { "$ref": "#/$defs/WatcherConfig" },
            "snapshot": { "type": "boolean" },
            "plugin": { "type": "array", "items": {} },
            "share": { "type": "string" },
            "autoshare": { "type": "boolean" },
            "autoupdate": {},
            "disabled_providers": { "type": "array", "items": { "type": "string" } },
            "enabled_providers": { "type": "array", "items": { "type": "string" } },
            "model": { "type": "string" },
            "small_model": { "type": "string" },
            "default_agent": { "type": "string" },
            "username": { "type": "string" },
            "mode": { "type": "object", "additionalProperties": true },
            "agent": { "type": "object", "additionalProperties": true },
            "provider": { "type": "object", "additionalProperties": true },
            "mcp": { "type": "object", "additionalProperties": true },
            "formatter": {},
            "lsp": {},
            "instructions": { "type": "array", "items": { "type": "string" } },
            "layout": { "type": "string" },
            "permission": { "$ref": "#/$defs/PermissionInput" },
            "tools": { "type": "object", "additionalProperties": { "type": "boolean" } },
            "enterprise": { "$ref": "#/$defs/EnterpriseConfig" },
            "tool_output": { "$ref": "#/$defs/ToolOutputConfig" },
            "compaction": { "$ref": "#/$defs/CompactionConfig" },
            "experimental": { "$ref": "#/$defs/ExperimentalConfig" }
        },
        "$defs": {
            "WatcherConfig": {
                "type": "object",
                "additionalProperties": true
            },
            "PermissionInput": {
                "oneOf": [
                    { "type": "array", "items": { "type": "string" } },
                    { "type": "object", "additionalProperties": true }
                ]
            },
            "EnterpriseConfig": { "type": "object", "additionalProperties": true },
            "ToolOutputConfig": { "type": "object", "additionalProperties": true },
            "CompactionConfig": { "type": "object", "additionalProperties": true },
            "ExperimentalConfig": { "type": "object", "additionalProperties": true }
        }
    })
}

fn schema_theme() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "_note": migration_note("Theme"),
        "title": "Theme",
        "type": "object",
        "additionalProperties": true,
        "properties": {
            "name": { "type": "string" },
            "defs": { "type": "object", "additionalProperties": { "type": "string" } },
            "theme": { "type": "object", "additionalProperties": true }
        }
    })
}

fn schema_keybinds_table() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "_note": migration_note("KeybindsTable"),
        "title": "KeybindsTable",
        "type": "object",
        "description": "Mapping from action name to a chord or chord set.",
        "additionalProperties": {
            "oneOf": [
                { "type": "string" },
                { "type": "array", "items": { "type": "string" } }
            ]
        }
    })
}

fn schema_provider_info() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "_note": migration_note("ProviderInfo"),
        "title": "ProviderInfo",
        "type": "object",
        "required": ["id", "name"],
        "properties": {
            "id": { "type": "string" },
            "name": { "type": "string" },
            "api": { "type": "object", "additionalProperties": true },
            "auth": { "type": "object", "additionalProperties": true },
            "modalities": { "type": "object", "additionalProperties": true },
            "interleaved": {},
            "capabilities": { "type": "object", "additionalProperties": true },
            "cost": { "type": "object", "additionalProperties": true },
            "limits": { "type": "object", "additionalProperties": true },
            "models": {
                "type": "array",
                "items": { "$ref": "#/$defs/Model" }
            },
            "source": { "type": "string" }
        },
        "$defs": {
            "Model": {
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "string" },
                    "name": { "type": "string" },
                    "status": { "type": "string" }
                },
                "additionalProperties": true
            }
        }
    })
}

fn schema_session_info() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "_note": migration_note("SessionInfo"),
        "title": "SessionInfo",
        "type": "object",
        "additionalProperties": true,
        "properties": {
            "id": { "type": "string" },
            "title": { "type": "string" },
            "createdAt": { "type": "string", "format": "date-time" },
            "updatedAt": { "type": "string", "format": "date-time" }
        }
    })
}

fn schema_permission_config() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "_note": migration_note("PermissionConfig"),
        "title": "PermissionConfig",
        "type": "object",
        "additionalProperties": true,
        "properties": {
            "edit": { "type": "string", "enum": ["allow", "ask", "deny"] },
            "bash": { "type": "string", "enum": ["allow", "ask", "deny"] },
            "webfetch": { "type": "string", "enum": ["allow", "ask", "deny"] }
        }
    })
}

fn schema_project_id() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "_note": migration_note("ProjectId"),
        "title": "ProjectId",
        "type": "string",
        "minLength": 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_have_unique_filenames() {
        let entries = entries();
        let mut names: Vec<&str> = entries.iter().map(|e| e.file).collect();
        names.sort();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "schema filenames must be unique");
    }

    #[test]
    fn every_schema_carries_migration_note() {
        for entry in entries() {
            let note = entry.schema.get("_note");
            assert!(
                note.is_some(),
                "schema {} is missing _note header pointing at schemars",
                entry.file
            );
            const DEFAULT_NOTE_TEXT: &str = "";
            #[allow(clippy::manual_unwrap_or_default)]
            let text = match note.unwrap().as_str() {
                Some(value) => value,
                None => DEFAULT_NOTE_TEXT,
            };
            assert!(
                text.contains("schemars"),
                "schema {} _note must reference schemars",
                entry.file
            );
        }
    }

    #[test]
    fn emit_writes_all_entries_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let written = emit(dir.path()).unwrap();
        assert_eq!(written, entries().len());
        for entry in entries() {
            let path = dir
                .path()
                .join("crates/xtask/generated-schemas")
                .join(entry.file);
            assert!(path.exists(), "{} should have been emitted", path.display());
            let body = std::fs::read_to_string(&path).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(
                parsed
                    .get("_note")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains("schemars")),
                Some(true)
            );
        }
    }
}
