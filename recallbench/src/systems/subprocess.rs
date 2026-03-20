use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::traits::MemorySystem;
use crate::types::{ConversationSession, IngestStats, RetrievalResult};

/// Generic subprocess adapter — run any memory system via CLI commands.
pub struct SubprocessAdapter {
    name: String,
    version: String,
    reset_cmd: Vec<String>,
    ingest_cmd: Vec<String>,
    retrieve_cmd: Vec<String>,
}

/// TOML configuration for a subprocess adapter.
#[derive(Debug, Deserialize)]
pub struct SubprocessConfig {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub commands: Commands,
}

#[derive(Debug, Deserialize)]
pub struct Commands {
    /// Command and args for reset (e.g., ["my-system", "reset"])
    pub reset: Vec<String>,
    /// Command and args for ingest — receives JSON on stdin
    pub ingest: Vec<String>,
    /// Command and args for retrieve — receives JSON on stdin, outputs JSON on stdout
    pub retrieve: Vec<String>,
}

fn default_version() -> String { "unknown".to_string() }

impl SubprocessAdapter {
    pub fn from_config(config: SubprocessConfig) -> Self {
        Self {
            name: config.name,
            version: config.version,
            reset_cmd: config.commands.reset,
            ingest_cmd: config.commands.ingest,
            retrieve_cmd: config.commands.retrieve,
        }
    }

    pub fn from_toml(toml_str: &str) -> Result<Self> {
        let config: SubprocessConfig = toml::from_str(toml_str)
            .context("Failed to parse subprocess config")?;
        Ok(Self::from_config(config))
    }

    async fn run_command(&self, cmd: &[String], stdin_data: Option<&str>) -> Result<String> {
        let (program, args) = cmd.split_first()
            .context("Empty command")?;

        let mut child = tokio::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn: {program}"))?;

        if let Some(data) = stdin_data {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(data.as_bytes()).await?;
            }
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{program} exited with {}: {}", output.status, stderr.trim());
        }

        Ok(String::from_utf8(output.stdout)?)
    }
}

#[async_trait]
impl MemorySystem for SubprocessAdapter {
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    async fn reset(&self) -> Result<()> {
        self.run_command(&self.reset_cmd, None).await?;
        Ok(())
    }

    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats> {
        let start = std::time::Instant::now();
        let json = serde_json::to_string(session)?;
        let output = self.run_command(&self.ingest_cmd, Some(&json)).await?;

        let stats: IngestStats = serde_json::from_str(&output).unwrap_or(IngestStats {
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
        let request = serde_json::json!({
            "query": query,
            "query_date": query_date,
            "token_budget": token_budget,
        });
        let output = self.run_command(&self.retrieve_cmd, Some(&request.to_string())).await?;

        let result: RetrievalResult = serde_json::from_str(&output)
            .context("Failed to parse subprocess retrieve output")?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config() {
        let toml = r#"
name = "my-cli-system"
version = "2.0"

[commands]
reset = ["my-system", "reset"]
ingest = ["my-system", "ingest"]
retrieve = ["my-system", "retrieve"]
"#;
        let adapter = SubprocessAdapter::from_toml(toml).unwrap();
        assert_eq!(adapter.name(), "my-cli-system");
        assert_eq!(adapter.version(), "2.0");
    }
}
