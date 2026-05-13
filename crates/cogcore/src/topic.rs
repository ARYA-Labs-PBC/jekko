//! Topic strength formula.

use crate::concept::{Topic, TopicStats};
use crate::config::{
    TOPIC_CONTRADICTION_WEIGHT, TOPIC_NOVELTY_WEIGHT, TOPIC_RECALL_SUCCESS_WEIGHT,
    TOPIC_RECENCY_WEIGHT, TOPIC_RECURRENCE_WEIGHT, TOPIC_SOURCE_QUALITY_WEIGHT,
    TOPIC_UTILITY_WEIGHT,
};
use crate::fsrs::{decay, hours_between, ln1p, topic_half_life_hours};

/// Recompute `topic.strength` and `topic.half_life_hours` from current stats.
pub fn recompute(topic: &mut Topic, now: &str) {
    let dt_h = hours_between(&topic.last_update_tx, now);
    let half_life = topic.half_life_hours.max(1.0);
    let decayed_base = decay(topic.strength, dt_h, half_life);

    let stats = &topic.stats;
    let recency = (stats.recent_observes as f32 / 30.0).clamp(0.0, 1.0);
    let recurrence = (ln1p(stats.distinct_subjects as f32) / 5.0).clamp(0.0, 1.0);
    let utility =
        stats.success_count as f32 / (stats.success_count + stats.failure_count + 1) as f32;
    let novelty = (stats.recent_observes as f32 / 10.0).clamp(0.0, 1.0);
    let src_q = stats.avg_source_quality.clamp(0.0, 1.0);
    let retr_succ = stats.recall_count as f32 / (stats.recall_count + 1) as f32;
    let pressure = TOPIC_CONTRADICTION_WEIGHT * topic.contradiction_pressure.clamp(0.0, 1.0);

    let s = decayed_base
        + TOPIC_RECENCY_WEIGHT * recency
        + TOPIC_RECURRENCE_WEIGHT * recurrence
        + TOPIC_UTILITY_WEIGHT * utility
        + TOPIC_NOVELTY_WEIGHT * novelty
        + TOPIC_SOURCE_QUALITY_WEIGHT * src_q
        + TOPIC_RECALL_SUCCESS_WEIGHT * retr_succ
        - pressure;

    topic.strength = s.clamp(0.0, 1.0);
    topic.half_life_hours = topic_half_life_hours(
        topic.strength,
        stats.success_count,
        stats.failure_count,
        stats.recall_count,
    );
    topic.last_update_tx = now.to_string();
}

pub fn empty_stats() -> TopicStats {
    TopicStats::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concept::Topic;

    fn topic_with(success: u32, failure: u32, recall: u32) -> Topic {
        Topic {
            id: 0,
            label: "neutrino-physics".to_string(),
            concepts: Vec::new(),
            strength: 0.5,
            half_life_hours: 24.0,
            last_update_tx: "2026-05-12T00:00:00Z".to_string(),
            contradiction_pressure: 0.0,
            stats: TopicStats {
                recall_count: recall,
                success_count: success,
                failure_count: failure,
                recent_observes: 5,
                distinct_subjects: 10,
                avg_source_quality: 0.9,
            },
        }
    }

    #[test]
    fn high_success_increases_strength() {
        let mut hi = topic_with(20, 0, 30);
        let mut lo = topic_with(0, 20, 30);
        let now = "2026-05-12T01:00:00Z";
        recompute(&mut hi, now);
        recompute(&mut lo, now);
        assert!(hi.strength > lo.strength);
    }

    #[test]
    fn contradiction_pressure_reduces_strength() {
        let mut clean = topic_with(10, 0, 10);
        let mut pressured = topic_with(10, 0, 10);
        pressured.contradiction_pressure = 0.6;
        let now = "2026-05-12T01:00:00Z";
        recompute(&mut clean, now);
        recompute(&mut pressured, now);
        assert!(clean.strength > pressured.strength);
    }
}
