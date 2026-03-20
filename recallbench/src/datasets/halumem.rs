use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};

/// HaluMem dataset (MemTensor) — memory hallucination detection.
pub struct HaluMemDataset {
    variant: String,
    questions: Vec<BenchmarkQuestion>,
}

#[derive(Debug, Deserialize)]
struct RawEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    question: String,
    #[serde(default)]
    answer: serde_json::Value,
    #[serde(default, alias = "type", alias = "operation_type")]
    question_type: String,
    #[serde(default)]
    memory_points: Vec<RawMemoryPoint>,
    #[serde(default)]
    context: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMemoryPoint {
    #[serde(default)]
    content: String,
    #[serde(default)]
    timestamp: Option<String>,
}

impl HaluMemDataset {
    pub fn from_json(variant: &str, json: &str) -> Result<Self> {
        let raw: Vec<RawEntry> = serde_json::from_str(json)
            .context("Failed to parse HaluMem JSON")?;

        let questions = raw.into_iter().map(|entry| {
            let ground_truth = match entry.answer {
                serde_json::Value::String(s) => vec![s],
                serde_json::Value::Bool(b) => vec![b.to_string()],
                serde_json::Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                other => vec![other.to_string()],
            };

            let mut sessions = Vec::new();
            if !entry.memory_points.is_empty() {
                sessions.push(ConversationSession {
                    id: "memories".to_string(),
                    date: None,
                    turns: entry.memory_points.iter().map(|mp| Turn {
                        role: "user".to_string(),
                        content: mp.content.clone(),
                    }).collect(),
                });
            }
            if let Some(ctx) = &entry.context {
                sessions.push(ConversationSession {
                    id: "context".to_string(),
                    date: None,
                    turns: vec![Turn { role: "user".to_string(), content: ctx.clone() }],
                });
            }

            BenchmarkQuestion {
                id: entry.id,
                question_type: if entry.question_type.is_empty() { "hallucination-detection".to_string() } else { entry.question_type },
                question: entry.question,
                ground_truth,
                question_date: None,
                sessions,
                is_abstention: false,
                metadata: std::collections::HashMap::new(),
            }
        }).collect();

        Ok(Self { variant: variant.to_string(), questions })
    }
}

impl BenchmarkDataset for HaluMemDataset {
    fn name(&self) -> &str { "halumem" }
    fn variant(&self) -> &str { &self.variant }
    fn description(&self) -> &str { "HaluMem (MemTensor) — memory hallucination detection" }
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
    fn parse_halumem() {
        let json = r#"[{
            "id": "hm1",
            "question": "Is this memory accurate?",
            "answer": "no",
            "operation_type": "extraction",
            "memory_points": [
                {"content": "User likes pizza", "timestamp": "2024-01-15"}
            ]
        }]"#;
        let ds = HaluMemDataset::from_json("medium", json).unwrap();
        assert_eq!(ds.questions().len(), 1);
        assert_eq!(ds.questions()[0].question_type, "extraction");
        assert_eq!(ds.variant(), "medium");
    }
}
