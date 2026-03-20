use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::traits::LLMClient;

/// Generic CLI adapter that works with claude, chatgpt, gemini, codex CLIs.
///
/// Uses the CLI subscription model — no API key needed.
/// This is the default provider mode.
pub struct CliLLMClient {
    /// The CLI command to invoke (e.g., "claude", "chatgpt", "gemini").
    command: String,
    /// The model identifier passed to the CLI.
    model: String,
}

impl CliLLMClient {
    pub fn new(command: &str, model: &str) -> Self {
        Self {
            command: command.to_string(),
            model: model.to_string(),
        }
    }

    /// Build CLI arguments for the given command.
    fn build_args(&self, _max_tokens: usize) -> Vec<String> {
        match self.command.as_str() {
            "claude" => {
                vec![
                    "--print".to_string(),
                    "--model".to_string(),
                    self.model.clone(),
                    "--max-turns".to_string(),
                    "1".to_string(),
                ]
            }
            "chatgpt" => {
                vec![
                    "--model".to_string(),
                    self.model.clone(),
                ]
            }
            "gemini" => {
                vec![
                    "--model".to_string(),
                    self.model.clone(),
                ]
            }
            "codex" => {
                vec![
                    "--model".to_string(),
                    self.model.clone(),
                ]
            }
            _ => {
                vec![
                    "--model".to_string(),
                    self.model.clone(),
                ]
            }
        }
    }
}

#[async_trait]
impl LLMClient for CliLLMClient {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let args = self.build_args(max_tokens);

        let mut cmd = tokio::process::Command::new(&self.command);
        cmd.args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()
            .with_context(|| format!("Failed to spawn '{}'. Is it installed?", self.command))?;

        if let Some(stdin) = child.stdin.as_mut() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(prompt.as_bytes()).await
                .context("Failed to write prompt to CLI stdin")?;
        }

        let output = child.wait_with_output().await
            .context("Failed to read CLI output")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "{} exited with status {}: {}",
                self.command,
                output.status,
                stderr.trim()
            );
        }

        let response = String::from_utf8(output.stdout)
            .context("CLI output was not valid UTF-8")?;

        Ok(response.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_client_name() {
        let client = CliLLMClient::new("claude", "claude-sonnet");
        assert_eq!(client.name(), "claude-sonnet");
    }

    #[test]
    fn claude_args() {
        let client = CliLLMClient::new("claude", "claude-sonnet");
        let args = client.build_args(1024);
        assert!(args.contains(&"--print".to_string()));
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"claude-sonnet".to_string()));
    }

    #[test]
    fn chatgpt_args() {
        let client = CliLLMClient::new("chatgpt", "gpt-4o");
        let args = client.build_args(1024);
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"gpt-4o".to_string()));
        assert!(!args.contains(&"--print".to_string()));
    }
}
