use super::*;

pub(crate) fn fill_acceptance_metrics(challenge: &mut ChallengeRecord) {
    if challenge.acceptance_metrics.is_some() {
        return;
    }
    let saturated_mean_confidence = if challenge.saturated_blind_trials.is_empty() {
        1.0
    } else {
        challenge
            .saturated_blind_trials
            .iter()
            .map(|trial| trial.confidence)
            .sum::<f64>()
            / challenge.saturated_blind_trials.len() as f64
    };
    let support_minimality = if challenge.support.len() <= 2 {
        1.0
    } else {
        0.75
    };
    let distractor_pressure = if challenge.context_pack.distractor_section_ids.is_empty() {
        0.0
    } else {
        (challenge.context_pack.distractor_section_ids.len() as f64
            / (challenge.context_pack.target_section_ids.len()
                + challenge.context_pack.distractor_section_ids.len()) as f64)
            .min(1.0)
    };
    challenge.acceptance_metrics = Some(AcceptanceMetrics {
        focused_agreement: challenge.acceptance.auditor_agreement,
        focused_correct_rate: challenge.acceptance.focused_correct_rate,
        answerability: challenge.acceptance.answerability,
        saturated_blind_correct_rate: challenge.acceptance.blind_correct_rate,
        saturated_mean_confidence,
        support_minimality,
        distractor_pressure,
    });
}

pub(crate) fn fill_difficulty_score(challenge: &mut ChallengeRecord) {
    let Some(metrics) = challenge.acceptance_metrics.as_ref() else {
        return;
    };
    let blind_failure_rate = (1.0 - metrics.saturated_blind_correct_rate).clamp(0.0, 1.0);
    let low_confidence = (1.0 - metrics.saturated_mean_confidence).clamp(0.0, 1.0);
    let focused_agreement = metrics.focused_agreement.clamp(0.0, 1.0);
    let support_minimality = metrics.support_minimality.clamp(0.0, 1.0);
    let distractor_pressure = metrics.distractor_pressure.clamp(0.0, 1.0);
    let score = blind_failure_rate * 0.35
        + low_confidence * 0.20
        + focused_agreement * 0.20
        + support_minimality * 0.15
        + distractor_pressure * 0.10;
    challenge.difficulty_score = score.clamp(0.0, 1.0);
    challenge.difficulty_components = BTreeMap::from([
        ("blind_failure_rate".to_string(), blind_failure_rate),
        ("low_confidence".to_string(), low_confidence),
        ("focused_agreement".to_string(), focused_agreement),
        ("support_minimality".to_string(), support_minimality),
        ("distractor_pressure".to_string(), distractor_pressure),
    ]);
}

pub fn cogcore_events_for_papers(
    papers: &[PaperRecord],
    challenges: &[ChallengeRecord],
) -> Vec<CogcoreEventRecord> {
    let mut sorted_papers = papers.to_vec();
    sorted_papers.sort_by(|left, right| left.publication_hash.cmp(&right.publication_hash));
    let accepted_challenges = challenges
        .iter()
        .filter(|challenge| acceptance_passes(&challenge.acceptance))
        .collect::<Vec<_>>();
    let mut topics_by_section: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for challenge in accepted_challenges {
        for support in &challenge.support {
            let key = (
                challenge.publication_hash.clone(),
                support.section_id.clone(),
            );
            let topics = topics_by_section.entry(key).or_default();
            for topic in &challenge.topics {
                if !topics.contains(topic) {
                    topics.push(topic.clone());
                }
            }
            topics.sort();
        }
    }

    let mut out = Vec::new();
    for paper in &sorted_papers {
        for section in &paper.sections {
            let topics = topics_by_section
                .get(&(paper.publication_hash.clone(), section.section_id.clone()))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            out.push(cogcore_section_event(paper, section, topics));
        }
    }
    out
}

fn cogcore_section_event(
    paper: &PaperRecord,
    section: &PaperSection,
    topics: &[String],
) -> CogcoreEventRecord {
    let tx_time = match paper.published_at.clone() {
        Some(value) => value,
        None => "1970-01-01T00:00:00Z".to_string(),
    };
    let mut tags = vec![
        "qbank".to_string(),
        "paper-section".to_string(),
        format!("publication:{}", paper.publication_hash),
        format!("section:{}", section.section_id),
        format!("section_hash:{}", section.section_hash),
    ];
    tags.extend(topics.iter().map(|topic| format!("topic:{topic}")));
    CogcoreEventRecord {
        id: String::new(),
        kind: "Claim".to_string(),
        subject: paper.title.clone(),
        body: section.text.clone(),
        tx_time: tx_time.clone(),
        valid_from: Some(tx_time),
        valid_to: None,
        privacy_class: "Public".to_string(),
        claim_modality: Some("AssertedBySource".to_string()),
        tags,
        sources: vec![paper_source_ref(paper, section)],
        supersedes: Vec::new(),
        contradicts: Vec::new(),
    }
}

fn paper_source_ref(paper: &PaperRecord, section: &PaperSection) -> CogcoreSourceRef {
    let uri = match paper.license.source_url.clone() {
        Some(value) => value,
        None => format!(
            "qbank://paper/{}/{}",
            paper.publication_hash, section.section_id
        ),
    };
    CogcoreSourceRef {
        uri,
        citation: format!("{} :: {}", paper.title, section.title),
        quality: 0.95,
    }
}
