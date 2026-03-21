//! MindCore native adapter for RecallBench.
//!
//! Links directly to the mindcore crate for zero-overhead benchmarking.

use anyhow::Result;
use async_trait::async_trait;
use recallbench::traits::MemorySystem;
use recallbench::types::{ConversationSession, IngestStats, RetrievalResult};

pub struct MindCoreAdapter {
    // Will hold a MindCore MemoryEngine instance
    name: String,
    version: String,
}

impl MindCoreAdapter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            name: "mindcore".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}

#[async_trait]
impl MemorySystem for MindCoreAdapter {
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    async fn reset(&self) -> Result<()> {
        // TODO: reset MindCore engine state
        Ok(())
    }

    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats> {
        let start = std::time::Instant::now();
        // TODO: convert ConversationSession turns to MindCore memories and ingest
        Ok(IngestStats {
            memories_stored: session.turns.len(),
            duplicates_skipped: 0,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn retrieve_context(
        &self,
        _query: &str,
        _query_date: Option<&str>,
        _token_budget: usize,
    ) -> Result<RetrievalResult> {
        // TODO: use MindCore's hybrid search
        Ok(RetrievalResult {
            context: String::new(),
            items_retrieved: 0,
            tokens_used: 0,
            duration_ms: 0,
        })
    }
}
