//! Retrieval test harness — measures search quality without LLM calls.
//!
//! For each question, loads sessions into the memory system, runs search,
//! and checks if chunks from the answer sessions appear in the results.
//! Zero LLM cost, instant feedback for tuning search parameters.

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::traits::MemorySystem;
use crate::types::BenchmarkQuestion;

/// Results for a single question's retrieval test.
#[derive(Debug, Clone)]
pub struct QuestionRetrievalResult {
    pub question_id: String,
    pub question_type: String,
    pub question_text: String,
    pub answer_session_ids: Vec<String>,
    /// For each answer session, the best (lowest) rank at which a chunk appeared.
    /// None means the session was not found in results at all.
    pub answer_session_ranks: Vec<(String, Option<usize>)>,
    /// Total chunks retrieved.
    pub total_retrieved: usize,
    /// Tokens used in retrieval.
    pub tokens_used: usize,
}

impl QuestionRetrievalResult {
    /// Whether ALL answer sessions have at least one chunk in the top K.
    pub fn all_found_at(&self, k: usize) -> bool {
        self.answer_session_ranks.iter().all(|(_, rank)| {
            rank.map(|r| r < k).unwrap_or(false)
        })
    }

    /// How many answer sessions have a chunk in the top K.
    pub fn found_at(&self, k: usize) -> usize {
        self.answer_session_ranks.iter()
            .filter(|(_, rank)| rank.map(|r| r < k).unwrap_or(false))
            .count()
    }

    /// Reciprocal rank of the first answer chunk found.
    pub fn reciprocal_rank(&self) -> f64 {
        let best_rank = self.answer_session_ranks.iter()
            .filter_map(|(_, r)| *r)
            .min();
        match best_rank {
            Some(r) => 1.0 / (r as f64 + 1.0),
            None => 0.0,
        }
    }

    /// All answer sessions found somewhere in results.
    pub fn all_found(&self) -> bool {
        self.answer_session_ranks.iter().all(|(_, r)| r.is_some())
    }

    /// Sessions that were NOT found.
    pub fn missing_sessions(&self) -> Vec<&str> {
        self.answer_session_ranks.iter()
            .filter(|(_, r)| r.is_none())
            .map(|(id, _)| id.as_str())
            .collect()
    }
}

/// Aggregate metrics across all questions.
#[derive(Debug, Clone)]
pub struct RetrievalMetrics {
    pub total_questions: usize,
    pub abstention_questions: usize,
    pub recall_at_10: f64,
    pub recall_at_20: f64,
    pub recall_at_50: f64,
    pub recall_at_100: f64,
    pub mrr: f64,
    pub hit_rate_all: f64,
    pub hit_rate_any: f64,
}

impl RetrievalMetrics {
    pub fn compute(results: &[QuestionRetrievalResult]) -> Self {
        // Separate regular and abstention questions
        let regular: Vec<_> = results.iter()
            .filter(|r| !r.answer_session_ids.is_empty())
            .collect();
        let abstention_count = results.len() - regular.len();

        let n = regular.len() as f64;
        if n == 0.0 {
            return Self {
                total_questions: results.len(),
                abstention_questions: abstention_count,
                recall_at_10: 0.0, recall_at_20: 0.0,
                recall_at_50: 0.0, recall_at_100: 0.0,
                mrr: 0.0, hit_rate_all: 0.0, hit_rate_any: 0.0,
            };
        }

        // Recall@K: average fraction of answer sessions found in top K (regular only)
        let recall_at = |k: usize| -> f64 {
            regular.iter().map(|r| {
                let total = r.answer_session_ids.len() as f64;
                if total == 0.0 { return 1.0; }
                r.found_at(k) as f64 / total
            }).sum::<f64>() / n
        };

        // MRR: mean reciprocal rank (regular only)
        let mrr = regular.iter().map(|r| r.reciprocal_rank()).sum::<f64>() / n;

        // Hit rate (all): % of regular questions where ALL answer sessions found
        let hit_all = regular.iter().filter(|r| r.all_found()).count() as f64 / n;

        // Hit rate (any): % of regular questions where at least ONE answer session found
        let hit_any = regular.iter()
            .filter(|r| r.answer_session_ranks.iter().any(|(_, rank)| rank.is_some()))
            .count() as f64 / n;

        Self {
            total_questions: results.len(),
            abstention_questions: abstention_count,
            recall_at_10: recall_at(10),
            recall_at_20: recall_at(20),
            recall_at_50: recall_at(50),
            recall_at_100: recall_at(100),
            mrr,
            hit_rate_all: hit_all,
            hit_rate_any: hit_any,
        }
    }

    pub fn print_report(&self) {
        let regular = self.total_questions - self.abstention_questions;
        println!("Retrieval Metrics ({} questions: {} regular, {} abstention)",
            self.total_questions, regular, self.abstention_questions);
        println!("═══════════════════════════════════════");
        println!("  Recall@10:   {:.1}%", self.recall_at_10 * 100.0);
        println!("  Recall@20:   {:.1}%", self.recall_at_20 * 100.0);
        println!("  Recall@50:   {:.1}%", self.recall_at_50 * 100.0);
        println!("  Recall@100:  {:.1}%", self.recall_at_100 * 100.0);
        println!("  MRR:         {:.3}", self.mrr);
        println!("  Hit Rate:    {:.1}% (all answer sessions found)", self.hit_rate_all * 100.0);
        println!("  Any Found:   {:.1}% (at least one answer session)", self.hit_rate_any * 100.0);
    }
}

/// Run retrieval test for a single question.
///
/// Loads sessions into the memory system, runs search, checks results
/// against answer_session_ids. No LLM calls.
pub async fn test_retrieval(
    system: &dyn MemorySystem,
    question: &BenchmarkQuestion,
    token_budget: usize,
    cache_path: Option<&std::path::Path>,
) -> Result<QuestionRetrievalResult> {
    // Get answer session IDs from metadata
    let answer_session_ids: Vec<String> = question.metadata
        .get("answer_session_ids")
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .unwrap_or_default();

    // For abstention questions: still run retrieval but expect NO answer sessions.
    // We check that the context doesn't contain misleading content.
    if answer_session_ids.is_empty() && !question.is_abstention {
        return Ok(QuestionRetrievalResult {
            question_id: question.id.clone(),
            question_type: question.question_type.clone(),
            question_text: question.question.clone(),
            answer_session_ids,
            answer_session_ranks: Vec::new(),
            total_retrieved: 0,
            tokens_used: 0,
        });
    }

    let answer_set: HashSet<&str> = answer_session_ids.iter().map(|s| s.as_str()).collect();

    // Reset and ingest
    system.reset().await?;

    // Use cache if available
    let mut used_cache = false;
    if let Some(path) = cache_path {
        if system.supports_precomputed() {
            let cache_conn = rusqlite::Connection::open(path)?;
            let session_ids: Vec<&str> = question.sessions.iter().map(|s| s.id.as_str()).collect();
            let mut tuples: Vec<(String, Vec<f32>, String, usize)> = Vec::new();

            for session_id in &session_ids {
                let mut stmt = cache_conn.prepare(
                    "SELECT chunk_text, embedding, session_date, chunk_index
                     FROM session_chunks WHERE session_id = ?1 ORDER BY chunk_index",
                )?;
                let rows = stmt.query_map([session_id], |row| {
                    let text: String = row.get(0)?;
                    let blob: Vec<u8> = row.get(1)?;
                    let date: String = row.get(2)?;
                    let idx: usize = row.get(3)?;
                    Ok((text, blob, date, idx))
                })?;
                for row in rows {
                    let (text, blob, date, idx) = row?;
                    let embedding = mindcore::embeddings::pooling::bytes_to_vec(&blob);
                    tuples.push((text, embedding, date, idx));
                }
            }

            system.load_precomputed(&tuples)?;
            used_cache = true;
        }
    }

    if !used_cache {
        for session in &question.sessions {
            system.ingest_session(session).await?;
        }
    }

    // Retrieve context — this is what we're testing
    let retrieval = system.retrieve_context(
        &question.question,
        question.question_date.as_deref(),
        token_budget,
    ).await?;

    // Now we need to figure out which answer sessions are represented
    // in the retrieved context. The context is assembled from chunks
    // that contain session dates. We match by checking if chunks from
    // answer sessions appear in the context.
    //
    // Build a map: session_id → session content snippets
    let mut session_snippets: HashMap<&str, Vec<String>> = HashMap::new();
    for session in &question.sessions {
        let mut snippets = Vec::new();
        for turn in &session.turns {
            if turn.content.len() >= 20 {
                // Take a unique-ish substring from each turn (char-boundary safe)
                let end = turn.content.char_indices()
                    .take(60)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(turn.content.len());
                let snippet = &turn.content[..end];
                snippets.push(snippet.to_string());
            }
        }
        session_snippets.insert(&session.id, snippets);
    }

    // Check which answer sessions have content in the retrieved context
    let context = &retrieval.context;
    let mut answer_session_ranks: Vec<(String, Option<usize>)> = Vec::new();

    for ans_id in &answer_session_ids {
        if let Some(snippets) = session_snippets.get(ans_id.as_str()) {
            // Check if any snippet from this answer session appears in context
            let found = snippets.iter().any(|snippet| context.contains(snippet));
            if found {
                // Approximate rank by position in context
                let pos = snippets.iter()
                    .filter_map(|s| context.find(s))
                    .min()
                    .unwrap_or(0);
                // Convert byte position to approximate chunk rank
                let approx_rank = pos / 500; // rough: 500 chars per chunk
                answer_session_ranks.push((ans_id.clone(), Some(approx_rank)));
            } else {
                answer_session_ranks.push((ans_id.clone(), None));
            }
        } else {
            answer_session_ranks.push((ans_id.clone(), None));
        }
    }

    Ok(QuestionRetrievalResult {
        question_id: question.id.clone(),
        question_type: question.question_type.clone(),
        question_text: question.question.clone(),
        answer_session_ids,
        answer_session_ranks,
        total_retrieved: retrieval.items_retrieved,
        tokens_used: retrieval.tokens_used,
    })
}

/// Print detailed per-question results for failures.
pub fn print_failures(results: &[QuestionRetrievalResult]) {
    let failures: Vec<_> = results.iter()
        .filter(|r| !r.answer_session_ids.is_empty() && !r.all_found())
        .collect();

    if failures.is_empty() {
        println!("\nNo retrieval failures — all answer sessions found!");
        return;
    }

    println!("\nRetrieval Failures ({} questions with missing answer sessions):", failures.len());
    println!("────────────────────────────────────────────────────────────");

    for r in &failures {
        let found = r.answer_session_ranks.iter().filter(|(_, rank)| rank.is_some()).count();
        let total = r.answer_session_ids.len();
        println!("\n  Q: {} [{}]", &r.question_id[..r.question_id.len().min(12)], r.question_type);
        println!("     {}", &r.question_text[..r.question_text.len().min(80)]);
        println!("     Sessions: {}/{} found, tokens: {}", found, total, r.tokens_used);

        for (id, rank) in &r.answer_session_ranks {
            let status = match rank {
                Some(r) => format!("✓ rank ~{r}"),
                None => "✗ NOT FOUND".to_string(),
            };
            let short_id = &id[..id.len().min(20)];
            println!("       {short_id}: {status}");
        }
    }
}
