//! zep HTTP adapter for RecallBench.

use anyhow::{Context, Result};
use async_trait::async_trait;
use recallbench::traits::MemorySystem;
use recallbench::types::{ConversationSession, IngestStats, RetrievalResult};

pub struct Adapter {
    base_url: String,
    client: reqwest::Client,
}

impl Adapter {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl MemorySystem for Adapter {
    fn name(&self) -> &str { "zep" }
    fn version(&self) -> &str { "0.1.0" }

    async fn reset(&self) -> Result<()> {
        self.client.post(format!("{}/reset", self.base_url))
            .send().await.context("reset failed")?;
        Ok(())
    }

    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats> {
        let start = std::time::Instant::now();
        self.client.post(format!("{}/memory", self.base_url))
            .json(session)
            .send().await.context("ingest failed")?;
        Ok(IngestStats {
            memories_stored: session.turns.len(),
            duplicates_skipped: 0,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn retrieve_context(
        &self,
        query: &str,
        _query_date: Option<&str>,
        _token_budget: usize,
    ) -> Result<RetrievalResult> {
        let start = std::time::Instant::now();
        let resp = self.client.post(format!("{}/search", self.base_url))
            .json(&serde_json::json!({"query": query}))
            .send().await.context("search failed")?;
        let body = resp.text().await?;
        Ok(RetrievalResult {
            context: body,
            items_retrieved: 1,
            tokens_used: body.len() / 4,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
