//! Extraction test harness — measures LLM fact extraction quality.
//!
//! Feeds raw text through LlmIngest, reports what was extracted.
//! Tests extraction quality independently from search quality.
//! LLM cost for extraction, zero cost for analysis.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Results from extracting facts from a single text segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionTestResult {
    pub source_id: String,
    pub source_chars: usize,
    pub facts_extracted: usize,
    pub entities_found: Vec<String>,
    pub relationships_found: Vec<(String, String, String)>,
    pub categories: std::collections::HashMap<String, usize>,
    pub avg_importance: f32,
    pub extraction_ms: u64,
}

/// Aggregate metrics across all extractions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionMetrics {
    pub total_sources: usize,
    pub total_facts: usize,
    pub total_entities: usize,
    pub total_relationships: usize,
    pub facts_per_source: f64,
    pub entities_per_fact: f64,
    pub relationships_per_fact: f64,
    pub category_distribution: std::collections::HashMap<String, usize>,
    pub avg_importance: f32,
    pub total_extraction_ms: u64,
}

impl ExtractionMetrics {
    pub fn compute(results: &[ExtractionTestResult]) -> Self {
        let total_sources = results.len();
        let total_facts: usize = results.iter().map(|r| r.facts_extracted).sum();
        let total_entities: usize = results.iter().map(|r| r.entities_found.len()).sum();
        let total_relationships: usize = results.iter().map(|r| r.relationships_found.len()).sum();
        let total_extraction_ms: u64 = results.iter().map(|r| r.extraction_ms).sum();

        let mut category_distribution = std::collections::HashMap::new();
        for r in results {
            for (cat, count) in &r.categories {
                *category_distribution.entry(cat.clone()).or_insert(0) += count;
            }
        }

        let avg_importance = if total_sources > 0 {
            results.iter().map(|r| r.avg_importance).sum::<f32>() / total_sources as f32
        } else {
            0.0
        };

        Self {
            total_sources,
            total_facts,
            total_entities,
            total_relationships,
            facts_per_source: if total_sources > 0 { total_facts as f64 / total_sources as f64 } else { 0.0 },
            entities_per_fact: if total_facts > 0 { total_entities as f64 / total_facts as f64 } else { 0.0 },
            relationships_per_fact: if total_facts > 0 { total_relationships as f64 / total_facts as f64 } else { 0.0 },
            category_distribution,
            avg_importance,
            total_extraction_ms,
        }
    }

    pub fn print_report(&self) {
        println!("Extraction Metrics ({} sources)", self.total_sources);
        println!("═══════════════════════════════════════");
        println!("  Total facts:          {}", self.total_facts);
        println!("  Total entities:       {}", self.total_entities);
        println!("  Total relationships:  {}", self.total_relationships);
        println!("  Facts/source:         {:.1}", self.facts_per_source);
        println!("  Entities/fact:        {:.2}", self.entities_per_fact);
        println!("  Relationships/fact:   {:.2}", self.relationships_per_fact);
        println!("  Avg importance:       {:.1}", self.avg_importance);
        println!("  Extraction time:      {}ms", self.total_extraction_ms);

        if !self.category_distribution.is_empty() {
            println!("\n  Categories:");
            let mut cats: Vec<_> = self.category_distribution.iter().collect();
            cats.sort_by(|a, b| b.1.cmp(a.1));
            for (cat, count) in cats {
                println!("    {:<20} {}", cat, count);
            }
        }
    }
}

/// Run extraction test on a text segment.
pub fn test_extraction(
    source_id: &str,
    text: &str,
    llm: &dyn mindcore::traits::LlmCallback,
) -> Result<ExtractionTestResult> {
    let start = std::time::Instant::now();
    let result = mindcore::ingest::llm_extract::extract_facts(text, llm)?;
    let elapsed = start.elapsed().as_millis() as u64;

    let mut entities: Vec<String> = Vec::new();
    let mut relationships: Vec<(String, String, String)> = Vec::new();
    let mut categories: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut importance_sum = 0u32;

    for fact in &result.facts {
        entities.extend(fact.entities.clone());
        relationships.extend(fact.relationships.clone());
        *categories.entry(fact.category.clone()).or_insert(0) += 1;
        importance_sum += fact.importance as u32;
    }

    // Deduplicate entities
    entities.sort();
    entities.dedup();

    let avg_importance = if result.facts.is_empty() {
        0.0
    } else {
        importance_sum as f32 / result.facts.len() as f32
    };

    Ok(ExtractionTestResult {
        source_id: source_id.to_string(),
        source_chars: text.len(),
        facts_extracted: result.facts.len(),
        entities_found: entities,
        relationships_found: relationships,
        categories,
        avg_importance,
        extraction_ms: elapsed,
    })
}

/// Save extraction test results to JSON.
pub fn save_results(
    results: &[ExtractionTestResult],
    path: &std::path::Path,
) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    std::fs::write(path, json)?;
    tracing::info!("Extraction results saved to {}", path.display());
    Ok(())
}
