use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};

/// MemoryAgentBench dataset (ICLR 2026) — selective forgetting + fact consolidation.
pub struct MemoryAgentBenchDataset {
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
    #[serde(default, alias = "type", alias = "task_type")]
    question_type: String,
    #[serde(default)]
    interactions: Vec<RawInteraction>,
}

#[derive(Debug, Deserialize)]
struct RawInteraction {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    messages: Vec<RawMessage>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    #[serde(default)]
    role: String,
    #[serde(default)]
    content: String,
}

impl MemoryAgentBenchDataset {
    pub fn from_json(json: &str) -> Result<Self> {
        let raw: Vec<RawEntry> = serde_json::from_str(json)
            .context("Failed to parse MemoryAgentBench JSON")?;

        let questions = raw.into_iter().map(|entry| {
            let ground_truth = match entry.answer {
                serde_json::Value::String(s) => vec![s],
                serde_json::Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                other => vec![other.to_string()],
            };

            let sessions: Vec<ConversationSession> = entry.interactions.iter()
                .enumerate()
                .map(|(i, interaction)| ConversationSession {
                    id: interaction.session_id.clone().unwrap_or_else(|| format!("interaction_{i}")),
                    date: None,
                    turns: interaction.messages.iter().map(|m| Turn {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    }).collect(),
                })
                .collect();

            BenchmarkQuestion {
                id: entry.id,
                question_type: if entry.question_type.is_empty() { "event-qa".to_string() } else { entry.question_type },
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

impl BenchmarkDataset for MemoryAgentBenchDataset {
    fn name(&self) -> &str { "memoryagentbench" }
    fn variant(&self) -> &str { "default" }
    fn description(&self) -> &str { "MemoryAgentBench (ICLR 2026) — selective forgetting and fact consolidation" }
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
    fn parse_mab() {
        let json = r#"[{
            "id": "mab1",
            "question": "What event happened first?",
            "answer": "meeting",
            "task_type": "event-qa",
            "interactions": [{
                "session_id": "s1",
                "messages": [
                    {"role": "user", "content": "I had a meeting today"},
                    {"role": "assistant", "content": "Got it"}
                ]
            }]
        }]"#;
        let ds = MemoryAgentBenchDataset::from_json(json).unwrap();
        assert_eq!(ds.questions().len(), 1);
        assert_eq!(ds.questions()[0].question_type, "event-qa");
    }
}
