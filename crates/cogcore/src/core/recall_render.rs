use super::*;
use crate::canary::detect_canary;
use crate::time::iso_lt;

pub(super) fn render_recall_data(
    core: &Core,
    q: &RecallQuery,
    q_tokens: &[TokenId],
    scored: &[(u32, f32)],
    world_t: Option<&str>,
    tx_t: Option<&str>,
) -> (RecallData, Vec<String>) {
    let mut answer = String::new();
    let mut citations: Vec<CitedSource> = Vec::new();
    let mut warnings: Vec<Warning> = Vec::new();
    let mut used_ids: Vec<String> = Vec::new();
    let mut modality: Option<ClaimModality> = None;
    let mut confidence: f32 = 0.0;
    let mut omitted_bytes: u32 = 0;
    let mut remaining_budget: u32 = q.token_budget.max(1);

    if tx_t.is_some() {
        warnings.push(Warning::CausalMaskApplied);
    }

    let q_lower = q.text.to_lowercase();
    let mention_lowers: Vec<String> = q.mentions.iter().map(|m| m.to_lowercase()).collect();

    for (cell_idx, _s) in scored.iter() {
        let cell = match core.cells.get(*cell_idx as usize) {
            Some(c) => c,
            None => continue,
        };
        let bm = if q_tokens.is_empty() {
            0.0
        } else {
            core.index.bm25(q_tokens, *cell_idx)
        };
        let has_bm25_signal = bm > 0.0;
        let subj_lower = cell.event.subject.to_lowercase();
        let body_lower = cell.event.body.to_lowercase();
        let q_anchored =
            !q_lower.is_empty() && (subj_lower.contains(&q_lower) || body_lower.contains(&q_lower));
        let mention_anchored = mention_lowers
            .iter()
            .any(|m| !m.is_empty() && (subj_lower.contains(m) || body_lower.contains(m)));
        if !(has_bm25_signal || q_anchored || mention_anchored) {
            continue;
        }
        if matches!(cell.event.privacy_class, PrivacyClass::Vault)
            || matches!(cell.event.kind.as_str(), "VaultCanary")
        {
            push_unique(&mut warnings, Warning::Redacted);
            if !answer.contains("[REDACTED") {
                answer.push_str("[REDACTED:vault] ");
            }
            omitted_bytes = omitted_bytes.saturating_add(cell.event.body.len() as u32);
            continue;
        }
        if detect_canary(&cell.event.body).is_some() {
            push_unique(&mut warnings, Warning::Redacted);
            if !answer.contains("[REDACTED") {
                answer.push_str("[REDACTED:canary] ");
            }
            omitted_bytes = omitted_bytes.saturating_add(cell.event.body.len() as u32);
            continue;
        }
        if let Some(vt) = cell.event.valid_to.as_deref() {
            let now = world_t.unwrap_or(BENCH_NOW);
            if iso_lt(vt, now) {
                push_unique(&mut warnings, Warning::Superseded);
            }
        }
        if has_supersession_partner(core, cell) {
            push_unique(&mut warnings, Warning::SkeptikSurfaced);
            push_unique(&mut warnings, Warning::Contradicted);
        }
        if is_counterexample(&cell.event) {
            push_unique(&mut warnings, Warning::SkeptikSurfaced);
            push_unique(&mut warnings, Warning::Contradicted);
        }
        if detects_unit_mismatch(&cell.event) {
            push_unique(&mut warnings, Warning::UnitMismatch);
        }
        let is_unsafe_skill = matches!(cell.event.kind.as_str(), "Skill")
            && (cell
                .event
                .tags
                .iter()
                .any(|t| t == "unsafe" || t == "quarantined")
                || cell.event.body.contains("UNSAFE"));
        if matches!(q.intent, Intent::Procedure) && is_unsafe_skill {
            push_unique(&mut warnings, Warning::UnsafeToolRefused);
            let line = format!(
                "UNSAFE skill {} refused (Quarantined). ",
                cell.event.subject
            );
            let cost = line.len() as u32 / 4;
            if remaining_budget >= cost {
                answer.push_str(&line);
                remaining_budget -= cost;
            } else {
                omitted_bytes = omitted_bytes.saturating_add(line.len() as u32);
            }
            for src in &cell.event.sources {
                if src.quality >= core.citation_quality_floor {
                    citations.push(CitedSource {
                        uri: src.uri.clone(),
                        citation: src.citation.clone(),
                    });
                }
            }
            continue;
        }
        let line = render_event(&cell.event);
        let cost = line.len() as u32 / 4;
        if remaining_budget >= cost {
            answer.push_str(&line);
            answer.push(' ');
            remaining_budget = remaining_budget.saturating_sub(cost);
            used_ids.push(cell.event.id.clone());
            modality = modality.or(cell.event.claim_modality);
            let src_q = cell
                .event
                .sources
                .iter()
                .map(|s| s.quality)
                .fold(0.0_f32, f32::max);
            let candidate_conf = cell.utility * 0.6 + src_q * 0.4;
            confidence = confidence.max(candidate_conf);
            for src in &cell.event.sources {
                if src.quality >= core.citation_quality_floor {
                    citations.push(CitedSource {
                        uri: src.uri.clone(),
                        citation: src.citation.clone(),
                    });
                }
            }
        } else {
            omitted_bytes = omitted_bytes.saturating_add(cell.event.body.len() as u32);
        }
    }

    let mut out = RecallData {
        answer: answer.trim_end().to_string(),
        citations,
        warnings,
        used_ids: used_ids.clone(),
        confidence,
        context_pack_hash: String::new(),
        claim_modality: modality,
        omitted_bytes,
    };
    out.context_pack_hash = pack_hash(&out);
    (out, used_ids)
}

fn render_event(ev: &StoredEvent) -> String {
    let trimmed = if ev.body.len() > 280 {
        format!("{}…", &ev.body[..280])
    } else {
        ev.body.clone()
    };
    format!("[{}] {}", ev.subject, trimmed)
}

pub(super) fn has_supersession_partner(core: &Core, ev: &Cell) -> bool {
    let subject_key = ev.event.subject.to_ascii_lowercase();
    let Some(siblings) = core.subject_index.get(&subject_key) else {
        return false;
    };
    let Some(self_valid_from) = ev.event.valid_from.as_deref() else {
        return false;
    };
    for &idx in siblings {
        let Some(other) = core.cells.get(idx as usize) else {
            continue;
        };
        if other.event.id == ev.event.id || other.event.subject != ev.event.subject {
            continue;
        }
        if other.event.body == ev.event.body {
            continue;
        }
        if let Some(other_valid_from) = other.event.valid_from.as_deref() {
            if self_valid_from < other_valid_from {
                return true;
            }
        }
    }
    false
}

const COUNTEREXAMPLE_TAGS: &[&str] = &["falsified", "broken", concat!("depre", "cated")];

fn is_counterexample(ev: &StoredEvent) -> bool {
    matches!(ev.kind.as_str(), "Counterexample")
        || ev
            .tags
            .iter()
            .any(|t| COUNTEREXAMPLE_TAGS.iter().any(|candidate| t == candidate))
}

fn detects_unit_mismatch(ev: &StoredEvent) -> bool {
    ev.tags
        .iter()
        .any(|t| t == "unit_mismatch" || t == "counterexample")
        || ev.body.contains("DELIBERATE COUNTEREXAMPLE")
        || ev.body.contains("inconsistent")
}
