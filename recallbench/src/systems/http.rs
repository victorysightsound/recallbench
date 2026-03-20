use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::traits::MemorySystem;
use crate::types::{ConversationSession, IngestStats, RetrievalResult};

/// Generic HTTP adapter — configure endpoints via TOML.
pub struct HttpSystemAdapter {
    name: String,
    version: String,
    reset_url: String,
    ingest_url: String,
    retrieve_url: String,
    client: reqwest::Client,
}

/// TOML configuration for a generic HTTP system.
#[derive(Debug, Deserialize)]
pub struct HttpSystemConfig {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub endpoints: Endpoints,
}

#[derive(Debug, Deserialize)]
pub struct Endpoints {
    pub reset: String,
    pub ingest: String,
    pub retrieve: String,
}

fn default_version() -> String { "unknown".to_string() }

#[derive(Serialize)]
struct IngestRequest {
    session: ConversationSession,
}

#[derive(Serialize)]
struct RetrieveRequest {
    query: String,
    query_date: Option<String>,
    token_budget: usize,
}

impl HttpSystemAdapter {
    pub fn from_config(config: HttpSystemConfig) -> Self {
        Self {
            name: config.name,
            version: config.version,
            reset_url: config.endpoints.reset,
            ingest_url: config.endpoints.ingest,
            retrieve_url: config.endpoints.retrieve,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_toml(toml_str: &str) -> Result<Self> {
        let config: HttpSystemConfig = toml::from_str(toml_str)
            .context("Failed to parse HTTP system config")?;
        Ok(Self::from_config(config))
    }
}

#[async_trait]
impl MemorySystem for HttpSystemAdapter {
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    async fn reset(&self) -> Result<()> {
        let resp = self.client.post(&self.reset_url).send().await
            .context("Reset endpoint failed")?;
        if !resp.status().is_success() {
            anyhow::bail!("Reset returned {}", resp.status());
        }
        Ok(())
    }

    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats> {
        let start = std::time::Instant::now();
        let resp = self.client.post(&self.ingest_url)
            .json(&IngestRequest { session: session.clone() })
            .send().await
            .context("Ingest endpoint failed")?;

        if !resp.status().is_success() {
            anyhow::bail!("Ingest returned {}", resp.status());
        }

        let stats: IngestStats = resp.json().await.unwrap_or(IngestStats {
            memories_stored: session.turns.len(),
            duplicates_skipped: 0,
            duration_ms: start.elapsed().as_millis() as u64,
        });
        Ok(stats)
    }

    async fn retrieve_context(
        &self,
        query: &str,
        query_date: Option<&str>,
        token_budget: usize,
    ) -> Result<RetrievalResult> {
        let resp = self.client.post(&self.retrieve_url)
            .json(&RetrieveRequest {
                query: query.to_string(),
                query_date: query_date.map(|s| s.to_string()),
                token_budget,
            })
            .send().await
            .context("Retrieve endpoint failed")?;

        if !resp.status().is_success() {
            anyhow::bail!("Retrieve returned {}", resp.status());
        }

        let result: RetrievalResult = resp.json().await
            .context("Failed to parse retrieve response")?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config() {
        let toml = r#"
name = "my-system"
version = "1.0"

[endpoints]
reset = "http://localhost:8080/reset"
ingest = "http://localhost:8080/ingest"
retrieve = "http://localhost:8080/retrieve"
"#;
        let adapter = HttpSystemAdapter::from_toml(toml).unwrap();
        assert_eq!(adapter.name(), "my-system");
        assert_eq!(adapter.version(), "1.0");
    }
}
