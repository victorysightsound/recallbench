use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};

/// MemoryAgentBench dataset (ICLR 2026).
///
/// NOTE: The original dataset is in Parquet format on HuggingFace (ai-hyz/MemoryAgentBench).
/// This parser supports a JSON export of that data. To use:
/// 1. pip install datasets
/// 2. python -c "from datasets import load_dataset; ds = load_dataset('ai-hyz/MemoryAgentBench'); ds.to_json('mab_data.json')"
/// 3. recallbench run --dataset custom --variant default (with the exported JSON)
pub struct MemoryAgentBenchDataset {
    questions: Vec<BenchmarkQuestion>,
}

#[derive(Debug, Deserialize)]
struct RawEntry {
    #[serde(default)]
    context: String,
    #[serde(default)]
    questions: Vec<String>,
    #[serde(default)]
    answers: Vec<Vec<String>>,
    #[serde(default)]
    metadata: Option<RawMetadata>,
}

#[derive(Debug, Deserialize)]
struct RawMetadata {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    question_types: Option<Vec<String>>,
    #[serde(default)]
    question_ids: Option<Vec<String>>,
    #[serde(default)]
    haystack_sessions: Option<Vec<Vec<Vec<RawMessage>>>>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    role: String,
}

impl MemoryAgentBenchDataset {
    pub fn from_json(json: &str) -> Result<Self> {
        let entries: Vec<RawEntry> = serde_json::from_str(json)
            .context("Failed to parse MemoryAgentBench JSON")?;

        let mut questions = Vec::new();

        for (ei, entry) in entries.iter().enumerate() {
            let source = entry.metadata.as_ref()
                .and_then(|m| m.source.as_deref())
                .unwrap_or("unknown");

            // Build sessions from metadata if available
            let sessions: Vec<ConversationSession> = entry.metadata.as_ref()
                .and_then(|m| m.haystack_sessions.as_ref())
                .map(|hs| {
                    hs.iter().enumerate().map(|(si, session_group)| {
                        let turns: Vec<Turn> = session_group.iter()
                            .flat_map(|msgs| msgs.iter().map(|m| Turn {
                                role: m.role.clone(),
                                content: m.content.clone(),
                            }))
                            .collect();
                        ConversationSession {
                            id: format!("entry{ei}_session{si}"),
                            date: None,
                            turns,
                        }
                    }).collect()
                })
                .unwrap_or_else(|| {
                    // Fallback: use context as a single session
                    if entry.context.is_empty() { return vec![]; }
                    vec![ConversationSession {
                        id: format!("entry{ei}_context"),
                        date: None,
                        turns: vec![Turn { role: "user".to_string(), content: entry.context.clone() }],
                    }]
                });

            for (qi, question) in entry.questions.iter().enumerate() {
                let ground_truth = entry.answers.get(qi)
                    .cloned()
                    .unwrap_or_default();

                let qtype = entry.metadata.as_ref()
                    .and_then(|m| m.question_types.as_ref())
                    .and_then(|types| types.get(qi))
                    .cloned()
                    .unwrap_or_else(|| source.to_string());

                let qid = entry.metadata.as_ref()
                    .and_then(|m| m.question_ids.as_ref())
                    .and_then(|ids| ids.get(qi))
                    .cloned()
                    .unwrap_or_else(|| format!("mab_e{ei}_q{qi}"));

                questions.push(BenchmarkQuestion {
                    id: qid,
                    question_type: qtype,
                    question: question.clone(),
                    ground_truth,
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

impl BenchmarkDataset for MemoryAgentBenchDataset {
    fn name(&self) -> &str { "memoryagentbench" }
    fn variant(&self) -> &str { "default" }
    fn description(&self) -> &str { "MemoryAgentBench (ICLR 2026) — selective forgetting and fact consolidation (requires Parquet-to-JSON export)" }
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
    fn parse_mab_json_export() {
        let json = r#"[{
            "context": "Long conversation history here...",
            "questions": ["What happened first?", "Who was involved?"],
            "answers": [["meeting"], ["Alice", "Bob"]],
            "metadata": {
                "source": "eventqa",
                "question_types": ["event-qa", "entity"],
                "question_ids": ["q001", "q002"]
            }
        }]"#;
        let ds = MemoryAgentBenchDataset::from_json(json).unwrap();
        assert_eq!(ds.questions().len(), 2);
        assert_eq!(ds.questions()[0].question_type, "event-qa");
        assert_eq!(ds.questions()[1].ground_truth, vec!["Alice", "Bob"]);
    }
}
