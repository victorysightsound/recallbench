use std::sync::Mutex;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;

use crate::traits::MemorySystem;
use crate::types::{ConversationSession, IngestStats, RetrievalResult};

/// A test adapter that stores ingested sessions and returns them as context.
///
/// EchoSystem does not perform any actual memory operations — it simply
/// echoes back the ingested content as retrieval context. This is used for
/// testing the benchmark pipeline without needing a real memory system.
pub struct EchoSystem {
    sessions: Mutex<Vec<ConversationSession>>,
}

impl EchoSystem {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(Vec::new()),
        }
    }
}

impl Default for EchoSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemorySystem for EchoSystem {
    fn name(&self) -> &str {
        "echo"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn reset(&self) -> Result<()> {
        let mut sessions = self.sessions.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        sessions.clear();
        Ok(())
    }

    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats> {
        let start = Instant::now();
        let mut sessions = self.sessions.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let turn_count = session.turns.len();
        sessions.push(session.clone());
        Ok(IngestStats {
            memories_stored: turn_count,
            duplicates_skipped: 0,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn retrieve_context(
        &self,
        _query: &str,
        _query_date: Option<&str>,
        token_budget: usize,
    ) -> Result<RetrievalResult> {
        let start = Instant::now();
        let sessions = self.sessions.lock().map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut context = String::new();
        let mut items = 0;

        for session in sessions.iter() {
            for turn in &session.turns {
                let line = format!("[{}]: {}\n", turn.role, turn.content);
                let estimated_tokens = context.len() / 4 + line.len() / 4;
                if estimated_tokens > token_budget {
                    break;
                }
                context.push_str(&line);
                items += 1;
            }
        }

        let tokens_used = context.len() / 4;
        Ok(RetrievalResult {
            context,
            items_retrieved: items,
            tokens_used,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Turn;

    #[tokio::test]
    async fn echo_ingest_and_retrieve() {
        let echo = EchoSystem::new();

        let session = ConversationSession {
            id: "s001".to_string(),
            date: Some("2024/01/15".to_string()),
            turns: vec![
                Turn { role: "user".to_string(), content: "Hello".to_string() },
                Turn { role: "assistant".to_string(), content: "Hi there!".to_string() },
            ],
        };

        let stats = echo.ingest_session(&session).await.unwrap();
        assert_eq!(stats.memories_stored, 2);
        assert_eq!(stats.duplicates_skipped, 0);

        let result = echo.retrieve_context("anything", None, 16384).await.unwrap();
        assert!(result.context.contains("Hello"));
        assert!(result.context.contains("Hi there!"));
        assert_eq!(result.items_retrieved, 2);
    }

    #[tokio::test]
    async fn echo_reset() {
        let echo = EchoSystem::new();

        let session = ConversationSession {
            id: "s001".to_string(),
            date: None,
            turns: vec![
                Turn { role: "user".to_string(), content: "Hello".to_string() },
            ],
        };

        echo.ingest_session(&session).await.unwrap();
        echo.reset().await.unwrap();

        let result = echo.retrieve_context("anything", None, 16384).await.unwrap();
        assert!(result.context.is_empty());
        assert_eq!(result.items_retrieved, 0);
    }

    #[tokio::test]
    async fn echo_respects_token_budget() {
        let echo = EchoSystem::new();

        let session = ConversationSession {
            id: "s001".to_string(),
            date: None,
            turns: vec![
                Turn { role: "user".to_string(), content: "A".repeat(100) },
                Turn { role: "assistant".to_string(), content: "B".repeat(100) },
                Turn { role: "user".to_string(), content: "C".repeat(100) },
            ],
        };

        echo.ingest_session(&session).await.unwrap();

        // Very small budget should limit output
        let result = echo.retrieve_context("anything", None, 10).await.unwrap();
        assert!(result.items_retrieved < 3);
    }

    #[tokio::test]
    async fn echo_name_and_version() {
        let echo = EchoSystem::new();
        assert_eq!(echo.name(), "echo");
        assert_eq!(echo.version(), "1.0.0");
    }
}
