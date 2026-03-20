use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::traits::LLMClient;

/// Claude API client via direct HTTP (no SDK dependency).
///
/// Supports both CLI subscription mode (via CliLLMClient) and direct API mode.
/// This module implements the API mode.
pub struct AnthropicClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

impl AnthropicClient {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Create from ANTHROPIC_API_KEY environment variable.
    pub fn from_env(model: &str) -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY not set. Use CLI mode (default) or set the env var.")?;
        Ok(Self::new(&api_key, model))
    }

    fn resolve_model(&self) -> &str {
        match self.model.as_str() {
            "claude-sonnet" => "claude-sonnet-4-6",
            "claude-opus" => "claude-opus-4-6",
            "claude-haiku" => "claude-haiku-4-5-20251001",
            other => other,
        }
    }
}

#[async_trait]
impl LLMClient for AnthropicClient {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let request = MessagesRequest {
            model: self.resolve_model().to_string(),
            max_tokens,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            metadata: None,
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error ({}): {}", status, body);
        }

        let body: MessagesResponse = response.json().await
            .context("Failed to parse Anthropic API response")?;

        let text = body.content.iter()
            .filter_map(|block| block.text.as_deref())
            .collect::<Vec<_>>()
            .join("");

        Ok(text.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_resolution() {
        let client = AnthropicClient::new("test-key", "claude-sonnet");
        assert_eq!(client.resolve_model(), "claude-sonnet-4-6");

        let client = AnthropicClient::new("test-key", "claude-opus");
        assert_eq!(client.resolve_model(), "claude-opus-4-6");

        let client = AnthropicClient::new("test-key", "claude-sonnet-4-6");
        assert_eq!(client.resolve_model(), "claude-sonnet-4-6");
    }

    #[test]
    fn client_name() {
        let client = AnthropicClient::new("test-key", "claude-sonnet");
        assert_eq!(client.name(), "claude-sonnet");
    }
}
