use super::*;
use std::collections::BTreeSet;

pub(crate) fn paper_quality_allowed(paper: &PaperRecord) -> bool {
    let title = paper.title.to_ascii_lowercase();
    let blocked_title_terms = [
        "correction",
        "erratum",
        "corrigendum",
        "retraction",
        "editorial",
        "publisher's note",
        "publisher note",
        "article notice",
        "expression of concern",
        "notice of",
    ];
    if blocked_title_terms.iter().any(|term| title.contains(term)) {
        return false;
    }
    let body_sections = paper
        .sections
        .iter()
        .filter(|section| eligible_support_section(section))
        .collect::<Vec<_>>();
    if body_sections.is_empty() {
        return false;
    }
    let body_chars = body_sections
        .iter()
        .map(|section| section.text.trim().chars().count())
        .sum::<usize>();
    let all_chars = paper
        .sections
        .iter()
        .map(|section| section.text.trim().chars().count())
        .sum::<usize>()
        .max(1);
    if body_chars < 1_200 {
        return false;
    }
    if body_chars as f64 / all_chars as f64 <= 0.35 {
        return false;
    }
    let caption_like = body_sections
        .iter()
        .filter(|section| {
            let lower = format!(
                "{} {}",
                section.title.to_ascii_lowercase(),
                section.text.to_ascii_lowercase()
            );
            lower.contains("table ") || lower.contains("figure ") || lower.contains("caption")
        })
        .count();
    caption_like * 2 < body_sections.len().max(1)
}

pub(crate) fn support_quote_candidates(paper: &PaperRecord) -> Vec<SupportQuoteCandidate> {
    support_quote_candidates_with_min_score(paper, 10)
}

pub(crate) fn support_quote_candidates_with_min_score(
    paper: &PaperRecord,
    min_support_quote_score: i32,
) -> Vec<SupportQuoteCandidate> {
    let mut scored = Vec::<(i32, usize, SupportQuoteCandidate)>::new();
    let mut ordinal = 0usize;
    for section in &paper.sections {
        if !eligible_support_section(section) {
            continue;
        }
        for sentence in exact_sentences(&section.text) {
            if !eligible_support_quote(&section.title, &sentence, min_support_quote_score) {
                continue;
            }
            let score = support_quote_hardness_score(&section.title, &sentence);
            if scored
                .iter()
                .any(|(_, _, candidate)| candidate.quote == sentence)
            {
                continue;
            }
            ordinal += 1;
            scored.push((
                score,
                ordinal,
                SupportQuoteCandidate {
                    id: String::new(),
                    section_id: section.section_id.clone(),
                    section_hash: section.section_hash.clone(),
                    section_title: section.title.clone(),
                    quote: sentence,
                    score,
                },
            ));
        }
    }
    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored
        .into_iter()
        .take(24)
        .enumerate()
        .map(|(index, (_, _, mut candidate))| {
            candidate.id = format!("q{:03}", index + 1);
            candidate
        })
        .collect()
}

pub(crate) fn eligible_support_section(section: &PaperSection) -> bool {
    let key = format!(
        "{} {}",
        section.section_id.to_ascii_lowercase(),
        section.title.to_ascii_lowercase()
    );
    let blocked = [
        "abstract",
        "source",
        "reference",
        "bibliography",
        "acknowledg",
        "funding",
        "competing interest",
        "conflict",
        "author contribution",
        "data availability",
        "ethics",
        "publisher",
        "supplement",
        "appendix",
    ];
    !blocked.iter().any(|term| key.contains(term))
}

pub(crate) fn eligible_support_quote(
    section_title: &str,
    sentence: &str,
    min_support_quote_score: i32,
) -> bool {
    let trimmed = sentence.trim();
    let chars = trimmed.chars().count();
    let marker_count = support_quote_specificity_marker_count(trimmed);
    let clause_count = trimmed
        .chars()
        .filter(|ch| matches!(ch, ',' | ';' | ':' | '('))
        .count();
    let alphabetic_words = trimmed
        .split_whitespace()
        .filter(|word| word.chars().filter(|ch| ch.is_alphabetic()).count() >= 3)
        .count();
    let lower = trimmed.to_ascii_lowercase();
    chars >= 80
        && chars >= 120
        && chars <= 520
        && trimmed.contains(' ')
        && alphabetic_words >= 12
        && marker_count >= 2
        && clause_count >= 1
        && !trimmed.starts_with("http")
        && !lower.contains("all claims expressed")
        && !lower.starts_with("table ")
        && !lower.starts_with("figure ")
        && !is_table_or_formula_fragment(trimmed)
        && !is_protocol_recipe(trimmed)
        && support_quote_hardness_score(section_title, trimmed) >= min_support_quote_score
}

pub(crate) fn support_quote_hardness_score(section_title: &str, sentence: &str) -> i32 {
    let specificity = support_quote_score(section_title, sentence);
    let lower = sentence.to_ascii_lowercase();
    let table_penalty = i32::from(is_table_or_formula_fragment(sentence)) * 8;
    let method_penalty = i32::from(is_protocol_recipe(sentence)) * 4;
    let title_like_penalty =
        i32::from(lower.starts_with("table ") || lower.starts_with("figure ")) * 8;
    specificity - table_penalty - method_penalty - title_like_penalty
}

pub(crate) fn is_table_or_formula_fragment(text: &str) -> bool {
    let words = text.split_whitespace().count().max(1);
    let digits = text
        .split_whitespace()
        .filter(|word| word.chars().any(|ch| ch.is_ascii_digit()))
        .count();
    let symbol_count = text.chars().filter(|ch| "=<>±×/%".contains(*ch)).count();
    digits * 3 > words || symbol_count > 12
}

pub(crate) fn is_protocol_recipe(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("gradient program")
        || lower.contains("formula for calculating")
        || lower.contains("major constituents")
        || lower.contains("melting temperatures")
}

pub(crate) fn content_tokens(text: &str) -> BTreeSet<String> {
    let stopwords = [
        "the", "and", "for", "with", "that", "this", "from", "were", "was", "are", "has", "had",
        "have", "into", "between", "among", "which", "their", "using", "used", "than", "then",
        "when", "where", "what", "how", "why", "paper", "study", "result", "results",
    ];
    text.split(|ch: char| !ch.is_alphanumeric())
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| token.len() >= 4)
        .filter(|token| !stopwords.contains(&token.as_str()))
        .collect()
}

pub(crate) fn support_quote_score(section_title: &str, sentence: &str) -> i32 {
    let mut score = 0;
    let title = section_title.to_ascii_lowercase();
    for term in [
        "result",
        "results",
        "finding",
        "findings",
        "method",
        "methods",
        "discussion",
        "analysis",
        "case",
    ] {
        if title.contains(term) {
            score += 3;
            break;
        }
    }
    if sentence.chars().any(|ch| ch.is_ascii_digit()) {
        score += 4;
    }
    score += support_quote_specificity_marker_count(sentence).min(8) as i32;
    let lower = sentence.to_ascii_lowercase();
    for term in [
        "%",
        "rate",
        "ratio",
        "mean",
        "median",
        "increase",
        "decrease",
        "significant",
        "highest",
        "lowest",
        "maximum",
        "minimum",
        "identified",
        "observed",
        "measured",
        "found",
    ] {
        if lower.contains(term) {
            score += 1;
        }
    }
    score
}

pub(crate) fn support_quote_specificity_marker_count(sentence: &str) -> usize {
    let lower = sentence.to_ascii_lowercase();
    let digit_markers = sentence
        .split_whitespace()
        .filter(|part| part.chars().any(|ch| ch.is_ascii_digit()))
        .count();
    let symbol_markers = sentence
        .chars()
        .filter(|ch| {
            matches!(
                ch,
                '%' | '\u{00b1}' | '\u{00d7}' | '=' | '<' | '>' | '/' | '-'
            )
        })
        .count();
    let unit_markers = [
        " mg",
        " \u{03bc}",
        " mm",
        " cm",
        " kg",
        " wt",
        " \u{00b0}c",
        " rh",
        " ci",
        " p ",
        " fold",
        " ratio",
        " percent",
        " coefficient",
        " probability",
    ]
    .iter()
    .filter(|marker| lower.contains(**marker))
    .count();
    digit_markers + symbol_markers + unit_markers
}

pub(crate) fn exact_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut start = None;
    for (index, ch) in text.char_indices() {
        if start.is_none() && !ch.is_whitespace() {
            start = Some(index);
        }
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }
        let end = index + ch.len_utf8();
        let next_is_boundary = text[end..]
            .chars()
            .next()
            .map(|next| next.is_whitespace())
            .unwrap_or(true);
        if next_is_boundary {
            if let Some(sentence_start) = start.take() {
                let sentence = text[sentence_start..end].trim().to_string();
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }
            }
        }
    }
    if let Some(sentence_start) = start {
        let sentence = text[sentence_start..].trim().to_string();
        if !sentence.is_empty() {
            sentences.push(sentence);
        }
    }
    let mut combined = sentences.clone();
    for pair in sentences.windows(2) {
        let joined = format!("{} {}", pair[0], pair[1]);
        if joined.chars().count() <= 520 {
            combined.push(joined);
        }
    }
    combined
}
