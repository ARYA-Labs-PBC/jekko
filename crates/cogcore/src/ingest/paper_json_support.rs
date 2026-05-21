use std::collections::BTreeMap;

use super::paper_json_parse::get_source_array;
use super::paper_json_parse::get_string;
use super::paper_json_parse::get_string_array;
use crate::core::{ClaimModality, PrivacyClass};

pub(crate) fn parse_string_or_none(map: &BTreeMap<String, String>, key: &str) -> Option<String> {
    get_string(map, key)
}

#[allow(clippy::manual_unwrap_or_default)]
pub(crate) fn parse_string_or_empty(map: &BTreeMap<String, String>, key: &str) -> String {
    match parse_string_or_none(map, key) {
        Some(value) => value,
        None => String::new(),
    }
}

pub(crate) fn parse_string_or_default(
    map: &BTreeMap<String, String>,
    key: &str,
    default: &str,
) -> String {
    match parse_string_or_none(map, key) {
        Some(value) => value,
        None => default.to_string(),
    }
}

pub(crate) fn parse_json_classifiers(
    map: &BTreeMap<String, String>,
) -> (PrivacyClass, Option<ClaimModality>) {
    let privacy_class = match map.get("privacy_class").map(String::as_str) {
        Some("Internal") => PrivacyClass::Internal,
        Some("Confidential") => PrivacyClass::Confidential,
        Some("Secret") => PrivacyClass::Secret,
        Some("Vault") => PrivacyClass::Vault,
        Some(_) | None => PrivacyClass::Public,
    };

    let claim_modality = match map.get("claim_modality").map(String::as_str) {
        Some("Observed") => Some(ClaimModality::Observed),
        Some("AssertedBySource") => Some(ClaimModality::AssertedBySource),
        Some("InferredByAgent") => Some(ClaimModality::InferredByAgent),
        Some("HumanApproved") => Some(ClaimModality::HumanApproved),
        Some("FormallyVerified") => Some(ClaimModality::FormallyVerified),
        Some(_) | None => None,
    };

    (privacy_class, claim_modality)
}

pub(crate) fn parse_string_array_with_default(
    map: &BTreeMap<String, String>,
    key: &str,
) -> Vec<String> {
    #[allow(clippy::manual_unwrap_or_default)]
    match get_string_array(map, key) {
        Some(values) => values,
        None => Vec::new(),
    }
}

pub(crate) fn parse_source_array_with_default(
    map: &BTreeMap<String, String>,
    key: &str,
) -> Vec<crate::core::SourceRef> {
    #[allow(clippy::manual_unwrap_or_default)]
    match get_source_array(map, key) {
        Some(values) => values,
        None => Vec::new(),
    }
}
