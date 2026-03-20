pub mod anthropic;
pub mod cli;
pub mod codex;
pub mod compatible;
pub mod gemini;
pub mod openai;
pub mod rate_limit;

use std::collections::HashMap;

use anyhow::Result;

use crate::traits::LLMClient;

/// Registry mapping model names to LLM client constructors.
pub struct LLMRegistry {
    providers: HashMap<String, ProviderInfo>,
}

#[derive(Debug, Clone)]
struct ProviderInfo {
    prefix: String,
    mode: ProviderMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProviderMode {
    Cli,
    Api,
}

impl LLMRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Create a registry with default provider mappings.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register("claude", ProviderMode::Cli);
        registry.register("chatgpt", ProviderMode::Cli);
        registry.register("gemini", ProviderMode::Cli);
        registry.register("codex", ProviderMode::Cli);
        registry
    }

    fn register(&mut self, prefix: &str, mode: ProviderMode) {
        self.providers.insert(prefix.to_string(), ProviderInfo {
            prefix: prefix.to_string(),
            mode,
        });
    }

    /// Resolve a model string (e.g., "claude-sonnet") to a provider prefix and model.
    pub fn resolve_provider(model: &str) -> (&str, &str) {
        if let Some(rest) = model.strip_prefix("claude-") {
            ("claude", rest)
        } else if model.starts_with("claude") {
            ("claude", model)
        } else if let Some(rest) = model.strip_prefix("chatgpt-") {
            ("chatgpt", rest)
        } else if model.strip_prefix("gpt-").is_some() {
            ("chatgpt", model)
        } else if let Some(rest) = model.strip_prefix("gemini-") {
            ("gemini", rest)
        } else if model.starts_with("codex") {
            ("codex", model)
        } else {
            // Default to claude
            ("claude", model)
        }
    }

    /// Create an LLM client for a given model string and mode.
    pub fn create_client(model: &str, mode: ProviderMode) -> Result<Box<dyn LLMClient>> {
        let (provider, _model_name) = Self::resolve_provider(model);

        match (provider, mode) {
            ("claude", ProviderMode::Cli) => {
                Ok(Box::new(cli::CliLLMClient::new("claude", model)))
            }
            ("claude", ProviderMode::Api) => {
                Ok(Box::new(anthropic::AnthropicClient::from_env(model)?))
            }
            ("chatgpt" | "gpt", ProviderMode::Cli) => {
                Ok(Box::new(cli::CliLLMClient::new("chatgpt", model)))
            }
            ("chatgpt" | "gpt", ProviderMode::Api) => {
                Ok(Box::new(openai::OpenAIClient::from_env(model)?))
            }
            ("gemini", ProviderMode::Cli) => {
                Ok(Box::new(cli::CliLLMClient::new("gemini", model)))
            }
            ("gemini", ProviderMode::Api) => {
                Ok(Box::new(gemini::GeminiClient::from_env(model)?))
            }
            ("codex", _) => {
                Ok(Box::new(codex::CodexClient::new(model)))
            }
            _ => anyhow::bail!("Unknown provider for model: {model}"),
        }
    }
}

impl Default for LLMRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Detect which CLI providers are installed on the system.
pub fn detect_installed_providers() -> Vec<String> {
    let mut installed = Vec::new();
    for cmd in ["claude", "chatgpt", "codex", "gemini"] {
        if which::which(cmd).is_ok() {
            installed.push(cmd.to_string());
        }
    }
    installed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_claude_models() {
        assert_eq!(LLMRegistry::resolve_provider("claude-sonnet"), ("claude", "sonnet"));
        assert_eq!(LLMRegistry::resolve_provider("claude-opus"), ("claude", "opus"));
    }

    #[test]
    fn resolve_chatgpt_models() {
        assert_eq!(LLMRegistry::resolve_provider("chatgpt-4o"), ("chatgpt", "4o"));
        assert_eq!(LLMRegistry::resolve_provider("gpt-4o"), ("chatgpt", "gpt-4o"));
    }

    #[test]
    fn resolve_gemini_models() {
        assert_eq!(LLMRegistry::resolve_provider("gemini-pro"), ("gemini", "pro"));
    }

    #[test]
    fn resolve_codex() {
        assert_eq!(LLMRegistry::resolve_provider("codex"), ("codex", "codex"));
    }

    #[test]
    fn resolve_unknown_defaults_to_claude() {
        assert_eq!(LLMRegistry::resolve_provider("something"), ("claude", "something"));
    }

    #[test]
    fn registry_defaults() {
        let registry = LLMRegistry::with_defaults();
        assert!(registry.providers.contains_key("claude"));
        assert!(registry.providers.contains_key("chatgpt"));
        assert!(registry.providers.contains_key("gemini"));
        assert!(registry.providers.contains_key("codex"));
    }
}
