use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};
use super::download::download_dataset;

const BASE_URL: &str = "https://raw.githubusercontent.com/import-myself/Membench/main/MemData/FirstAgent";

/// MemBench categories available for download.
const CATEGORIES: &[&str] = &[
    "simple", "aggregative", "comparative", "conditional",
    "knowledge_update", "highlevel", "noisy", "post_processing",
];

/// MemBench dataset (ACL 2025).
/// Multi-aspect memory evaluation with multiple-choice QA.
pub struct MemBenchDataset {
    category: String,
    questions: Vec<BenchmarkQuestion>,
}

#[derive(Debug, Deserialize)]
struct RawFile {
    #[serde(default)]
    roles: Vec<RawRole>,
}

#[derive(Debug, Deserialize)]
struct RawRole {
    #[serde(default)]
    tid: u32,
    #[serde(default)]
    message_list: Vec<Vec<RawTurn>>,
    #[serde(rename = "QA", default)]
    qa: Option<RawQA>,
}

#[derive(Debug, Deserialize)]
struct RawTurn {
    #[serde(default)]
    sid: u32,
    #[serde(default)]
    user_message: String,
    #[serde(default)]
    assistant_message: String,
    #[serde(default)]
    time: Option<String>,
    #[serde(default)]
    place: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawQA {
    #[serde(default)]
    qid: u32,
    #[serde(default)]
    question: String,
    #[serde(default)]
    answer: String,
    #[serde(default)]
    ground_truth: String,
    #[serde(default)]
    choices: Option<std::collections::HashMap<String, String>>,
}

impl MemBenchDataset {
    pub async fn load(category: &str, force_download: bool) -> Result<Self> {
        if !CATEGORIES.contains(&category) {
            anyhow::bail!("Unknown MemBench category: {category}. Available: {}",
                CATEGORIES.join(", "));
        }
        let url = format!("{BASE_URL}/{category}.json");
        let filename = format!("membench_{category}.json");
        let path = download_dataset(&url, &filename, force_download).await?;
        let content = tokio::fs::read_to_string(&path).await?;
        Self::from_json(category, &content)
    }

    pub fn from_json(category: &str, json: &str) -> Result<Self> {
        let raw: RawFile = serde_json::from_str(json)
            .context("Failed to parse MemBench JSON")?;

        let mut questions = Vec::new();

        for role in &raw.roles {
            // Build sessions from message_list
            let sessions: Vec<ConversationSession> = role.message_list.iter()
                .enumerate()
                .map(|(block_idx, block)| ConversationSession {
                    id: format!("tid{}_{}", role.tid, block_idx),
                    date: block.first().and_then(|t| t.time.clone()),
                    turns: block.iter().flat_map(|turn| {
                        let mut turns = Vec::new();
                        if !turn.user_message.is_empty() {
                            turns.push(Turn { role: "user".to_string(), content: turn.user_message.clone() });
                        }
                        if !turn.assistant_message.is_empty() {
                            turns.push(Turn { role: "assistant".to_string(), content: turn.assistant_message.clone() });
                        }
                        turns
                    }).collect(),
                })
                .collect();

            if let Some(qa) = &role.qa {
                // Ground truth is the correct choice letter (A, B, C, D)
                // The actual answer text is in choices[ground_truth]
                let answer = if let Some(choices) = &qa.choices {
                    choices.get(&qa.ground_truth)
                        .cloned()
                        .unwrap_or_else(|| qa.answer.clone())
                } else {
                    qa.answer.clone()
                };

                questions.push(BenchmarkQuestion {
                    id: format!("membench_{category}_t{}_q{}", role.tid, qa.qid),
                    question_type: category.to_string(),
                    question: qa.question.clone(),
                    ground_truth: vec![answer],
                    question_date: None,
                    sessions,
                    is_abstention: false,
                    metadata: std::collections::HashMap::new(),
                });
            }
        }

        Ok(Self { category: category.to_string(), questions })
    }
}

impl BenchmarkDataset for MemBenchDataset {
    fn name(&self) -> &str { "membench" }
    fn variant(&self) -> &str { &self.category }
    fn description(&self) -> &str { "MemBench (ACL 2025) — multi-aspect memory evaluation with multiple-choice QA" }
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
    fn parse_membench_format() {
        let json = r#"{"roles":[{
            "tid": 0,
            "message_list": [[{
                "sid": 0,
                "user_message": "Tell me about coffee",
                "assistant_message": "Coffee is great!",
                "time": "2024-10-01 08:00"
            }]],
            "QA": {
                "qid": 0,
                "question": "What did the user ask about?",
                "answer": "Coffee",
                "ground_truth": "D",
                "choices": {"A": "Tea", "B": "Water", "C": "Juice", "D": "Coffee"}
            }
        }]}"#;
        let ds = MemBenchDataset::from_json("simple", json).unwrap();
        assert_eq!(ds.questions().len(), 1);
        assert_eq!(ds.questions()[0].ground_truth, vec!["Coffee"]); // Resolves D -> "Coffee"
        assert_eq!(ds.questions()[0].sessions.len(), 1);
        assert_eq!(ds.questions()[0].sessions[0].turns.len(), 2);
    }
}
