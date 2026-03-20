use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::traits::LLMClient;

/// Google Gemini API client via direct HTTP.
pub struct GeminiClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct GenerateRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    max_output_tokens: usize,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Option<Vec<CandidatePart>>,
}

#[derive(Deserialize)]
struct CandidatePart {
    text: Option<String>,
}

impl GeminiClient {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env(model: &str) -> Result<Self> {
        let api_key = std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .context("GEMINI_API_KEY or GOOGLE_API_KEY not set. Use CLI mode or set the env var.")?;
        Ok(Self::new(&api_key, model))
    }

    fn resolve_model(&self) -> &str {
        match self.model.as_str() {
            "gemini-pro" => "gemini-2.5-pro",
            "gemini-flash" => "gemini-2.5-flash",
            other => other,
        }
    }
}

#[async_trait]
impl LLMClient for GeminiClient {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let model = self.resolve_model();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );

        let request = GenerateRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt.to_string() }],
            }],
            generation_config: Some(GenerationConfig {
                max_output_tokens: max_tokens,
            }),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Gemini API error ({}): {}", status, body);
        }

        let body: GenerateResponse = response.json().await
            .context("Failed to parse Gemini API response")?;

        let text = body.candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .and_then(|p| p.into_iter().next())
            .and_then(|p| p.text)
            .unwrap_or_default();

        Ok(text.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_resolution() {
        let client = GeminiClient::new("test-key", "gemini-pro");
        assert_eq!(client.resolve_model(), "gemini-2.5-pro");
    }

    #[test]
    fn client_name() {
        let client = GeminiClient::new("test-key", "gemini-flash");
        assert_eq!(client.name(), "gemini-flash");
    }
}
