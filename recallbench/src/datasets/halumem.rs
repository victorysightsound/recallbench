use anyhow::{Context, Result};
use serde::Deserialize;

use crate::traits::BenchmarkDataset;
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};
use super::download::download_dataset;

const MEDIUM_URL: &str = "https://huggingface.co/datasets/IAAR-Shanghai/HaluMem/resolve/main/HaluMem-Medium.jsonl";
const LONG_URL: &str = "https://huggingface.co/datasets/IAAR-Shanghai/HaluMem/resolve/main/HaluMem-Long.jsonl";

/// HaluMem dataset (MemTensor/IAAR-Shanghai).
/// Memory hallucination detection across extraction, update, and QA axes.
pub struct HaluMemDataset {
    variant: String,
    questions: Vec<BenchmarkQuestion>,
}

#[derive(Debug, Deserialize)]
struct RawUser {
    uuid: String,
    #[serde(default)]
    persona_info: String,
    #[serde(default)]
    sessions: Vec<RawSession>,
}

#[derive(Debug, Deserialize)]
struct RawSession {
    #[serde(default)]
    start_time: Option<String>,
    #[serde(default)]
    dialogue: Vec<RawDialogueTurn>,
    #[serde(default)]
    memory_points: Vec<RawMemoryPoint>,
    #[serde(default)]
    questions: Vec<RawQuestion>,
}

#[derive(Debug, Deserialize)]
struct RawDialogueTurn {
    role: String,
    content: String,
    #[serde(default)]
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMemoryPoint {
    #[serde(default)]
    memory_content: String,
    #[serde(default)]
    memory_type: String,
}

#[derive(Debug, Deserialize)]
struct RawQuestion {
    question: String,
    answer: String,
    #[serde(default)]
    difficulty: Option<String>,
    #[serde(default)]
    question_type: Option<String>,
}

impl HaluMemDataset {
    pub async fn load(variant: &str, force_download: bool) -> Result<Self> {
        let (url, filename) = match variant {
            "medium" => (MEDIUM_URL, "halumem_medium.jsonl"),
            "long" => (LONG_URL, "halumem_long.jsonl"),
            _ => anyhow::bail!("Unknown HaluMem variant: {variant}. Use medium or long."),
        };
        let path = download_dataset(url, filename, force_download).await?;
        let content = tokio::fs::read_to_string(&path).await?;
        Self::from_jsonl(variant, &content)
    }

    pub fn from_jsonl(variant: &str, jsonl: &str) -> Result<Self> {
        let mut questions = Vec::new();

        for line in jsonl.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }

            let user: RawUser = serde_json::from_str(line)
                .context("Failed to parse HaluMem JSONL line")?;

            // Build sessions from dialogue
            let sessions: Vec<ConversationSession> = user.sessions.iter()
                .enumerate()
                .map(|(i, s)| ConversationSession {
                    id: format!("{}_{}", user.uuid, i),
                    date: s.start_time.clone(),
                    turns: s.dialogue.iter().map(|t| Turn {
                        role: t.role.clone(),
                        content: t.content.clone(),
                    }).collect(),
                })
                .collect();

            // Extract questions from QA sessions
            for (si, session) in user.sessions.iter().enumerate() {
                for (qi, q) in session.questions.iter().enumerate() {
                    let qtype = q.question_type.as_deref().unwrap_or("memory-qa");

                    questions.push(BenchmarkQuestion {
                        id: format!("{}_s{}_q{}", user.uuid, si, qi),
                        question_type: qtype.to_string(),
                        question: q.question.clone(),
                        ground_truth: vec![q.answer.clone()],
                        question_date: session.start_time.clone(),
                        sessions: sessions.clone(),
                        is_abstention: false,
                        metadata: std::collections::HashMap::new(),
                    });
                }
            }
        }

        Ok(Self { variant: variant.to_string(), questions })
    }
}

impl BenchmarkDataset for HaluMemDataset {
    fn name(&self) -> &str { "halumem" }
    fn variant(&self) -> &str { &self.variant }
    fn description(&self) -> &str { "HaluMem (IAAR-Shanghai) — memory hallucination detection across extraction, update, and QA" }
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
    fn parse_halumem_jsonl() {
        let jsonl = r#"{"uuid":"test-001","persona_info":"Name: Test User","sessions":[{"start_time":"Sep 04, 2025","dialogue":[{"role":"user","content":"Hello","timestamp":"2025-09-04"},{"role":"assistant","content":"Hi!","timestamp":"2025-09-04"}],"memory_points":[{"memory_content":"User greeted","memory_type":"Event Memory"}],"questions":[{"question":"What did the user say?","answer":"Hello","difficulty":"easy","question_type":"Generalization & Application"}]}]}"#;
        let ds = HaluMemDataset::from_jsonl("medium", jsonl).unwrap();
        assert_eq!(ds.questions().len(), 1);
        assert_eq!(ds.questions()[0].ground_truth, vec!["Hello"]);
        assert_eq!(ds.questions()[0].sessions.len(), 1);
        assert_eq!(ds.questions()[0].sessions[0].turns.len(), 2);
    }
}
