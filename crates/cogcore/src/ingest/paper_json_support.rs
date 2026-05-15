use std::collections::BTreeMap;

use crate::core::{ClaimModality, PrivacyClass};
use super::paper_json_parse::get_string;
use super::paper_json_parse::get_source_array;
use super::paper_json_parse::get_string_array;

pub(crate) fn parse_string_or_none(map: &BTreeMap<String, String>, key: &str) -> Option<String> {
    get_string(map, key)
}

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
    let privacy_class = if let Some(value) = map.get("privacy_class").map(String::as_str) {
        if value == "Internal" {
            PrivacyClass::Internal
        } else if value == "Confidential" {
            PrivacyClass::Confidential
        } else if value == "Secret" {
            PrivacyClass::Secret
        } else if value == "Vault" {
            PrivacyClass::Vault
        } else {
            PrivacyClass::Public
        }
    } else {
        PrivacyClass::Public
    };

    let claim_modality = if let Some(value) = map.get("claim_modality").map(String::as_str) {
        if value == "Observed" {
            Some(ClaimModality::Observed)
        } else if value == "AssertedBySource" {
            Some(ClaimModality::AssertedBySource)
        } else if value == "InferredByAgent" {
            Some(ClaimModality::InferredByAgent)
        } else if value == "HumanApproved" {
            Some(ClaimModality::HumanApproved)
        } else if value == "FormallyVerified" {
            Some(ClaimModality::FormallyVerified)
        } else {
            None
        }
    } else {
        None
    };

    (privacy_class, claim_modality)
}

pub(crate) fn parse_string_array_with_default(
    map: &BTreeMap<String, String>,
    key: &str,
) -> Vec<String> {
    match get_string_array(map, key) {
        Some(values) => values,
        None => Vec::new(),
    }
}

pub(crate) fn parse_source_array_with_default(
    map: &BTreeMap<String, String>,
    key: &str,
) -> Vec<crate::core::SourceRef> {
    match get_source_array(map, key) {
        Some(values) => values,
        None => Vec::new(),
    }
}
