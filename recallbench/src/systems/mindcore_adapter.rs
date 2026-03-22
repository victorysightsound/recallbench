//! MindCore native adapter — links directly to the mindcore crate.
//!
//! Enabled via the `mindcore-adapter` feature flag (on by default).

use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use mindcore::context::ContextBudget;
use mindcore::embeddings::CandleNativeBackend;
use mindcore::engine::MemoryEngine;
use mindcore::memory::store::StoreResult;
use mindcore::traits::{MemoryRecord, MemoryType};
use serde::{Deserialize, Serialize};

use crate::traits::MemorySystem;
use crate::types::{ConversationSession, IngestStats, RetrievalResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMemory {
    pub id: Option<i64>,
    pub content: String,
    pub role: String,
    pub session_index: usize,
    pub turn_index: usize,
    pub session_date: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl MemoryRecord for ConversationMemory {
    fn id(&self) -> Option<i64> { self.id }
    fn searchable_text(&self) -> String { self.content.clone() }
    fn memory_type(&self) -> MemoryType { MemoryType::Episodic }
    fn importance(&self) -> u8 { if self.role == "user" { 6 } else { 5 } }
    fn created_at(&self) -> chrono::DateTime<Utc> { self.created_at }
    fn category(&self) -> Option<&str> { Some(&self.role) }
    fn metadata(&self) -> HashMap<String, String> {
        let mut meta = HashMap::new();
        meta.insert("session_index".into(), self.session_index.to_string());
        meta.insert("turn_index".into(), self.turn_index.to_string());
        meta.insert("session_date".into(), self.session_date.clone());
        meta
    }
}

pub struct MindCoreAdapter {
    engine: Mutex<MemoryEngine<ConversationMemory>>,
    /// Accumulated records awaiting batch embedding. Flushed on retrieve_context().
    pending: Mutex<Vec<ConversationMemory>>,
    /// Reusable embedding backend — shared Arc avoids reloading model on reset().
    backend: std::sync::Arc<CandleNativeBackend>,
}

impl MindCoreAdapter {
    pub fn new() -> Result<Self> {
        let backend = std::sync::Arc::new(CandleNativeBackend::new()?);
        let engine = MemoryEngine::<ConversationMemory>::builder()
            .embedding_backend_arc(std::sync::Arc::clone(&backend) as std::sync::Arc<dyn mindcore::embeddings::EmbeddingBackend>)
            .build()?;
        Ok(Self {
            engine: Mutex::new(engine),
            pending: Mutex::new(Vec::new()),
            backend,
        })
    }

    /// Flush all pending records into the engine via a single store_batch().
    fn flush_pending(&self) -> Result<(usize, usize)> {
        let records: Vec<ConversationMemory> = {
            let mut pending = self.pending.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
            std::mem::take(&mut *pending)
        };

        if records.is_empty() {
            return Ok((0, 0));
        }

        tracing::info!("Flushing {} accumulated chunks for batch embedding", records.len());
        let engine = self.engine.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let results = engine.store_batch(&records)?;
        let stored = results.iter().filter(|r| matches!(r, StoreResult::Added(_))).count();
        let dupes = results.iter().filter(|r| matches!(r, StoreResult::Duplicate(_))).count();
        Ok((stored, dupes))
    }
}

#[async_trait]
impl MemorySystem for MindCoreAdapter {
    fn name(&self) -> &str { "mindcore" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    async fn reset(&self) -> Result<()> {
        // Reuse the existing backend Arc — no model reload needed
        let new_engine = MemoryEngine::<ConversationMemory>::builder()
            .embedding_backend_arc(std::sync::Arc::clone(&self.backend) as std::sync::Arc<dyn mindcore::embeddings::EmbeddingBackend>)
            .build()?;
        let mut guard = self.engine.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        *guard = new_engine;
        // Clear any pending records from previous question
        let mut pending = self.pending.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        pending.clear();
        Ok(())
    }

    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats> {
        let start = std::time::Instant::now();
        let session_date = session.date.clone().unwrap_or_default();

        // Chunk session turns into ~2000-char segments (larger = fewer embeddings, faster)
        let turns_iter = session.turns.iter().map(|t| (t.role.as_str(), t.content.as_str()));
        let chunks = mindcore::ingest::chunking::chunk_session(turns_iter, &session_date, 2000, 10);

        let records: Vec<ConversationMemory> = chunks.iter().enumerate()
            .map(|(idx, chunk)| ConversationMemory {
                id: None,
                content: chunk.text.clone(),
                role: "chunk".to_string(),
                session_index: 0,
                turn_index: idx,
                session_date: session_date.clone(),
                created_at: Utc::now(),
            })
            .collect();

        let count = records.len();

        // Accumulate — don't embed yet. All sessions will be batch-embedded at retrieve time.
        let mut pending = self.pending.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        pending.extend(records);

        Ok(IngestStats { memories_stored: count, duplicates_skipped: 0, duration_ms: start.elapsed().as_millis() as u64 })
    }

    async fn retrieve_context(&self, query: &str, _query_date: Option<&str>, token_budget: usize) -> Result<RetrievalResult> {
        // Flush all accumulated chunks in a single batch (one embed_batch() call)
        let (stored, _dupes) = self.flush_pending()?;
        if stored > 0 {
            tracing::info!("Batch embedded {stored} chunks");
        }

        let start = std::time::Instant::now();
        let engine = self.engine.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let budget = ContextBudget::new(token_budget);
        let assembly = engine.assemble_context(query, &budget)?;
        let context: String = assembly.items.iter().map(|item| item.content.as_str()).collect::<Vec<_>>().join("\n");

        Ok(RetrievalResult {
            context,
            items_retrieved: assembly.items.len(),
            tokens_used: assembly.total_tokens,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Turn;

    #[tokio::test]
    async fn ingest_and_retrieve() {
        let adapter = MindCoreAdapter::new().unwrap();
        let session = ConversationSession {
            id: "s1".to_string(), date: Some("2024-01-15".to_string()),
            turns: vec![
                Turn { role: "user".to_string(), content: "My favorite color is blue and I really enjoy painting landscapes".to_string() },
                Turn { role: "assistant".to_string(), content: "That sounds wonderful! Blue is a calming color for landscapes.".to_string() },
            ],
        };
        let stats = adapter.ingest_session(&session).await.unwrap();
        assert!(stats.memories_stored >= 1, "should store at least 1 chunk");
        // Retrieval triggers flush + embedding
        let result = adapter.retrieve_context("favorite color", None, 16384).await.unwrap();
        assert!(result.context.contains("blue"), "should find 'blue' in context");
    }

    #[tokio::test]
    async fn reset_clears() {
        let adapter = MindCoreAdapter::new().unwrap();
        let session = ConversationSession {
            id: "s1".to_string(), date: None,
            turns: vec![Turn { role: "user".to_string(), content: "Hello world, this is a test message with enough content".to_string() }],
        };
        adapter.ingest_session(&session).await.unwrap();
        adapter.reset().await.unwrap();
        let result = adapter.retrieve_context("hello", None, 16384).await.unwrap();
        assert_eq!(result.items_retrieved, 0);
    }
}
