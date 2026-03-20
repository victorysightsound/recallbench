use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};

/// MemBench dataset (ACL 2025) — multi-aspect memory evaluation.
pub struct MemBenchDataset {
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
    #[serde(default, alias = "type", alias = "category")]
    question_type: String,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    scenario: Option<String>,
    #[serde(default)]
    conversations: Vec<Vec<RawMessage>>,
    #[serde(default)]
    context: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    #[serde(default)]
    role: String,
    #[serde(default)]
    content: String,
}

impl MemBenchDataset {
    pub fn from_json(json: &str) -> Result<Self> {
        let raw: Vec<RawEntry> = serde_json::from_str(json)
            .context("Failed to parse MemBench JSON")?;

        let questions = raw.into_iter().map(|entry| {
            let ground_truth = match entry.answer {
                serde_json::Value::String(s) => vec![s],
                serde_json::Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                other => vec![other.to_string()],
            };

            let mut sessions: Vec<ConversationSession> = entry.conversations.iter()
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

            // If context is provided as string instead of conversations
            if sessions.is_empty() {
                if let Some(ctx) = &entry.context {
                    sessions.push(ConversationSession {
                        id: "context_0".to_string(),
                        date: None,
                        turns: vec![Turn { role: "user".to_string(), content: ctx.clone() }],
                    });
                }
            }

            let qtype = if entry.question_type.is_empty() {
                "general".to_string()
            } else {
                let mut t = entry.question_type;
                if let Some(level) = entry.level {
                    t = format!("{t}-{level}");
                }
                t
            };

            BenchmarkQuestion {
                id: entry.id,
                question_type: qtype,
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

impl BenchmarkDataset for MemBenchDataset {
    fn name(&self) -> &str { "membench" }
    fn variant(&self) -> &str { "default" }
    fn description(&self) -> &str { "MemBench (ACL 2025) — multi-aspect memory evaluation" }
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
    fn parse_membench() {
        let json = r#"[{
            "id": "mb1",
            "question": "What did the user prefer?",
            "answer": "coffee",
            "type": "effectiveness",
            "level": "factual",
            "conversations": [[
                {"role": "user", "content": "I prefer coffee over tea"},
                {"role": "assistant", "content": "Noted!"}
            ]]
        }]"#;
        let ds = MemBenchDataset::from_json(json).unwrap();
        assert_eq!(ds.questions().len(), 1);
        assert_eq!(ds.questions()[0].question_type, "effectiveness-factual");
    }
}
