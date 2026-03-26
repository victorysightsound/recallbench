//! Memloft adapter — benchmarks memloft-core's memory engine directly.
//!
//! Links to memloft-core as a library crate. Uses MemoryStore for storage
//! and HybridSearcher for retrieval. No daemon needed.
//!
//! Enabled via the `memloft-adapter` feature flag.

use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;

use memloft_core::{Db, MemoryStore, HybridSearcher};

use crate::traits::MemorySystem;
use crate::types::{ConversationSession, IngestStats, RetrievalResult};

pub struct MemloftAdapter {
    db: Mutex<Db>,
}

impl MemloftAdapter {
    /// Create a new memloft adapter with an in-memory database.
    pub fn new() -> Result<Self> {
        let db = Db::open_in_memory()
            .map_err(|e| anyhow::anyhow!("Failed to open memloft DB: {e}"))?;
        Ok(Self { db: Mutex::new(db) })
    }

    /// Reset the database by re-creating it.
    fn reset_db(&self) -> Result<()> {
        let mut db = self.db.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        *db = Db::open_in_memory()
            .map_err(|e| anyhow::anyhow!("Failed to reset memloft DB: {e}"))?;
        Ok(())
    }
}

#[async_trait]
impl MemorySystem for MemloftAdapter {
    fn name(&self) -> &str { "memloft" }
    fn version(&self) -> &str { "0.2.0" }

    async fn reset(&self) -> Result<()> {
        self.reset_db()
    }

    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats> {
        let start = std::time::Instant::now();
        let db = self.db.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let store = MemoryStore::new(&db);
        let session_date = session.date.clone().unwrap_or_default();
        let mut stored = 0usize;
        let mut duplicates = 0usize;

        // Concatenate turns into session chunks for storage
        // Each chunk becomes a memloft memory with category "note"
        let turns_iter = session.turns.iter().map(|t| (t.role.as_str(), t.content.as_str()));
        let chunks = femind::ingest::chunking::chunk_session(turns_iter, &session_date, 2000, 10);

        for (idx, chunk) in chunks.iter().enumerate() {
            let topic = format!("session-{}-chunk-{}", session.id, idx);
            match store.log_if_new("note", &topic, &chunk.text, 5, Some("benchmark")) {
                Ok(Some(_id)) => stored += 1,
                Ok(None) => duplicates += 1,
                Err(e) => tracing::warn!("Failed to store chunk: {e}"),
            }
        }

        Ok(IngestStats {
            memories_stored: stored,
            duplicates_skipped: duplicates,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn retrieve_context(&self, query: &str, _query_date: Option<&str>, token_budget: usize) -> Result<RetrievalResult> {
        let start = std::time::Instant::now();
        let db = self.db.lock().map_err(|e| anyhow::anyhow!("{e}"))?;

        // Try hybrid search if available, fall back to keyword
        let results = {
            let searcher = HybridSearcher::new(&db, None);
            match searcher.search(query, "hybrid", 50, 0.0) {
                Ok(results) => results,
                Err(_) => {
                    // Hybrid not available (no embeddings), fall back to keyword
                    let store = MemoryStore::new(&db);
                    let keyword_results = store.search(query, 50)
                        .map_err(|e| anyhow::anyhow!("search failed: {e}"))?;
                    // Convert MemoryRow to a compatible format
                    keyword_results.into_iter().map(|row| {
                        memloft_core::SearchResult {
                            id: row.id,
                            category: row.category,
                            topic: row.topic,
                            content: row.content,
                            score: 1.0, // No score from keyword search
                            match_type: memloft_core::MatchType::Keyword,
                            tier: String::new(),
                            metadata: row.metadata,
                            plan_id: row.plan_id,
                        }
                    }).collect()
                }
            }
        };

        // Assemble context within token budget
        let mut context_parts = Vec::new();
        let mut tokens_used = 0usize;

        for result in &results {
            let estimated = memloft_core::estimate_tokens(&result.content);
            if tokens_used + estimated > token_budget {
                break;
            }
            context_parts.push(result.content.clone());
            tokens_used += estimated;
        }

        let context = context_parts.join("\n\n");

        Ok(RetrievalResult {
            context,
            items_retrieved: context_parts.len(),
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
    async fn memloft_ingest_and_retrieve() {
        let adapter = MemloftAdapter::new().unwrap();
        let session = ConversationSession {
            id: "s1".to_string(),
            date: Some("2024-01-15".to_string()),
            turns: vec![
                Turn { role: "user".to_string(), content: "My favorite programming language is Rust because of its safety guarantees".to_string() },
                Turn { role: "assistant".to_string(), content: "Rust is a great choice for systems programming with memory safety".to_string() },
            ],
        };
        let stats = adapter.ingest_session(&session).await.unwrap();
        assert!(stats.memories_stored >= 1);

        let result = adapter.retrieve_context("favorite programming language", None, 16384).await.unwrap();
        assert!(result.context.contains("Rust"), "should find Rust in context: {}", result.context);
    }

    #[tokio::test]
    async fn memloft_reset_clears() {
        let adapter = MemloftAdapter::new().unwrap();
        let session = ConversationSession {
            id: "s1".to_string(),
            date: None,
            turns: vec![Turn { role: "user".to_string(), content: "Remember this important fact about quantum computing".to_string() }],
        };
        adapter.ingest_session(&session).await.unwrap();
        adapter.reset().await.unwrap();
        let result = adapter.retrieve_context("quantum computing", None, 16384).await.unwrap();
        assert_eq!(result.items_retrieved, 0);
    }
}
