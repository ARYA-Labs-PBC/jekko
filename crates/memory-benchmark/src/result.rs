use crate::{ClaimModality, Warning};

#[derive(Debug, Clone, Default)]
pub struct RecallResult {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub claims: Vec<ClaimRecord>,
    pub warnings: Vec<Warning>,
    pub omitted: Vec<OmissionNote>,
    pub redactions: Vec<Redaction>,
    pub skill_calls: Vec<SkillCall>,
    pub used_ids: Vec<String>,
    pub excluded_ids: Vec<String>,
    pub derived_from: Vec<String>,
    pub confidence: f32,
    pub context_token_count: u32,
    pub retrieved_token_count: u32,
    pub state_bytes: u64,
    pub context_pack_hash: String,
    pub claim_modality: Option<ClaimModality>,
}

#[derive(Debug, Clone)]
pub struct Citation {
    pub source_uri: String,
    pub citation: String,
    pub quote: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClaimRecord {
    pub id: String,
    pub text: String,
    pub status: ClaimStatus,
    pub support: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimStatus {
    Supported,
    Contradicted,
    Superseded,
    Redacted,
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct Redaction {
    pub channel: String,
    pub reason: String,
    pub evidence_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SkillCall {
    pub name: String,
    pub args_hash: String,
    pub refused: bool,
}

#[derive(Debug, Clone)]
pub struct ContextMetric {
    pub budget_tokens: u32,
    pub used_tokens: u32,
    pub retrieved_tokens: u32,
    pub state_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct OmissionNote {
    pub reason: String,
    pub kind: String,
    pub bytes: u32,
}
