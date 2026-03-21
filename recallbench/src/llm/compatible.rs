use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::CustomEndpoint;
use crate::traits::LLMClient;

/// Generic OpenAI-compatible API client.
///
/// Works with any service that implements the OpenAI Chat Completions API:
/// - Local: Ollama, LM Studio, vLLM, llama.cpp
/// - Cloud: DeepInfra, Together, Fireworks, Groq, Replicate
pub struct CompatibleClient {
    base_url: String,
    api_key: Option<String>,
    model: String,
    display_name: String,
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

impl CompatibleClient {
    /// Create from a CustomEndpoint config section.
    pub fn from_config(name: &str, config: &CustomEndpoint) -> Result<Self> {
        // Try api_key_cmd first (e.g., "op read ..." or "security find-generic-password ...")
        let api_key = if let Some(ref cmd) = config.api_key_cmd {
            match std::process::Command::new("sh")
                .args(["-c", cmd])
                .output()
            {
                Ok(output) if output.status.success() => {
                    let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if key.is_empty() {
                        tracing::warn!("api_key_cmd for '{name}' returned empty");
                        None
                    } else {
                        Some(key)
                    }
                }
                Ok(output) => {
                    tracing::warn!("api_key_cmd for '{name}' failed: {}", String::from_utf8_lossy(&output.stderr).trim());
                    None
                }
                Err(e) => {
                    tracing::warn!("api_key_cmd for '{name}' error: {e}");
                    None
                }
            }
        } else if !config.api_key_env.is_empty() {
            match std::env::var(&config.api_key_env) {
                Ok(key) => Some(key),
                Err(_) => {
                    tracing::warn!(
                        "Env var {} not set for endpoint '{}', proceeding without auth",
                        config.api_key_env, name
                    );
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key,
            model: config.model.clone(),
            display_name: format!("{name}:{}", config.model),
            client: reqwest::Client::new(),
        })
    }

    /// Create directly with parameters.
    pub fn new(base_url: &str, api_key: Option<&str>, model: &str, name: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.map(|s| s.to_string()),
            model: model.to_string(),
            display_name: format!("{name}:{model}"),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LLMClient for CompatibleClient {
    fn name(&self) -> &str {
        &self.display_name
    }

    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        self.generate_with_seed(prompt, max_tokens, 0).await
    }

    async fn generate_with_seed(&self, prompt: &str, max_tokens: usize, seed: u64) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);

        let request = ChatRequest {
            model: self.model.clone(),
            max_tokens,
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            seed: if seed > 0 { Some(seed) } else { None },
        };

        let mut req = self.client.post(&url)
            .header("Content-Type", "application/json")
            .json(&request);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let response = req.send().await
            .with_context(|| format!("Failed to connect to {url}"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error ({status}) from {}: {body}", self.base_url);
        }

        let body: ChatResponse = response.json().await
            .context("Failed to parse API response")?;

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
    fn client_from_config() {
        let config = CustomEndpoint {
            base_url: "http://localhost:11434/v1".to_string(),
            api_key_env: "".to_string(),
            model: "llama3.1:70b".to_string(),
            rate_limit_rpm: 0,
            rate_limit_tpm: 0,
        };
        let client = CompatibleClient::from_config("local", &config).unwrap();
        assert_eq!(client.name(), "local:llama3.1:70b");
        assert!(client.api_key.is_none());
    }

    #[test]
    fn client_direct_construction() {
        let client = CompatibleClient::new(
            "https://api.deepinfra.com/v1/openai",
            Some("test-key"),
            "meta-llama/Llama-3.1-70B-Instruct",
            "deepinfra",
        );
        assert!(client.name().contains("deepinfra"));
        assert!(client.api_key.is_some());
    }

    #[test]
    fn base_url_trailing_slash_stripped() {
        let client = CompatibleClient::new(
            "http://localhost:11434/v1/",
            None,
            "test",
            "local",
        );
        assert_eq!(client.base_url, "http://localhost:11434/v1");
    }
}
