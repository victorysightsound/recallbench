//! Full pipeline test: extraction → storage → graph → search → context assembly.
//!
//! Modular — features can be toggled independently:
//! - extraction: on/off (off = use raw chunks instead of LLM extraction)
//! - graph: on/off (off = no graph edges)
//! - embedding: on/off (off = FTS5 only)
//! - recency: configurable weight
//!
//! Reports both extraction metrics and retrieval metrics.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::types::BenchmarkQuestion;

/// Configuration for a pipeline test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub dataset: String,
    pub variant: String,
    pub use_extraction: bool,
    pub use_graph: bool,
    pub use_embedding: bool,
    pub recency_weight: f32,
    pub max_per_session: usize,
    pub token_budget: usize,
    pub llm_model: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            dataset: "memoryagentbench".to_string(),
            variant: "conflict_resolution".to_string(),
            use_extraction: true,
            use_graph: true,
            use_embedding: true,
            recency_weight: 0.3,
            max_per_session: 0,
            token_budget: 16384,
            llm_model: "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string(),
        }
    }
}

/// Results from a full pipeline test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub config: PipelineConfig,
    pub extraction_stats: ExtractionStats,
    pub retrieval_stats: RetrievalStats,
    pub per_question: Vec<QuestionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionStats {
    pub total_facts: usize,
    pub total_entities: usize,
    pub total_relationships: usize,
    pub graph_edges: usize,
    pub extraction_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalStats {
    pub total_questions: usize,
    pub answer_in_context: usize,
    pub answer_accuracy: f64,
    pub avg_tokens_used: usize,
    pub avg_retrieval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionResult {
    pub question_id: String,
    pub question_text: String,
    pub ground_truth: String,
    pub answer_in_context: bool,
    pub tokens_used: usize,
    pub retrieval_ms: u64,
}

impl PipelineResult {
    pub fn print_report(&self) {
        println!("Pipeline Test: {} {} (extraction={}, graph={}, embedding={})",
            self.config.dataset, self.config.variant,
            self.config.use_extraction, self.config.use_graph, self.config.use_embedding);
        println!("═══════════════════════════════════════════════════════");

        println!("\nExtraction:");
        println!("  Facts: {}", self.extraction_stats.total_facts);
        println!("  Entities: {}", self.extraction_stats.total_entities);
        println!("  Relationships: {}", self.extraction_stats.total_relationships);
        println!("  Graph edges: {}", self.extraction_stats.graph_edges);
        println!("  Time: {}ms", self.extraction_stats.extraction_ms);

        println!("\nRetrieval:");
        println!("  Questions: {}", self.retrieval_stats.total_questions);
        println!("  Answer in context: {}/{} ({:.1}%)",
            self.retrieval_stats.answer_in_context,
            self.retrieval_stats.total_questions,
            self.retrieval_stats.answer_accuracy * 100.0);
        println!("  Avg tokens: {}", self.retrieval_stats.avg_tokens_used);
        println!("  Avg retrieval: {}ms", self.retrieval_stats.avg_retrieval_ms);

        // Show failures
        let failures: Vec<_> = self.per_question.iter()
            .filter(|q| !q.answer_in_context)
            .collect();
        if !failures.is_empty() {
            println!("\nMissed answers ({}):", failures.len());
            for q in failures.iter().take(10) {
                println!("  {} — GT: {}", &q.question_id[..q.question_id.len().min(15)],
                    &q.ground_truth[..q.ground_truth.len().min(40)]);
            }
        }
    }

    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        tracing::info!("Pipeline results saved to {}", path.display());
        Ok(())
    }
}
