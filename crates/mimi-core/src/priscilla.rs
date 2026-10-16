//! Priscilla advisory layer (M3 / T3.5+ T3.6) — critique without veto.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::pandora_client::{FailurePattern, PandoraClient};
use crate::priscilla_failure::insights_for_draft;

/// Incoming task draft for review (Zenoh / Beatrice pipeline).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskDraftMessage {
    pub task_id: String,
    pub intent: String,
    pub estimated_tokens: u32,
    pub skill_id: Option<String>,
}

/// Advisory commentary (never blocks execution).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CritiqueCommentaryMessage {
    pub task_id: String,
    pub necessity_score: f32,
    pub cost_score: f32,
    pub concerns: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PriscillaConfig {
    /// 0.0 = permissive, 1.0 = maximum skepticism
    pub cynicism: f32,
}

impl Default for PriscillaConfig {
    fn default() -> Self {
        Self { cynicism: 0.5 }
    }
}

pub struct PriscillaAdvisor {
    config: PriscillaConfig,
}

impl PriscillaAdvisor {
    pub fn new(config: PriscillaConfig) -> Self {
        Self { config }
    }

    pub fn critique(&self, draft: &TaskDraftMessage) -> CritiqueCommentaryMessage {
        let mut concerns = Vec::new();
        let mut suggestions = Vec::new();

        let necessity = analyze_necessity(&draft.intent, self.config.cynicism, &mut concerns);
        let cost = analyze_cost(draft.estimated_tokens, self.config.cynicism, &mut concerns, &mut suggestions);

        if draft.skill_id.is_none() {
            suggestions.push("Consider reusing an existing skill from Pandora before creating a new path.".into());
        }

        CritiqueCommentaryMessage {
            task_id: draft.task_id.clone(),
            necessity_score: necessity,
            cost_score: cost,
            concerns,
            suggestions,
        }
    }

    /// T3.6 — enrich critique with Pandora failure patterns (read-only).
    pub async fn critique_with_pandora(
        &self,
        draft: &TaskDraftMessage,
        pandora: &PandoraClient,
        window_hours: u32,
    ) -> CritiqueCommentaryMessage {
        let patterns = pandora
            .query_failure_patterns(window_hours)
            .await
            .unwrap_or_default();
        self.critique_with_patterns(draft, &patterns)
    }

    pub fn critique_with_patterns(
        &self,
        draft: &TaskDraftMessage,
        patterns: &[FailurePattern],
    ) -> CritiqueCommentaryMessage {
        let mut msg = self.critique(draft);
        for insight in insights_for_draft(draft, patterns, Utc::now()) {
            if !msg.concerns.contains(&insight.concern) {
                msg.concerns.push(insight.concern);
            }
            msg.necessity_score -= 0.05 * insight.weight;
        }
        msg.necessity_score = msg.necessity_score.clamp(0.0, 1.0);
        msg
    }
}

fn analyze_necessity(intent: &str, cynicism: f32, concerns: &mut Vec<String>) -> f32 {
    let words: Vec<&str> = intent.split_whitespace().collect();
    let mut score = 1.0f32;
    if words.len() < 3 {
        concerns.push("Intent is very short — clarify desired outcome.".into());
        score -= 0.2;
    }
    let lower = intent.to_lowercase();
    if lower.contains("again") || lower.contains("retry") || lower.contains("loop") {
        concerns.push("Possible redundant or looped work detected.".into());
        score -= 0.25 * cynicism;
    }
    score.clamp(0.0, 1.0)
}

fn analyze_cost(
    tokens: u32,
    cynicism: f32,
    concerns: &mut Vec<String>,
    suggestions: &mut Vec<String>,
) -> f32 {
    let mut score = 1.0f32;
    if tokens > 8_000 {
        concerns.push(format!("High token budget ({}).", tokens));
        score -= 0.3 * cynicism;
        suggestions.push("Split the task or cache context in Pandora L1.".into());
    } else if tokens > 2_000 {
        score -= 0.1 * cynicism;
    }
    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_redundant_language() {
        let advisor = PriscillaAdvisor::new(PriscillaConfig::default());
        let msg = advisor.critique(&TaskDraftMessage {
            task_id: "t1".into(),
            intent: "retry again deploy again".into(),
            estimated_tokens: 100,
            skill_id: None,
        });
        assert!(msg.necessity_score < 1.0);
        assert!(!msg.concerns.is_empty());
    }
}
