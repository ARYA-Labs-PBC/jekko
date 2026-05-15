use super::{
    GeneratorAgentOutput, GradingAgentOutput, PaperRecord, SupportQuote, TestingAgentOutput,
    VerificationAgentOutput,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

pub fn extract_agent_json(raw: &str) -> Result<&str, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("agent output is empty".to_string());
    }
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Ok(trimmed);
    }
    if let Some(extracted) = fenced_json(trimmed) {
        return Ok(extracted);
    }
    balanced_object(trimmed)
}

pub fn parse_agent_json<T>(raw: &str) -> Result<T, String>
where
    T: DeserializeOwned + Serialize,
{
    let json = extract_agent_json(raw)?;
    let value = serde_json::from_str::<Value>(json).map_err(|err| err.to_string())?;
    serde_json::from_value(value).map_err(|err| err.to_string())
}

pub fn validate_generator_output(
    output: &GeneratorAgentOutput,
    paper: &PaperRecord,
) -> Result<(), String> {
    require_text("question", &output.question)?;
    require_text("answer", &output.answer)?;
    require_text("difficulty_rationale", &output.difficulty_rationale)?;
    require_text("expected_failure_mode", &output.expected_failure_mode)?;
    validate_agent_confidence(output.confidence)?;
    if output.support.is_empty() {
        return Err("generator support is empty".to_string());
    }
    for support in &output.support {
        validate_support_quote(support, paper)?;
    }
    if output.required_key_points.len() < 3 || output.required_key_points.len() > 8 {
        return Err("required_key_points must contain 3..=8 items".to_string());
    }
    let support_quote = output
        .support
        .first()
        .map(|support| support.quote.as_str())
        .unwrap_or("");
    for point in &output.required_key_points {
        require_text("required_key_points[]", point)?;
        if !support_quote.contains(point.trim()) {
            return Err("required_key_points[] must be exact support quote substrings".to_string());
        }
    }
    Ok(())
}

pub fn validate_verification_output(output: &VerificationAgentOutput) -> Result<(), String> {
    require_text("answer", &output.answer)?;
    require_text("reason", &output.reason)?;
    validate_agent_confidence(output.confidence)
}

pub fn validate_testing_output(output: &TestingAgentOutput) -> Result<(), String> {
    require_text("answer", &output.answer)?;
    require_text("reasoning_summary", &output.reasoning_summary)?;
    validate_agent_confidence(output.confidence)
}

pub fn validate_grading_output(output: &GradingAgentOutput) -> Result<(), String> {
    require_text("reason", &output.reason)?;
    validate_score_0_100(output.score_0_100)
}

fn fenced_json(input: &str) -> Option<&str> {
    let start = input.find("```json")?;
    let rest = &input[start + "```json".len()..];
    let end = rest.find("```")?;
    Some(rest[..end].trim())
}

fn balanced_object(input: &str) -> Result<&str, String> {
    let mut start = None;
    let mut depth = 0_i64;
    let mut in_string = false;
    let mut escaped = false;
    let mut end = None;
    for (index, ch) in input.char_indices() {
        if start.is_none() {
            if ch == '{' {
                start = Some(index);
                depth = 1;
            }
            continue;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                end = Some(index + ch.len_utf8());
                break;
            }
        }
    }
    let Some(start) = start else {
        return Err("agent output contains no JSON object".to_string());
    };
    let Some(end) = end else {
        return Err("agent output contains an unterminated JSON object".to_string());
    };
    if input[end..].contains('{') {
        return Err("agent output contains multiple JSON objects".to_string());
    }
    Ok(input[start..end].trim())
}

fn validate_support_quote(support: &SupportQuote, paper: &PaperRecord) -> Result<(), String> {
    require_text("support.section_id", &support.section_id)?;
    require_text("support.section_hash", &support.section_hash)?;
    require_text("support.quote", &support.quote)?;
    let Some(section) = paper
        .sections
        .iter()
        .find(|section| section.section_id == support.section_id)
    else {
        return Err(format!("support section {} is unknown", support.section_id));
    };
    if section.section_hash != support.section_hash {
        return Err(format!(
            "support section {} hash mismatch",
            support.section_id
        ));
    }
    if !section.text.contains(support.quote.trim()) {
        return Err(format!(
            "support quote is absent from section {}",
            support.section_id
        ));
    }
    Ok(())
}

fn require_text(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} is empty"))
    } else {
        Ok(())
    }
}

fn validate_agent_confidence(confidence: u8) -> Result<(), String> {
    if (1..=100).contains(&confidence) {
        Ok(())
    } else {
        Err("confidence outside 1..=100".to_string())
    }
}

fn validate_score_0_100(score: u8) -> Result<(), String> {
    if score <= 100 {
        Ok(())
    } else {
        Err("score outside 0..=100".to_string())
    }
}
