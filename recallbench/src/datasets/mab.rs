use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};

const HF_API: &str = "https://datasets-server.huggingface.co/rows";
const DATASET_ID: &str = "ai-hyz/MemoryAgentBench";

/// Available splits in MemoryAgentBench.
const SPLITS: &[(&str, &str)] = &[
    ("accurate_retrieval", "Accurate_Retrieval"),
    ("conflict_resolution", "Conflict_Resolution"),
    ("long_range", "Long_Range_Understanding"),
    ("test_time_learning", "Test_Time_Learning"),
];

/// MemoryAgentBench dataset (ICLR 2026).
/// Tests selective forgetting, fact consolidation, and long-range retrieval.
/// Downloaded directly from HuggingFace API (no Parquet dependency).
pub struct MemoryAgentBenchDataset {
    split: String,
    questions: Vec<BenchmarkQuestion>,
}

#[derive(Debug, Deserialize)]
struct HfResponse {
    #[serde(default)]
    rows: Vec<HfRow>,
    #[serde(default)]
    num_rows_total: usize,
}

#[derive(Debug, Deserialize)]
struct HfRow {
    row: RawEntry,
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
    question_ids: Option<Vec<String>>,
}

impl MemoryAgentBenchDataset {
    /// Load a split from HuggingFace API.
    pub async fn load(split_name: &str) -> Result<Self> {
        let (short_name, hf_split) = SPLITS.iter()
            .find(|(short, _)| *short == split_name)
            .ok_or_else(|| anyhow::anyhow!(
                "Unknown MAB split: {split_name}. Available: {}",
                SPLITS.iter().map(|(s, _)| *s).collect::<Vec<_>>().join(", ")
            ))?;

        // First get total count
        let url = format!("{HF_API}?dataset={DATASET_ID}&config=default&split={hf_split}&offset=0&length=1");
        let client = reqwest::Client::new();
        let resp: HfResponse = client.get(&url).send().await?.json().await
            .context("Failed to query HuggingFace API for row count")?;
        let total = resp.num_rows_total;

        tracing::info!("Downloading MemoryAgentBench/{hf_split}: {total} examples from HuggingFace API");

        // Download all rows (paginated in chunks of 100)
        let mut all_entries = Vec::new();
        let mut offset = 0;
        while offset < total {
            let batch_size = 100.min(total - offset);
            let url = format!(
                "{HF_API}?dataset={DATASET_ID}&config=default&split={hf_split}&offset={offset}&length={batch_size}"
            );
            let resp: HfResponse = client.get(&url).send().await?.json().await
                .with_context(|| format!("Failed to fetch rows at offset {offset}"))?;

            for row in resp.rows {
                all_entries.push(row.row);
            }
            offset += batch_size;
        }

        Self::from_entries(short_name, &all_entries)
    }

    fn from_entries(split: &str, entries: &[RawEntry]) -> Result<Self> {
        let mut questions = Vec::new();

        for (ei, entry) in entries.iter().enumerate() {
            let source = entry.metadata.as_ref()
                .and_then(|m| m.source.as_deref())
                .unwrap_or("unknown");

            // Use context as a single session (these are long-context benchmarks)
            let sessions = if entry.context.is_empty() {
                vec![]
            } else {
                vec![ConversationSession {
                    id: format!("entry{ei}_context"),
                    date: None,
                    turns: vec![Turn { role: "user".to_string(), content: entry.context.clone() }],
                }]
            };

            for (qi, question) in entry.questions.iter().enumerate() {
                let ground_truth = entry.answers.get(qi)
                    .cloned()
                    .unwrap_or_default();

                // Deduplicate answers (MAB often has ["France", "France", "France", "France"])
                let ground_truth: Vec<String> = ground_truth.into_iter()
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                let qid = entry.metadata.as_ref()
                    .and_then(|m| m.question_ids.as_ref())
                    .and_then(|ids| ids.get(qi))
                    .cloned()
                    .unwrap_or_else(|| format!("mab_{split}_e{ei}_q{qi}"));

                questions.push(BenchmarkQuestion {
                    id: qid,
                    question_type: source.to_string(),
                    question: question.clone(),
                    ground_truth,
                    question_date: None,
                    sessions: sessions.clone(),
                    is_abstention: false,
                    metadata: std::collections::HashMap::new(),
                });
            }
        }

        Ok(Self { split: split.to_string(), questions })
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let entries: Vec<RawEntry> = serde_json::from_str(json)
            .context("Failed to parse MemoryAgentBench JSON")?;
        Self::from_entries("custom", &entries)
    }
}

impl BenchmarkDataset for MemoryAgentBenchDataset {
    fn name(&self) -> &str { "memoryagentbench" }
    fn variant(&self) -> &str { &self.split }
    fn description(&self) -> &str { "MemoryAgentBench (ICLR 2026) — selective forgetting, fact consolidation, long-range retrieval" }
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
    fn parse_json_format() {
        let json = r#"[{
            "context": "Long history...",
            "questions": ["What happened?", "Who was there?"],
            "answers": [["meeting", "meeting"], ["Alice"]],
            "metadata": {"source": "eventqa", "question_ids": ["q1", "q2"]}
        }]"#;
        let ds = MemoryAgentBenchDataset::from_json(json).unwrap();
        assert_eq!(ds.questions().len(), 2);
        assert_eq!(ds.questions()[0].ground_truth, vec!["meeting"]); // deduplicated
        assert_eq!(ds.questions()[1].ground_truth, vec!["Alice"]);
    }

    #[test]
    fn split_names() {
        for (short, _full) in SPLITS {
            assert!(!short.is_empty());
        }
        assert_eq!(SPLITS.len(), 4);
    }
}
