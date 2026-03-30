use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;

use crate::traits::LLMClient;

/// OpenAI Codex CLI client. CLI-only provider.
pub struct CodexClient {
    model: String,
    reasoning_effort: Option<String>,
}

impl CodexClient {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
            reasoning_effort: None,
        }
    }

    pub fn with_effort(model: &str, reasoning_effort: Option<&str>) -> Self {
        Self {
            model: model.to_string(),
            reasoning_effort: reasoning_effort.map(|s| s.to_string()),
        }
    }
}

#[async_trait]
impl LLMClient for CodexClient {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate(&self, prompt: &str, _max_tokens: usize) -> Result<String> {
        let output_path = codex_output_path();
        let mut cmd = tokio::process::Command::new("codex");
        let mut args = vec![
            "exec".to_string(),
            "--output-last-message".to_string(),
            output_path.to_string_lossy().into_owned(),
            "--model".to_string(),
            self.model.clone(),
        ];
        if let Some(ref effort) = self.reasoning_effort {
            args.push("--config".to_string());
            args.push(format!("model_reasoning_effort={effort}"));
        }
        args.extend([
            "--skip-git-repo-check".to_string(),
            "--sandbox".to_string(),
            "read-only".to_string(),
            "--ephemeral".to_string(),
            "-".to_string(),
        ]);
        cmd.args(&args)
            .env("RECALLBENCH_SUBPROCESS", "1")
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

        let stdout = String::from_utf8(output.stdout)
            .context("Codex output was not valid UTF-8")?;
        let file_output = std::fs::read_to_string(&output_path).ok()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty());
        let _ = std::fs::remove_file(&output_path);
        let response = file_output.unwrap_or_else(|| stdout.trim().to_string());

        Ok(response)
    }
}

fn codex_output_path() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "recallbench-codex-output-{}-{stamp}.txt",
        std::process::id()
    ))
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
