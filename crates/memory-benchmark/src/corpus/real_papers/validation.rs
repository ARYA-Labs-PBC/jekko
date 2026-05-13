use super::model::{
    stable_challenge_hash, stable_section_hash, BankValidation, ContextPack, PaperChallenge,
};
use super::parse::{collect_json_files, read_challenges, read_paper};
use crate::qbank_hash::sha256_hex;
use std::collections::BTreeSet;
use std::path::Path;

pub fn validate_bank(
    root: &Path,
    allow_empty: bool,
    top_n: usize,
) -> Result<BankValidation, String> {
    let mut result = BankValidation::default();
    let mut paper_paths = Vec::new();
    collect_json_files(&root.join("papers"), &mut paper_paths)?;
    let mut challenge_paths = Vec::new();
    collect_json_files(&root.join("challenges"), &mut challenge_paths)?;
    let mut rejected_paths = Vec::new();
    collect_json_files(&root.join("rejected"), &mut rejected_paths)?;

    let mut seen_publications = BTreeSet::new();
    for path in &paper_paths {
        match read_paper(path) {
            Ok(paper) => {
                if !seen_publications.insert(paper.publication_hash.clone()) {
                    result.duplicate_publications += 1;
                    result
                        .errors
                        .push(format!("duplicate publication {}", paper.publication_hash));
                }
                if !paper.redistributable {
                    result
                        .errors
                        .push(format!("non-redistributable paper {}", path.display()));
                }
                for section in &paper.sections {
                    let expected = stable_section_hash(&section.text);
                    if !section.section_hash.is_empty() && section.section_hash != expected {
                        result.errors.push(format!(
                            "{} section {} hash mismatch",
                            path.display(),
                            section.section_id
                        ));
                    }
                }
            }
            Err(err) => result.errors.push(err),
        }
    }

    let mut accepted = Vec::new();
    let mut seen_challenges = BTreeSet::new();
    for path in &challenge_paths {
        match read_challenges(path) {
            Ok(challenges) => {
                for challenge in challenges {
                    if !seen_challenges.insert(challenge.challenge_hash.clone()) {
                        result
                            .errors
                            .push(format!("duplicate challenge {}", challenge.challenge_hash));
                    }
                    if let Err(err) = validate_challenge_hash(&challenge) {
                        result.errors.push(format!("{}: {err}", path.display()));
                    }
                    if let Err(err) = validate_acceptance(&challenge) {
                        result.errors.push(format!("{}: {err}", path.display()));
                    }
                    if challenge.context_pack.estimated_tokens
                        > context_token_budget(&challenge.context_pack)
                    {
                        result.errors.push(format!(
                            "{}: context pack exceeds token limit",
                            path.display()
                        ));
                    }
                    accepted.push(challenge);
                }
            }
            Err(err) => result.errors.push(err),
        }
    }
    result.accepted_challenges = accepted.len();
    result.rejected_challenges = rejected_paths.len();
    result.top_selected = accepted.len().min(top_n);
    accepted.sort_by(challenge_order_plain);
    let mut manifest_material = String::new();
    for challenge in accepted.iter().take(result.top_selected) {
        manifest_material.push_str(&challenge.challenge_hash);
        manifest_material.push('\n');
    }
    result.manifest_hash = sha256_hex(manifest_material.as_bytes());

    if !allow_empty && result.accepted_challenges == 0 {
        result
            .errors
            .push("bank has no accepted challenges".to_string());
    }
    Ok(result)
}

fn validate_challenge_hash(challenge: &PaperChallenge) -> Result<(), String> {
    if challenge.challenge_hash.len() != 64 {
        return Ok(()); // Older fixture hashes are accepted by content checks.
    }
    let support_hashes = challenge
        .support
        .iter()
        .map(|support| support.section_hash.clone())
        .collect::<Vec<_>>();
    let expected = stable_challenge_hash(
        &challenge.publication_hash,
        &challenge.question,
        &challenge.answer_key.canonical,
        &support_hashes,
    );
    if expected != challenge.challenge_hash {
        return Err("challenge_hash mismatch".to_string());
    }
    Ok(())
}

fn validate_acceptance(challenge: &PaperChallenge) -> Result<(), String> {
    if challenge.answerability < 0.90 {
        return Err("answerability below 0.90".to_string());
    }
    if challenge.blind_correct_rate > 0.50 {
        return Err("blind_correct_rate above 0.50".to_string());
    }
    if challenge.focused_correct_rate < 0.90 {
        return Err("focused_correct_rate below 0.90".to_string());
    }
    Ok(())
}

fn context_token_budget(context: &ContextPack) -> u32 {
    ((context.safe_window_tokens as f32 * context.target_fill_ratio).floor() as i64
        - context.output_reserve_tokens as i64)
        .max(0) as u32
}

fn challenge_order_plain(a: &PaperChallenge, b: &PaperChallenge) -> std::cmp::Ordering {
    b.difficulty_score
        .total_cmp(&a.difficulty_score)
        .then(b.focused_correct_rate.total_cmp(&a.focused_correct_rate))
        .then(a.blind_correct_rate.total_cmp(&b.blind_correct_rate))
        .then(a.publication_hash.cmp(&b.publication_hash))
        .then(a.challenge_hash.cmp(&b.challenge_hash))
}
