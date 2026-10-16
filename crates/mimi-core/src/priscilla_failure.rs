//! Priscilla T3.6 — Pandora failure pattern integration (read-only).

use chrono::{DateTime, Utc};

use crate::pandora_client::FailurePattern;
use crate::priscilla::TaskDraftMessage;

const HALF_LIFE_HOURS: f64 = 48.0;

/// Exponential decay: recent failures weigh more (T3.6 temporal weighting).
pub fn temporal_weight(last: DateTime<Utc>, now: DateTime<Utc>) -> f32 {
    let hours = (now - last).num_seconds().max(0) as f64 / 3600.0;
    let w = 0.5f64.powf(hours / HALF_LIFE_HOURS);
    w.clamp(0.05, 1.0) as f32
}

/// Token overlap between intent and pattern label (baseline similarity).
pub fn intent_similarity(intent: &str, pattern_type: &str) -> f32 {
    let intent_tokens: std::collections::HashSet<String> = intent
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .map(|w| w.to_string())
        .collect();
    if intent_tokens.is_empty() {
        return 0.0;
    }
    let pattern_tokens: std::collections::HashSet<String> = pattern_type
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || c == '_' || c == '-')
        .filter(|w| w.len() > 2)
        .map(|w| w.to_string())
        .collect();
    if pattern_tokens.is_empty() {
        return 0.0;
    }
    let overlap = intent_tokens.intersection(&pattern_tokens).count();
    (overlap as f32 / intent_tokens.len() as f32).clamp(0.0, 1.0)
}

pub struct PatternInsight {
    pub concern: String,
    pub weight: f32,
}

/// Rank patterns against a draft intent; returns advisory concerns for Priscilla.
pub fn insights_for_draft(
    draft: &TaskDraftMessage,
    patterns: &[FailurePattern],
    now: DateTime<Utc>,
) -> Vec<PatternInsight> {
    let mut out = Vec::new();
    for p in patterns {
        let sim = intent_similarity(&draft.intent, &p.pattern_type);
        if sim < 0.15 && p.frequency < 3 {
            continue;
        }
        let age_w = temporal_weight(p.last_occurrence, now);
        let score = sim * age_w * (1.0 + (p.frequency as f32 * 0.1));
        if score < 0.12 {
            continue;
        }
        let states = if p.states_involved.is_empty() {
            String::new()
        } else {
            format!(" (states: {})", p.states_involved.join(", "))
        };
        out.push(PatternInsight {
            concern: format!(
                "Pandora failure pattern \"{}\" — {} occurrence(s) in window{}",
                p.pattern_type,
                p.frequency,
                states
            ),
            weight: score,
        });
    }
    out.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(5);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_failure_weighs_more() {
        let now = Utc::now();
        let recent = temporal_weight(now - chrono::Duration::hours(1), now);
        let old = temporal_weight(now - chrono::Duration::hours(200), now);
        assert!(recent > old);
    }

    #[test]
    fn overlap_detects_shared_tokens() {
        let sim = intent_similarity("deploy api service", "deploy_timeout");
        assert!(sim > 0.0);
    }
}
