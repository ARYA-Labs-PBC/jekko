use super::*;

pub(crate) fn push_unique(warnings: &mut Vec<Warning>, w: Warning) {
    if !warnings.contains(&w) {
        warnings.push(w);
    }
}

pub fn pack_hash(r: &RecallData) -> String {
    let mut buf = String::new();
    buf.push_str("a:");
    buf.push_str(&r.answer);
    buf.push('|');
    buf.push_str("c:");
    for c in r.citations.iter() {
        buf.push_str(&c.uri);
        buf.push('@');
        buf.push_str(&c.citation);
        buf.push(';');
    }
    buf.push_str("|w:");
    for w in r.warnings.iter() {
        buf.push_str(warning_name(*w));
        buf.push(',');
    }
    buf.push_str("|u:");
    for id in r.used_ids.iter() {
        buf.push_str(id);
        buf.push(',');
    }
    buf.push_str("|conf:");
    buf.push_str(&format!("{:.4}", r.confidence));
    fnv1a_hex(&buf)
}

fn warning_name(w: Warning) -> &'static str {
    match w {
        Warning::Superseded => "superseded",
        Warning::Contradicted => "contradicted",
        Warning::Redacted => "redacted",
        Warning::CausalMaskApplied => "causal_mask_applied",
        Warning::SkeptikSurfaced => "skeptic_surfaced",
        Warning::UnitMismatch => "unit_mismatch",
        Warning::Abstained => "abstained",
        Warning::UnsafeToolRefused => "unsafe_tool_refused",
    }
}

pub(crate) fn privacy_byte(p: PrivacyClass) -> u8 {
    match p {
        PrivacyClass::Public => 0,
        PrivacyClass::Internal => 1,
        PrivacyClass::Confidential => 2,
        PrivacyClass::Secret => 3,
        PrivacyClass::Vault => 4,
    }
}

pub(crate) fn privacy_from_byte(b: u8) -> PrivacyClass {
    match b {
        1 => PrivacyClass::Internal,
        2 => PrivacyClass::Confidential,
        3 => PrivacyClass::Secret,
        4 => PrivacyClass::Vault,
        _ => PrivacyClass::Public,
    }
}

pub(crate) fn modality_byte(m: ClaimModality) -> u8 {
    match m {
        ClaimModality::Observed => 0,
        ClaimModality::AssertedBySource => 1,
        ClaimModality::InferredByAgent => 2,
        ClaimModality::HumanApproved => 3,
        ClaimModality::FormallyVerified => 4,
    }
}

pub(crate) fn modality_from_byte(b: u8) -> ClaimModality {
    match b {
        1 => ClaimModality::AssertedBySource,
        2 => ClaimModality::InferredByAgent,
        3 => ClaimModality::HumanApproved,
        4 => ClaimModality::FormallyVerified,
        _ => ClaimModality::Observed,
    }
}

pub(crate) fn outcome_byte(o: Outcome) -> u8 {
    match o {
        Outcome::TaskSuccess => 0,
        Outcome::TaskFailure => 1,
        Outcome::Verified => 2,
        Outcome::Falsified => 3,
        Outcome::Ignored => 4,
    }
}

pub(crate) fn outcome_from_byte(b: u8) -> Outcome {
    match b {
        1 => Outcome::TaskFailure,
        2 => Outcome::Verified,
        3 => Outcome::Falsified,
        4 => Outcome::Ignored,
        _ => Outcome::TaskSuccess,
    }
}
