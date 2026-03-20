use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::traits::LLMClient;

/// OpenAI Codex CLI client. CLI-only provider.
pub struct CodexClient {
    model: String,
}

impl CodexClient {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl LLMClient for CodexClient {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate(&self, prompt: &str, _max_tokens: usize) -> Result<String> {
        let mut cmd = tokio::process::Command::new("codex");
        cmd.args(["--model", &self.model])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()
            .context("Failed to spawn codex CLI. Is it installed?")?;

        if let Some(stdin) = child.stdin.as_mut() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(prompt.as_bytes()).await
                .context("Failed to write to codex stdin")?;
        }

        let output = child.wait_with_output().await
            .context("Failed to read codex output")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("codex exited with {}: {}", output.status, stderr.trim());
        }

        let response = String::from_utf8(output.stdout)
            .context("Codex output was not valid UTF-8")?;

        Ok(response.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_name() {
        let client = CodexClient::new("codex");
        assert_eq!(client.name(), "codex");
    }
}
