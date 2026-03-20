use std::fmt;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};
use super::download::download_dataset;

/// LongMemEval question types (7 types across 5 abilities).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QuestionType {
    SingleSessionUser,
    SingleSessionAssistant,
    SingleSessionPreference,
    MultiSession,
    KnowledgeUpdate,
    TemporalReasoning,
}

impl fmt::Display for QuestionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SingleSessionUser => write!(f, "single-session-user"),
            Self::SingleSessionAssistant => write!(f, "single-session-assistant"),
            Self::SingleSessionPreference => write!(f, "single-session-preference"),
            Self::MultiSession => write!(f, "multi-session"),
            Self::KnowledgeUpdate => write!(f, "knowledge-update"),
            Self::TemporalReasoning => write!(f, "temporal-reasoning"),
        }
    }
}

/// Dataset variant sizes.
#[derive(Debug, Clone, Copy)]
pub enum Variant {
    Oracle,
    Small,
    Medium,
}

impl Variant {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "oracle" => Ok(Self::Oracle),
            "small" | "s" => Ok(Self::Small),
            "medium" | "m" => Ok(Self::Medium),
            _ => anyhow::bail!("Unknown LongMemEval variant: {s}. Use oracle, small, or medium."),
        }
    }

    pub fn filename(&self) -> &str {
        match self {
            Self::Oracle => "longmemeval_oracle.json",
            Self::Small => "longmemeval_s_cleaned.json",
            Self::Medium => "longmemeval_m_cleaned.json",
        }
    }

    pub fn url(&self) -> String {
        let base = "https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/main";
        format!("{}/{}", base, self.filename())
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Oracle => "oracle",
            Self::Small => "small",
            Self::Medium => "medium",
        }
    }
}

/// Raw question from the LongMemEval JSON format.
#[derive(Debug, Deserialize)]
struct RawQuestion {
    question_id: String,
    question_type: String,
    question: String,
    answer: Answer,
    #[serde(default)]
    question_date: Option<String>,
    #[serde(default)]
    haystack_sessions: Vec<Vec<RawTurn>>,
    #[serde(default)]
    haystack_dates: Vec<String>,
    #[serde(default)]
    haystack_session_ids: Vec<String>,
}

/// Raw turn from LongMemEval.
#[derive(Debug, Deserialize)]
struct RawTurn {
    role: String,
    content: String,
}

/// Answer can be a string, number, or array.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Answer {
    Single(String),
    Number(serde_json::Number),
    Multiple(Vec<serde_json::Value>),
}

impl Answer {
    fn as_strings(&self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s.clone()],
            Self::Number(n) => vec![n.to_string()],
            Self::Multiple(arr) => arr.iter().map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }).collect(),
        }
    }
}

/// Loaded LongMemEval dataset.
pub struct LongMemEvalDataset {
    variant: String,
    questions: Vec<BenchmarkQuestion>,
}

impl LongMemEvalDataset {
    /// Load a LongMemEval variant, downloading if necessary.
    pub async fn load(variant_str: &str, force_download: bool) -> Result<Self> {
        let variant = Variant::from_str(variant_str)?;
        let path = download_dataset(&variant.url(), variant.filename(), force_download).await?;

        let content = tokio::fs::read_to_string(&path).await
            .context("Failed to read LongMemEval dataset file")?;

        let raw_questions: Vec<RawQuestion> = serde_json::from_str(&content)
            .context("Failed to parse LongMemEval JSON")?;

        let questions = raw_questions.into_iter().map(|raw| {
            let is_abstention = raw.question_id.contains("_abs");

            let sessions: Vec<ConversationSession> = raw.haystack_sessions.iter()
                .enumerate()
                .map(|(i, turns)| {
                    ConversationSession {
                        id: raw.haystack_session_ids.get(i)
                            .cloned()
                            .unwrap_or_else(|| format!("session_{i}")),
                        date: raw.haystack_dates.get(i).cloned(),
                        turns: turns.iter().map(|t| Turn {
                            role: t.role.clone(),
                            content: t.content.clone(),
                        }).collect(),
                    }
                })
                .collect();

            BenchmarkQuestion {
                id: raw.question_id,
                question_type: raw.question_type,
                question: raw.question,
                ground_truth: raw.answer.as_strings(),
                question_date: raw.question_date,
                sessions,
                is_abstention,
                metadata: std::collections::HashMap::new(),
            }
        }).collect();

        Ok(Self {
            variant: variant.name().to_string(),
            questions,
        })
    }

    /// Parse from a JSON string (for testing without download).
    pub fn from_json(variant: &str, json: &str) -> Result<Self> {
        let raw_questions: Vec<RawQuestion> = serde_json::from_str(json)
            .context("Failed to parse LongMemEval JSON")?;

        let questions = raw_questions.into_iter().map(|raw| {
            let is_abstention = raw.question_id.contains("_abs");
            let sessions: Vec<ConversationSession> = raw.haystack_sessions.iter()
                .enumerate()
                .map(|(i, turns)| {
                    ConversationSession {
                        id: raw.haystack_session_ids.get(i)
                            .cloned()
                            .unwrap_or_else(|| format!("session_{i}")),
                        date: raw.haystack_dates.get(i).cloned(),
                        turns: turns.iter().map(|t| Turn {
                            role: t.role.clone(),
                            content: t.content.clone(),
                        }).collect(),
                    }
                })
                .collect();

            BenchmarkQuestion {
                id: raw.question_id,
                question_type: raw.question_type,
                question: raw.question,
                ground_truth: raw.answer.as_strings(),
                question_date: raw.question_date,
                sessions,
                is_abstention,
                metadata: std::collections::HashMap::new(),
            }
        }).collect();

        Ok(Self {
            variant: variant.to_string(),
            questions,
        })
    }

    /// Get question count per type.
    pub fn type_distribution(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for q in &self.questions {
            *counts.entry(q.question_type.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Total number of turns across all sessions across all questions.
    pub fn total_turns(&self) -> usize {
        self.questions.iter()
            .flat_map(|q| &q.sessions)
            .map(|s| s.turns.len())
            .sum()
    }

    /// Total number of sessions across all questions.
    pub fn total_sessions(&self) -> usize {
        self.questions.iter()
            .map(|q| q.sessions.len())
            .sum()
    }
}

impl BenchmarkDataset for LongMemEvalDataset {
    fn name(&self) -> &str {
        "longmemeval"
    }

    fn variant(&self) -> &str {
        &self.variant
    }

    fn description(&self) -> &str {
        "LongMemEval (ICLR 2025) — 500 questions testing 5 core memory abilities"
    }

    fn questions(&self) -> &[BenchmarkQuestion] {
        &self.questions
    }

    fn question_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.type_distribution().keys().cloned().collect();
        types.sort();
        types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &str = r#"[
        {
            "question_id": "q001",
            "question_type": "temporal-reasoning",
            "question": "When did the user mention pizza?",
            "answer": "March 5th",
            "question_date": "2024/03/10 (Sun) 14:30",
            "haystack_sessions": [
                [
                    {"role": "user", "content": "I had pizza on March 5th"},
                    {"role": "assistant", "content": "That sounds nice!"}
                ]
            ],
            "haystack_dates": ["2024/03/05 (Tue) 12:00"],
            "haystack_session_ids": ["session_001"]
        },
        {
            "question_id": "q002_abs",
            "question_type": "single-session-user",
            "question": "What is the user's favorite movie?",
            "answer": ["The Matrix", "Matrix"],
            "haystack_sessions": [],
            "haystack_dates": [],
            "haystack_session_ids": []
        },
        {
            "question_id": "q003",
            "question_type": "knowledge-update",
            "question": "What phone does the user have?",
            "answer": 15,
            "haystack_sessions": [],
            "haystack_dates": [],
            "haystack_session_ids": []
        }
    ]"#;

    #[test]
    fn parse_sample_json() {
        let dataset = LongMemEvalDataset::from_json("oracle", SAMPLE_JSON).unwrap();
        assert_eq!(dataset.questions().len(), 3);
    }

    #[test]
    fn question_types_detected() {
        let dataset = LongMemEvalDataset::from_json("oracle", SAMPLE_JSON).unwrap();
        let types = dataset.question_types();
        assert!(types.contains(&"temporal-reasoning".to_string()));
        assert!(types.contains(&"single-session-user".to_string()));
        assert!(types.contains(&"knowledge-update".to_string()));
    }

    #[test]
    fn abstention_detection() {
        let dataset = LongMemEvalDataset::from_json("oracle", SAMPLE_JSON).unwrap();
        let q1 = &dataset.questions()[0];
        let q2 = &dataset.questions()[1];
        assert!(!q1.is_abstention);
        assert!(q2.is_abstention); // q002_abs
    }

    #[test]
    fn answer_formats() {
        let dataset = LongMemEvalDataset::from_json("oracle", SAMPLE_JSON).unwrap();
        // String answer
        assert_eq!(dataset.questions()[0].ground_truth, vec!["March 5th"]);
        // Array answer
        assert_eq!(dataset.questions()[1].ground_truth, vec!["The Matrix", "Matrix"]);
        // Number answer
        assert_eq!(dataset.questions()[2].ground_truth, vec!["15"]);
    }

    #[test]
    fn sessions_parsed() {
        let dataset = LongMemEvalDataset::from_json("oracle", SAMPLE_JSON).unwrap();
        let q1 = &dataset.questions()[0];
        assert_eq!(q1.sessions.len(), 1);
        assert_eq!(q1.sessions[0].id, "session_001");
        assert_eq!(q1.sessions[0].turns.len(), 2);
        assert_eq!(q1.sessions[0].turns[0].role, "user");
    }

    #[test]
    fn type_distribution() {
        let dataset = LongMemEvalDataset::from_json("oracle", SAMPLE_JSON).unwrap();
        let dist = dataset.type_distribution();
        assert_eq!(dist["temporal-reasoning"], 1);
        assert_eq!(dist["single-session-user"], 1);
        assert_eq!(dist["knowledge-update"], 1);
    }

    #[test]
    fn stats() {
        let dataset = LongMemEvalDataset::from_json("oracle", SAMPLE_JSON).unwrap();
        assert_eq!(dataset.total_sessions(), 1);
        assert_eq!(dataset.total_turns(), 2);
    }

    #[test]
    fn dataset_trait() {
        let dataset = LongMemEvalDataset::from_json("oracle", SAMPLE_JSON).unwrap();
        assert_eq!(dataset.name(), "longmemeval");
        assert_eq!(dataset.variant(), "oracle");
        assert!(!dataset.description().is_empty());
    }

    #[test]
    fn variant_parsing() {
        assert!(matches!(Variant::from_str("oracle").unwrap(), Variant::Oracle));
        assert!(matches!(Variant::from_str("small").unwrap(), Variant::Small));
        assert!(matches!(Variant::from_str("s").unwrap(), Variant::Small));
        assert!(matches!(Variant::from_str("medium").unwrap(), Variant::Medium));
        assert!(matches!(Variant::from_str("m").unwrap(), Variant::Medium));
        assert!(Variant::from_str("unknown").is_err());
    }
}
