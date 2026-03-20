use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};

/// ConvoMem dataset (used by memorybench/Supermemory).
pub struct ConvoMemDataset {
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
    #[serde(default, alias = "type")]
    question_type: String,
    #[serde(default)]
    conversations: Vec<Vec<RawMessage>>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    #[serde(default)]
    role: String,
    #[serde(default)]
    content: String,
}

impl ConvoMemDataset {
    pub fn from_json(json: &str) -> Result<Self> {
        let raw: Vec<RawEntry> = serde_json::from_str(json)
            .context("Failed to parse ConvoMem JSON")?;

        let questions = raw.into_iter().map(|entry| {
            let ground_truth = match entry.answer {
                serde_json::Value::String(s) => vec![s],
                serde_json::Value::Array(arr) => arr.iter().map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                }).collect(),
                other => vec![other.to_string()],
            };

            let sessions: Vec<ConversationSession> = entry.conversations.iter()
                .enumerate()
                .map(|(i, conv)| ConversationSession {
                    id: format!("session_{i}"),
                    date: None,
                    turns: conv.iter().map(|m| Turn {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    }).collect(),
                })
                .collect();

            BenchmarkQuestion {
                id: entry.id,
                question_type: if entry.question_type.is_empty() { "general".to_string() } else { entry.question_type },
                question: entry.question,
                ground_truth,
                question_date: None,
                sessions,
                is_abstention: false,
                metadata: std::collections::HashMap::new(),
            }
        }).collect();

        Ok(Self { questions })
    }
}

impl BenchmarkDataset for ConvoMemDataset {
    fn name(&self) -> &str { "convomem" }
    fn variant(&self) -> &str { "default" }
    fn description(&self) -> &str { "ConvoMem — conversational memory evaluation (memorybench)" }
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
    fn parse_convomem() {
        let json = r#"[{
            "id": "q1",
            "question": "What is the user's name?",
            "answer": "John",
            "type": "recall",
            "conversations": [[
                {"role": "user", "content": "My name is John"},
                {"role": "assistant", "content": "Nice to meet you, John!"}
            ]]
        }]"#;
        let ds = ConvoMemDataset::from_json(json).unwrap();
        assert_eq!(ds.questions().len(), 1);
        assert_eq!(ds.questions()[0].ground_truth, vec!["John"]);
    }
}
