use anyhow::Result;
use async_trait::async_trait;

use crate::types::{BenchmarkQuestion, ConversationSession, IngestStats, RetrievalResult};

/// The primary trait any memory system must implement to be benchmarked.
///
/// Memory systems are evaluated by ingesting conversation sessions, then
/// retrieving relevant context for questions. RecallBench handles the
/// LLM generation and judging steps.
#[async_trait]
pub trait MemorySystem: Send + Sync {
    /// Human-readable name for reports (e.g., "MindCore", "Mem0").
    fn name(&self) -> &str;

    /// Version string for reproducibility (e.g., "0.1.0").
    fn version(&self) -> &str;

    /// Reset all state. Called between questions for isolation.
    async fn reset(&self) -> Result<()>;

    /// Ingest a conversation session into the memory system.
    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats>;

    /// Retrieve relevant context for a question within a token budget.
    ///
    /// The memory system should search its stored memories using the query,
    /// optionally considering the query date for temporal reasoning, and
    /// assemble context that fits within the token budget.
    async fn retrieve_context(
        &self,
        query: &str,
        query_date: Option<&str>,
        token_budget: usize,
    ) -> Result<RetrievalResult>;
}

/// Abstraction over LLM providers for answer generation and judging.
///
/// Supports both CLI subscription modes (e.g., `claude --print`) and
/// direct HTTP API modes. Provider selection is handled by the registry.
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// Provider name for logging (e.g., "claude-sonnet", "chatgpt-4o").
    fn name(&self) -> &str;

    /// Generate a response to a prompt.
    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String>;

    /// Generate a response with a deterministic seed (where supported).
    /// Falls back to regular generate if the provider doesn't support seeding.
    async fn generate_with_seed(
        &self,
        prompt: &str,
        max_tokens: usize,
        seed: u64,
    ) -> Result<String> {
        let _ = seed;
        self.generate(prompt, max_tokens).await
    }
}

/// A benchmark dataset that can be loaded and iterated.
///
/// All datasets normalize their questions into the universal `BenchmarkQuestion`
/// format, allowing the runner to handle them uniformly.
pub trait BenchmarkDataset: Send + Sync {
    /// Dataset name (e.g., "longmemeval", "locomo").
    fn name(&self) -> &str;

    /// Dataset variant (e.g., "oracle", "small", "medium").
    fn variant(&self) -> &str;

    /// Human-readable description of the dataset.
    fn description(&self) -> &str;

    /// All questions in the dataset.
    fn questions(&self) -> &[BenchmarkQuestion];

    /// List of unique question types in this dataset.
    fn question_types(&self) -> Vec<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify traits are object-safe (can be used as trait objects).
    fn _assert_memory_system_object_safety(_: &dyn MemorySystem) {}
    fn _assert_llm_client_object_safety(_: &dyn LLMClient) {}
    fn _assert_dataset_object_safety(_: &dyn BenchmarkDataset) {}

    // Verify trait objects can be boxed.
    fn _assert_boxable(
        _ms: Box<dyn MemorySystem>,
        _llm: Box<dyn LLMClient>,
        _ds: Box<dyn BenchmarkDataset>,
    ) {}
}
