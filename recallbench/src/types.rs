use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single turn in a conversation (user or assistant message).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Turn {
    pub role: String,
    pub content: String,
}

/// A conversation session containing multiple turns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConversationSession {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    pub turns: Vec<Turn>,
}

/// Universal question format that all datasets normalize into.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkQuestion {
    pub id: String,
    pub question_type: String,
    pub question: String,
    pub ground_truth: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_date: Option<String>,
    pub sessions: Vec<ConversationSession>,
    #[serde(default)]
    pub is_abstention: bool,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Statistics returned from ingesting sessions into a memory system.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestStats {
    pub memories_stored: usize,
    pub duplicates_skipped: usize,
    pub duration_ms: u64,
}

/// Result of a retrieval operation from a memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub context: String,
    pub items_retrieved: usize,
    pub tokens_used: usize,
    pub duration_ms: u64,
}

/// Result of evaluating a single question against a single system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub question_id: String,
    pub system_name: String,
    pub question_type: String,
    #[serde(default)]
    pub is_abstention: bool,
    pub hypothesis: String,
    pub ground_truth: String,
    pub is_correct: bool,
    pub ingest_latency_ms: u64,
    pub retrieval_latency_ms: u64,
    pub generation_latency_ms: u64,
    pub judge_latency_ms: u64,
    pub tokens_used: u32,
    pub tokens_generated: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_roundtrip() {
        let turn = Turn {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&turn).unwrap();
        let parsed: Turn = serde_json::from_str(&json).unwrap();
        assert_eq!(turn, parsed);
    }

    #[test]
    fn conversation_session_roundtrip() {
        let session = ConversationSession {
            id: "s001".to_string(),
            date: Some("2024/01/15 (Mon) 14:30".to_string()),
            turns: vec![
                Turn { role: "user".to_string(), content: "Hi".to_string() },
                Turn { role: "assistant".to_string(), content: "Hello!".to_string() },
            ],
        };
        let json = serde_json::to_string(&session).unwrap();
        let parsed: ConversationSession = serde_json::from_str(&json).unwrap();
        assert_eq!(session, parsed);
    }

    #[test]
    fn conversation_session_no_date() {
        let json = r#"{"id":"s002","turns":[]}"#;
        let session: ConversationSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "s002");
        assert!(session.date.is_none());
        assert!(session.turns.is_empty());
    }

    #[test]
    fn benchmark_question_roundtrip() {
        let question = BenchmarkQuestion {
            id: "q001".to_string(),
            question_type: "temporal-reasoning".to_string(),
            question: "When did the user mention pizza?".to_string(),
            ground_truth: vec!["March 5th".to_string()],
            question_date: Some("2024/03/10".to_string()),
            sessions: vec![],
            is_abstention: false,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&question).unwrap();
        let parsed: BenchmarkQuestion = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "q001");
        assert_eq!(parsed.question_type, "temporal-reasoning");
        assert_eq!(parsed.ground_truth, vec!["March 5th"]);
        assert!(!parsed.is_abstention);
    }

    #[test]
    fn benchmark_question_defaults() {
        let json = r#"{
            "id": "q002",
            "question_type": "single-session-user",
            "question": "What is the user's name?",
            "ground_truth": ["John"],
            "sessions": []
        }"#;
        let parsed: BenchmarkQuestion = serde_json::from_str(json).unwrap();
        assert!(parsed.question_date.is_none());
        assert!(!parsed.is_abstention);
        assert!(parsed.metadata.is_empty());
    }

    #[test]
    fn ingest_stats_default() {
        let stats = IngestStats::default();
        assert_eq!(stats.memories_stored, 0);
        assert_eq!(stats.duplicates_skipped, 0);
        assert_eq!(stats.duration_ms, 0);
    }

    #[test]
    fn retrieval_result_roundtrip() {
        let result = RetrievalResult {
            context: "Some retrieved context".to_string(),
            items_retrieved: 5,
            tokens_used: 1024,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: RetrievalResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.items_retrieved, 5);
        assert_eq!(parsed.tokens_used, 1024);
    }

    #[test]
    fn eval_result_roundtrip() {
        let result = EvalResult {
            question_id: "q001".to_string(),
            system_name: "mindcore".to_string(),
            question_type: "temporal-reasoning".to_string(),
            is_abstention: false,
            hypothesis: "March 5th".to_string(),
            ground_truth: "March 5th".to_string(),
            is_correct: true,
            ingest_latency_ms: 12,
            retrieval_latency_ms: 8,
            generation_latency_ms: 2100,
            judge_latency_ms: 890,
            tokens_used: 1024,
            tokens_generated: 42,
            timestamp: Some(Utc::now()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: EvalResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.question_id, "q001");
        assert!(parsed.is_correct);
        assert!(parsed.timestamp.is_some());
    }

    #[test]
    fn eval_result_jsonl_format() {
        let result = EvalResult {
            question_id: "q001".to_string(),
            system_name: "mindcore".to_string(),
            question_type: "temporal-reasoning".to_string(),
            is_abstention: false,
            hypothesis: "March 5th".to_string(),
            ground_truth: "March 5th".to_string(),
            is_correct: true,
            ingest_latency_ms: 12,
            retrieval_latency_ms: 8,
            generation_latency_ms: 2100,
            judge_latency_ms: 890,
            tokens_used: 1024,
            tokens_generated: 42,
            timestamp: None,
        };
        let line = serde_json::to_string(&result).unwrap();
        assert!(!line.contains('\n'));
        let parsed: EvalResult = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed.system_name, "mindcore");
    }
}
