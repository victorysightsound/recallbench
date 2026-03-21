use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};
use super::download::download_dataset;

const BASE_URL: &str = "https://huggingface.co/datasets/Salesforce/ConvoMem/resolve/main/core_benchmark/pre_mixed_testcases";

/// ConvoMem categories with their download paths.
const CATEGORIES: &[(&str, &str)] = &[
    ("user_evidence", "user_evidence/1_evidence/batched_000.json"),
    ("assistant_facts", "assistant_facts_evidence/1_evidence/batched_000.json"),
    ("changing", "changing_evidence/2_evidence/batched_000.json"),
    ("abstention", "abstention_evidence/1_evidence/batched_000.json"),
    ("preference", "preference_evidence/1_evidence/batched_000.json"),
    ("implicit_connection", "implicit_connection_evidence/1_evidence/batched_000.json"),
];

/// ConvoMem dataset (Salesforce AI Research).
pub struct ConvoMemDataset {
    questions: Vec<BenchmarkQuestion>,
    category: String,
}

#[derive(Debug, Deserialize)]
struct RawTestCase {
    #[serde(rename = "evidenceItems")]
    evidence_items: Vec<RawEvidenceItem>,
}

#[derive(Debug, Deserialize)]
struct RawEvidenceItem {
    question: String,
    answer: String,
    #[serde(default, rename = "message_evidences")]
    message_evidences: Vec<RawMessage>,
    #[serde(default)]
    conversations: Vec<RawConversation>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    #[serde(default)]
    speaker: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct RawConversation {
    #[serde(default)]
    messages: Vec<RawMessage>,
}

impl ConvoMemDataset {
    /// Load a specific category.
    pub async fn load(category: &str, force_download: bool) -> Result<Self> {
        let (cat_name, path) = CATEGORIES.iter()
            .find(|(name, _)| *name == category)
            .ok_or_else(|| anyhow::anyhow!("Unknown ConvoMem category: {category}. Available: {}",
                CATEGORIES.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")))?;

        let url = format!("{BASE_URL}/{path}");
        let filename = format!("convomem_{cat_name}.json");
        let file_path = download_dataset(&url, &filename, force_download).await?;
        let content = tokio::fs::read_to_string(&file_path).await?;
        Self::from_json(cat_name, &content)
    }

    pub fn from_json(category: &str, json: &str) -> Result<Self> {
        let test_cases: Vec<RawTestCase> = serde_json::from_str(json)
            .context("Failed to parse ConvoMem JSON")?;

        let mut questions = Vec::new();
        let mut qi = 0;

        for tc in &test_cases {
            for item in &tc.evidence_items {
                let sessions: Vec<ConversationSession> = item.conversations.iter()
                    .enumerate()
                    .map(|(i, conv)| ConversationSession {
                        id: format!("conv_{i}"),
                        date: None,
                        turns: conv.messages.iter().map(|m| Turn {
                            role: if m.speaker == "user" { "user".to_string() } else { "assistant".to_string() },
                            content: m.text.clone(),
                        }).collect(),
                    })
                    .collect();

                questions.push(BenchmarkQuestion {
                    id: format!("convomem_{category}_{qi}"),
                    question_type: category.to_string(),
                    question: item.question.clone(),
                    ground_truth: vec![item.answer.clone()],
                    question_date: None,
                    sessions,
                    is_abstention: category == "abstention",
                    metadata: std::collections::HashMap::new(),
                });
                qi += 1;
            }
        }

        Ok(Self { questions, category: category.to_string() })
    }
}

impl BenchmarkDataset for ConvoMemDataset {
    fn name(&self) -> &str { "convomem" }
    fn variant(&self) -> &str { &self.category }
    fn description(&self) -> &str { "ConvoMem (Salesforce) — conversational memory evaluation with 6 evidence categories" }
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
    fn parse_convomem_format() {
        let json = r#"[{
            "evidenceItems": [{
                "question": "What trivia category did my team lose on?",
                "answer": "1980s one-hit wonders",
                "message_evidences": [{"speaker": "user", "text": "We lost on 1980s one-hit wonders"}],
                "conversations": [{
                    "messages": [
                        {"speaker": "user", "text": "Hey, trivia night was rough"},
                        {"speaker": "assistant", "text": "Oh no, what happened?"}
                    ]
                }]
            }]
        }]"#;
        let ds = ConvoMemDataset::from_json("user_evidence", json).unwrap();
        assert_eq!(ds.questions().len(), 1);
        assert_eq!(ds.questions()[0].question_type, "user_evidence");
        assert!(!ds.questions()[0].is_abstention);
    }

    #[test]
    fn abstention_category() {
        let json = r#"[{"evidenceItems": [{"question": "Q?", "answer": "A", "conversations": []}]}]"#;
        let ds = ConvoMemDataset::from_json("abstention", json).unwrap();
        assert!(ds.questions()[0].is_abstention);
    }
}
