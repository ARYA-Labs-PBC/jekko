use super::*;

pub(crate) fn generator_output_from_selection(
    selection: GeneratorSelectionOutput,
    quote_candidates: &[SupportQuoteCandidate],
    failures: &mut Vec<AgentFailure>,
    receipt: &AgentCallReceipt,
) -> GeneratorAgentOutput {
    let support = quote_candidates
        .iter()
        .find(|candidate| candidate.id == selection.support_quote_id)
        .map(|candidate| SupportQuote {
            section_id: candidate.section_id.clone(),
            section_hash: candidate.section_hash.clone(),
            quote: candidate.quote.clone(),
            why_it_matters: "Selected from deterministic canonical support candidates.".to_string(),
        })
        .into_iter()
        .collect::<Vec<_>>();
    if support.is_empty() {
        failures.push(failure(
            "generation",
            &receipt.agent_name,
            format!(
                "generator selected unknown support_quote_id {}",
                selection.support_quote_id
            ),
            receipt,
        ));
    }
    GeneratorAgentOutput {
        question: selection.question,
        answer: selection.answer,
        difficulty_rationale: selection.difficulty_rationale,
        expected_failure_mode: selection.expected_failure_mode,
        support,
        required_key_points: selection.required_key_points,
        confidence: selection.confidence,
    }
}

pub(crate) fn generator_prompt(
    paper: &PaperRecord,
    index: usize,
    quote_candidates: &[SupportQuoteCandidate],
) -> String {
    let mut candidates = String::new();
    for candidate in quote_candidates {
        candidates.push_str(&format!(
            "[quote_id: {}]\n[section_title: {}]\n{}\n\n",
            candidate.id, candidate.section_title, candidate.quote
        ));
    }
    format!(
        "Create one hard recall question from this redistributable paper using exactly one supplied support quote candidate.\n\
Rules:\n\
- Return support_quote_id as one quote_id from the supplied list.\n\
- Do not invent section ids, section hashes, quotes, or paper facts.\n\
- The runner will derive the production hard answer from the selected canonical quote.\n\
- The production hard answer is the complete selected support quote, so write a question whose correct answer requires all important details in that quote, not a single value or short gist.\n\
- The answer field may be a compact raw answer for receipt purposes, but it must not introduce facts absent from the selected quote.\n\
- Prefer quotes with multiple concrete constraints, measurements, groups, settings, or outcomes that saturated-context answerers may miss when target and distractor papers are mixed.\n\
- Avoid questions answerable by copying one obvious number, one direction of change, or a fact stated in the paper title.\n\
Hardness rules:\n\
- Do not include exact numbers, unique chemical names, table labels, section titles, variable names, or rare phrases from the answer unless they are unavoidable common domain terms.\n\
- Do not ask \"what were the three major constituents\", \"what was the gradient program\", or \"what formula was used\"; these are keyword-search questions.\n\
- The question must be unambiguous when the selected support quote is shown, but difficult when the target paper is hidden among similar papers.\n\
- Prefer questions requiring the complete relationship among condition, comparator, measurement, and conclusion.\n\
- required_key_points must list 3 to 8 exact substrings copied from the selected support quote.\n\
Generator index: {}\n\
Title: {}\n\
Publication hash: {}\n\
License: {}\n\n\
Support quote candidates:\n{}",
        index + 1,
        paper.title,
        paper.publication_hash,
        paper.license.spdx,
        candidates
    )
}

pub(crate) fn verifier_prompt(
    paper: &PaperRecord,
    question: &str,
    answer: &str,
    quote: &str,
) -> String {
    format!(
        "Verify whether this candidate is exactly answerable from the paper. Return accepted=false if support is missing, paraphrased, ambiguous, or not hard.\n\
Return accepted=false if the question can be answered correctly without every required material detail in the support quote.\n\
Return accepted=false if the question leaks rare answer tokens that make saturated retrieval easy.\n\
Question: {question}\n\
Answer: {answer}\n\
Required support quote: {quote}\n\n{}",
        paper_prompt_context(paper)
    )
}

pub(crate) fn grader_prompt(question: &str, answer: &str, tester_answer: &str) -> String {
    format!(
        "Grade the tester answer against the hard answer. The tester is correct only if every required key point is present with the same relation, comparator, condition, and value. Mark partial retrieval, one-number answers, matching only the topic, or one-clause answers incorrect even if one fact is right.\n\
Return compact JSON only. Keep reason, matched_key_points, and missed_key_points short.\n\
Question: {question}\n\
Hard answer: {answer}\n\
Tester answer: {tester_answer}"
    )
}

pub(crate) fn paper_prompt_context(paper: &PaperRecord) -> String {
    let mut out = format!(
        "Title: {}\nPublication hash: {}\nLicense: {}\n\n",
        paper.title, paper.publication_hash, paper.license.spdx
    );
    for section in &paper.sections {
        out.push_str(&format!(
            "[section_id: {}]\n[section_hash: {}]\n[title: {}]\n{}\n\n",
            section.section_id, section.section_hash, section.title, section.text
        ));
    }
    out
}

pub(crate) fn string_schema() -> serde_json::Value {
    json!({"type": "string", "minLength": 1})
}

pub(crate) fn confidence_schema() -> serde_json::Value {
    json!({"type": "integer", "minimum": 1, "maximum": 100})
}

pub(crate) fn generator_selection_response_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["question", "answer", "difficulty_rationale", "expected_failure_mode", "support_quote_id", "required_key_points", "confidence"],
        "properties": {
            "question": string_schema(),
            "answer": string_schema(),
            "difficulty_rationale": string_schema(),
            "expected_failure_mode": string_schema(),
            "support_quote_id": string_schema(),
            "required_key_points": {
                "type": "array",
                "minItems": 3,
                "maxItems": 8,
                "items": {"type": "string", "minLength": 4}
            },
            "confidence": confidence_schema()
        }
    })
}

pub(crate) fn verification_response_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["accepted", "answer", "confidence", "support_correct", "reason", "missing_or_wrong_support"],
        "properties": {
            "accepted": {"type": "boolean"},
            "answer": string_schema(),
            "confidence": confidence_schema(),
            "support_correct": {"type": "boolean"},
            "reason": string_schema(),
            "missing_or_wrong_support": {"type": "array", "items": string_schema()}
        }
    })
}

pub(crate) fn testing_response_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["answer", "confidence", "reasoning_summary"],
        "properties": {
            "answer": string_schema(),
            "confidence": confidence_schema(),
            "reasoning_summary": string_schema()
        }
    })
}

pub(crate) fn grading_response_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["correct", "score_0_100", "matched_key_points", "missed_key_points", "reason"],
        "properties": {
            "correct": {"type": "boolean"},
            "score_0_100": {"type": "integer", "minimum": 0, "maximum": 100},
            "matched_key_points": {"type": "array", "items": string_schema()},
            "missed_key_points": {"type": "array", "items": string_schema()},
            "reason": string_schema()
        }
    })
}
