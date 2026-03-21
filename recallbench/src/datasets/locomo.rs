use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};
use super::download::download_dataset;

const DOWNLOAD_URL: &str = "https://raw.githubusercontent.com/snap-research/locomo/main/data/locomo10.json";
const FILENAME: &str = "locomo10.json";

/// LoCoMo dataset (Snap Research, ACL 2024).
/// 10 conversations, 1,986 QA pairs across 5 categories.
pub struct LoCoMoDataset {
    questions: Vec<BenchmarkQuestion>,
}

#[derive(Debug, Deserialize)]
struct RawSample {
    sample_id: String,
    conversation: HashMap<String, serde_json::Value>,
    qa: Vec<RawQA>,
}

#[derive(Debug, Deserialize)]
struct RawQA {
    question: String,
    #[serde(default)]
    answer: serde_json::Value, // Can be string, integer, or missing
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default)]
    category: u32,
}

#[derive(Debug, Deserialize)]
struct RawUtterance {
    #[serde(default)]
    speaker: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    dia_id: String,
}

impl LoCoMoDataset {
    pub async fn load(force_download: bool) -> Result<Self> {
        let path = download_dataset(DOWNLOAD_URL, FILENAME, force_download).await?;
        let content = tokio::fs::read_to_string(&path).await?;
        Self::from_json(&content)
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let samples: Vec<RawSample> = serde_json::from_str(json)
            .context("Failed to parse LoCoMo JSON")?;

        let mut questions = Vec::new();

        for sample in &samples {
            // Extract sessions from conversation map (keys like "session_1", "session_2", etc.)
            let mut sessions = Vec::new();
            let mut session_keys: Vec<String> = sample.conversation.keys()
                .filter(|k| k.starts_with("session_"))
                .cloned()
                .collect();
            session_keys.sort();

            // Also grab date keys
            let date_keys: HashMap<String, String> = sample.conversation.iter()
                .filter(|(k, _)| k.contains("date_time"))
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect();

            for session_key in &session_keys {
                if let Some(turns_val) = sample.conversation.get(session_key) {
                    let turns: Vec<RawUtterance> = serde_json::from_value(turns_val.clone())
                        .unwrap_or_default();

                    // Find matching date
                    let date_key = format!("{}_date_time", session_key);
                    let date = date_keys.get(&date_key).cloned();

                    sessions.push(ConversationSession {
                        id: format!("{}_{}", sample.sample_id, session_key),
                        date,
                        turns: turns.iter().map(|u| Turn {
                            role: if u.speaker.contains("speaker_a") || u.speaker.contains("Speaker A") {
                                "user".to_string()
                            } else {
                                "assistant".to_string()
                            },
                            content: u.text.clone(),
                        }).collect(),
                    });
                }
            }

            // Convert QA pairs to BenchmarkQuestions
            for (qi, qa) in sample.qa.iter().enumerate() {
                let qtype = match qa.category {
                    1 => "single-hop",
                    2 => "temporal",
                    3 => "multi-hop",
                    4 => "open-domain",
                    5 => "unanswerable",
                    _ => "unknown",
                };

                let answer_str = match &qa.answer {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    other => other.to_string(),
                };

                questions.push(BenchmarkQuestion {
                    id: format!("{}_q{}", sample.sample_id, qi),
                    question_type: qtype.to_string(),
                    question: qa.question.clone(),
                    ground_truth: vec![answer_str],
                    question_date: None,
                    sessions: sessions.clone(),
                    is_abstention: qa.category == 5,
                    metadata: std::collections::HashMap::new(),
                });
            }
        }

        Ok(Self { questions })
    }
}

impl BenchmarkDataset for LoCoMoDataset {
    fn name(&self) -> &str { "locomo" }
    fn variant(&self) -> &str { "default" }
    fn description(&self) -> &str { "LoCoMo (Snap Research) — 1,986 QA pairs across 10 long conversations, 5 categories" }
    fn questions(&self) -> &[BenchmarkQuestion] { &self.questions }
    fn question_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.questions.iter()
            .map(|q| q.question_type.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter().collect();
        types.sort();
        types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_mapping() {
        // Verify all 5 categories map correctly
        for (cat, expected) in [(1, "single-hop"), (2, "temporal"), (3, "multi-hop"), (4, "open-domain"), (5, "unanswerable")] {
            let json = format!(r#"[{{
                "sample_id": "test",
                "conversation": {{}},
                "qa": [{{"question": "Q?", "answer": "A", "category": {cat}}}]
            }}]"#);
            let ds = LoCoMoDataset::from_json(&json).unwrap();
            assert_eq!(ds.questions()[0].question_type, expected);
            if cat == 5 {
                assert!(ds.questions()[0].is_abstention);
            }
        }
    }
}
