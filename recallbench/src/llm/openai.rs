use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::traits::LLMClient;

/// OpenAI API client via direct HTTP.
pub struct OpenAIClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

impl OpenAIClient {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env(model: &str) -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY not set. Use CLI mode or set the env var.")?;
        Ok(Self::new(&api_key, model))
    }
}

#[async_trait]
impl LLMClient for OpenAIClient {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            max_tokens,
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            seed: None,
        };

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error ({}): {}", status, body);
        }

        let body: ChatResponse = response.json().await
            .context("Failed to parse OpenAI API response")?;

        let text = body.choices.first()
            .and_then(|c| c.message.content.as_ref())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        Ok(text)
    }

    async fn generate_with_seed(&self, prompt: &str, max_tokens: usize, seed: u64) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            max_tokens,
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            seed: Some(seed),
        };

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error ({}): {}", status, body);
        }

        let body: ChatResponse = response.json().await
            .context("Failed to parse OpenAI API response")?;

        let text = body.choices.first()
            .and_then(|c| c.message.content.as_ref())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_name() {
        let client = OpenAIClient::new("test-key", "gpt-4o");
        assert_eq!(client.name(), "gpt-4o");
    }
}
