//! Part-level transforms: filter unsupported file/image content.
//!
//! Mirrors `unsupportedParts` in `transform-message-utils.ts`.

use jekko_core::provider::Model;
use serde_json::{json, Value};

use crate::transform::shared;

use super::ModelMessage;

/// Filter out unsupported file/image parts and replace them with an error
/// text block.
pub(super) fn unsupported_parts(msgs: Vec<ModelMessage>, model: &Model) -> Vec<ModelMessage> {
    msgs.into_iter()
        .map(|mut msg| {
            if msg.get("role").and_then(Value::as_str) != Some("user") {
                return msg;
            }
            let Some(arr) = msg.get_mut("content").and_then(Value::as_array_mut) else {
                return msg;
            };
            for part in arr.iter_mut() {
                let part_type = part
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if part_type != "file" && part_type != "image" {
                    continue;
                }

                if part_type == "image" {
                    // Detect empty base64 payload.
                    if let Some(image) = part.get("image").and_then(Value::as_str) {
                        if let Some(stripped) = image.strip_prefix("data:") {
                            if let Some((_, b64)) = stripped.split_once("base64,") {
                                if b64.is_empty() {
                                    *part = json!({
                                        "type": "text",
                                        "text": "ERROR: Image file is empty or corrupted. Please provide a valid image.",
                                    });
                                    continue;
                                }
                            }
                        }
                    }
                }

                let mime: String = extract_part_mime(part, &part_type);
                let filename = part
                    .get("filename")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());
                let modality = shared::mime_to_modality(&mime);
                let supported_mod = match modality {
                    shared::MimeModalityResult::Supported(m) => m,
                    shared::MimeModalityResult::Unsupported => continue,
                };
                let supported_by_model = match supported_mod {
                    shared::Modality::Text => model.capabilities.input.text,
                    shared::Modality::Image => model.capabilities.input.image,
                    shared::Modality::Audio => model.capabilities.input.audio,
                    shared::Modality::Video => model.capabilities.input.video,
                    shared::Modality::Pdf => model.capabilities.input.pdf,
                };
                if supported_by_model {
                    continue;
                }

                let label = match filename {
                    Some(name) => format!("\"{name}\""),
                    None => supported_mod.as_str().to_string(),
                };
                let msg_text = format!(
                    "ERROR: Cannot read {label} (this model does not support {} input). Inform the user.",
                    supported_mod.as_str(),
                );
                *part = json!({ "type": "text", "text": msg_text });
            }
            msg
        })
        .collect()
}

/// Read the MIME type from a `user` content part. For images, the MIME is
/// embedded in the `image` data URL prefix (`data:image/png;base64,...`); for
/// files, it comes from the explicit `mediaType` field. Returns an empty
/// string when neither shape carries a value — callers treat that as
/// "unknown" and skip the part.
fn extract_part_mime(part: &Value, part_type: &str) -> String {
    match part_type {
        "image" => match part.get("image").and_then(Value::as_str) {
            Some(image) => {
                let prefix = match image.split(';').next() {
                    Some(p) => p,
                    None => return String::new(),
                };
                prefix.replace("data:", "")
            }
            None => String::new(),
        },
        "file" => match part.get("mediaType").and_then(Value::as_str) {
            Some(s) => s.to_string(),
            None => String::new(),
        },
        _ => String::new(),
    }
}
