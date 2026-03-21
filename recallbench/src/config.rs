use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Top-level configuration loaded from `recallbench.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub llm: LlmConfig,
}

/// Default settings for benchmark runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_token_budget")]
    pub token_budget: usize,
    #[serde(default = "default_gen_model")]
    pub gen_model: String,
    #[serde(default = "default_judge_model")]
    pub judge_model: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default = "default_quick_size")]
    pub quick_size: usize,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            token_budget: default_token_budget(),
            gen_model: default_gen_model(),
            judge_model: default_judge_model(),
            output_dir: default_output_dir(),
            seed: None,
            quick_size: default_quick_size(),
        }
    }
}

fn default_quick_size() -> usize { 50 }

fn default_concurrency() -> usize { 10 }
fn default_token_budget() -> usize { 16384 }
fn default_gen_model() -> String { "claude-sonnet".to_string() }
fn default_judge_model() -> String { "claude-sonnet".to_string() }
fn default_output_dir() -> String { "results".to_string() }

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmConfig {
    #[serde(default)]
    pub anthropic: ProviderConfig,
    #[serde(default)]
    pub openai: ProviderConfig,
    #[serde(default)]
    pub gemini: ProviderConfig,
    /// Custom OpenAI-compatible endpoint.
    #[serde(default)]
    pub custom: Option<CustomEndpoint>,
    /// Local inference endpoint (Ollama, vLLM, etc.).
    #[serde(default)]
    pub local: Option<CustomEndpoint>,
}

/// Configuration for an OpenAI-compatible custom endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEndpoint {
    pub base_url: String,
    #[serde(default)]
    pub api_key_env: String,
    /// Shell command to fetch API key at runtime (e.g., "op read op://Personal/DeepInfra/credential")
    #[serde(default)]
    pub api_key_cmd: Option<String>,
    pub model: String,
    #[serde(default)]
    pub rate_limit_rpm: u32,
    #[serde(default)]
    pub rate_limit_tpm: u32,
}

/// Per-provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// "cli" or "api". CLI uses subscription (no API key), API uses direct HTTP.
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_rpm")]
    pub rate_limit_rpm: u32,
    #[serde(default = "default_tpm")]
    pub rate_limit_tpm: u32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            rate_limit_rpm: default_rpm(),
            rate_limit_tpm: default_tpm(),
        }
    }
}

fn default_mode() -> String { "cli".to_string() }
fn default_rpm() -> u32 { 60 }
fn default_tpm() -> u32 { 100_000 }

impl Config {
    /// Load configuration from a TOML file, falling back to defaults.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Load from the default path (`recallbench.toml` in the current directory).
    pub fn load_default() -> anyhow::Result<Self> {
        Self::load(&PathBuf::from("recallbench.toml"))
    }

    /// Apply environment variable overrides.
    pub fn with_env_overrides(mut self) -> Self {
        if let Ok(val) = std::env::var("RECALLBENCH_CONCURRENCY") {
            if let Ok(n) = val.parse() {
                self.defaults.concurrency = n;
            }
        }
        if let Ok(val) = std::env::var("RECALLBENCH_TOKEN_BUDGET") {
            if let Ok(n) = val.parse() {
                self.defaults.token_budget = n;
            }
        }
        if let Ok(val) = std::env::var("RECALLBENCH_GEN_MODEL") {
            self.defaults.gen_model = val;
        }
        if let Ok(val) = std::env::var("RECALLBENCH_JUDGE_MODEL") {
            self.defaults.judge_model = val;
        }
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            defaults: Defaults::default(),
            llm: LlmConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = Config::default();
        assert_eq!(config.defaults.concurrency, 10);
        assert_eq!(config.defaults.token_budget, 16384);
        assert_eq!(config.defaults.gen_model, "claude-sonnet");
        assert_eq!(config.defaults.judge_model, "claude-sonnet");
        assert_eq!(config.defaults.output_dir, "results");
        assert!(config.defaults.seed.is_none());
        assert_eq!(config.llm.anthropic.mode, "cli");
        assert_eq!(config.llm.anthropic.rate_limit_rpm, 60);
    }

    #[test]
    fn parse_toml() {
        let toml_str = r#"
[defaults]
concurrency = 20
token_budget = 8192
gen_model = "chatgpt-4o"
judge_model = "claude-opus"
output_dir = "output"
seed = 42

[llm.anthropic]
mode = "api"
rate_limit_rpm = 120

[llm.openai]
mode = "cli"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.defaults.concurrency, 20);
        assert_eq!(config.defaults.token_budget, 8192);
        assert_eq!(config.defaults.gen_model, "chatgpt-4o");
        assert_eq!(config.defaults.seed, Some(42));
        assert_eq!(config.llm.anthropic.mode, "api");
        assert_eq!(config.llm.anthropic.rate_limit_rpm, 120);
        assert_eq!(config.llm.openai.mode, "cli");
    }

    #[test]
    fn parse_empty_toml() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.defaults.concurrency, 10);
        assert_eq!(config.llm.anthropic.mode, "cli");
    }

    #[test]
    fn load_nonexistent_file() {
        let config = Config::load(Path::new("/nonexistent/recallbench.toml")).unwrap();
        assert_eq!(config.defaults.concurrency, 10);
    }

    #[test]
    fn env_overrides() {
        // SAFETY: Test runs single-threaded; no other threads read this var.
        unsafe { std::env::set_var("RECALLBENCH_CONCURRENCY", "5") };
        let config = Config::default().with_env_overrides();
        assert_eq!(config.defaults.concurrency, 5);
        unsafe { std::env::remove_var("RECALLBENCH_CONCURRENCY") };
    }
}
