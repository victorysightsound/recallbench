use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};

/// LoCoMo dataset (Snap Research) — long-context conversation memory.
pub struct LoCoMoDataset {
    questions: Vec<BenchmarkQuestion>,
}

#[derive(Debug, Deserialize)]
struct RawLoCoMo {
    #[serde(default)]
    conversation_id: String,
    #[serde(default)]
    conversations: Vec<RawConversation>,
    #[serde(default)]
    qa_pairs: Vec<RawQA>,
}

#[derive(Debug, Deserialize)]
struct RawConversation {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    dialogue: Vec<RawUtterance>,
}

#[derive(Debug, Deserialize)]
struct RawUtterance {
    #[serde(default)]
    speaker: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct RawQA {
    #[serde(default)]
    question_id: String,
    #[serde(default)]
    question: String,
    #[serde(default)]
    answer: String,
    #[serde(default, alias = "type")]
    question_type: String,
    #[serde(default)]
    category: Option<String>,
}

impl LoCoMoDataset {
    pub fn from_json(json: &str) -> Result<Self> {
        let raw_data: Vec<RawLoCoMo> = serde_json::from_str(json)
            .context("Failed to parse LoCoMo JSON")?;

        let mut questions = Vec::new();

        for entry in &raw_data {
            let sessions: Vec<ConversationSession> = entry.conversations.iter().map(|conv| {
                ConversationSession {
                    id: conv.session_id.clone(),
                    date: conv.date.clone(),
                    turns: conv.dialogue.iter().map(|u| Turn {
                        role: if u.speaker.to_lowercase().contains("user") { "user".to_string() } else { "assistant".to_string() },
                        content: u.text.clone(),
                    }).collect(),
                }
            }).collect();

            for qa in &entry.qa_pairs {
                let qtype = if !qa.question_type.is_empty() {
                    qa.question_type.clone()
                } else {
                    qa.category.clone().unwrap_or_else(|| "general".to_string())
                };

                questions.push(BenchmarkQuestion {
                    id: qa.question_id.clone(),
                    question_type: qtype,
                    question: qa.question.clone(),
                    ground_truth: vec![qa.answer.clone()],
                    question_date: None,
                    sessions: sessions.clone(),
                    is_abstention: false,
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
    fn description(&self) -> &str { "LoCoMo (Snap Research) — long-context conversation memory evaluation" }
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
    fn parse_locomo() {
        let json = r#"[{
            "conversation_id": "conv1",
            "conversations": [{
                "session_id": "s1",
                "date": "2024-01-15",
                "dialogue": [
                    {"speaker": "User", "text": "Hello"},
                    {"speaker": "Assistant", "text": "Hi there"}
                ]
            }],
            "qa_pairs": [{
                "question_id": "q1",
                "question": "What did the user say?",
                "answer": "Hello",
                "type": "factual"
            }]
        }]"#;
        let ds = LoCoMoDataset::from_json(json).unwrap();
        assert_eq!(ds.questions().len(), 1);
        assert_eq!(ds.questions()[0].sessions.len(), 1);
        assert_eq!(ds.questions()[0].sessions[0].turns.len(), 2);
    }
}
