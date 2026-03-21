use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use serde::Serialize;

use crate::judge;
use crate::traits::{LLMClient, MemorySystem};
use crate::types::{BenchmarkQuestion, ConversationSession, Turn};

/// Configuration for a longevity test.
pub struct LongevityConfig {
    /// Total sessions to ingest
    pub total_sessions: usize,
    /// Number of checkpoints (evaluation points)
    pub checkpoints: usize,
    /// Number of questions to evaluate at each checkpoint
    pub eval_questions: usize,
    /// Token budget for retrieval
    pub token_budget: usize,
}

/// Result at a single checkpoint.
#[derive(Debug, Clone, Serialize)]
pub struct CheckpointResult {
    pub sessions_ingested: usize,
    pub estimated_memories: usize,
    pub accuracy: f64,
    pub correct: usize,
    pub total: usize,
    pub avg_retrieval_latency_ms: f64,
    pub avg_ingest_latency_ms: f64,
}

/// Full longevity test results.
#[derive(Debug, Serialize)]
pub struct LongevityResult {
    pub system_name: String,
    pub checkpoints: Vec<CheckpointResult>,
}

/// Run a longitudinal degradation test.
///
/// Ingests sessions incrementally and evaluates at regular intervals.
pub async fn run_longevity(
    system: &dyn MemorySystem,
    gen_llm: Arc<dyn LLMClient>,
    judge_llm: Arc<dyn LLMClient>,
    config: &LongevityConfig,
) -> Result<LongevityResult> {
    system.reset().await?;

    let sessions = generate_sessions(config.total_sessions);
    let eval_questions = generate_eval_questions(config.eval_questions);

    let sessions_per_checkpoint = config.total_sessions / config.checkpoints.max(1);
    let mut checkpoints = Vec::new();
    let mut total_ingested = 0usize;
    let mut total_memories = 0usize;

    for checkpoint_idx in 0..config.checkpoints {
        let start_idx = checkpoint_idx * sessions_per_checkpoint;
        let end_idx = ((checkpoint_idx + 1) * sessions_per_checkpoint).min(sessions.len());

        // Ingest batch
        let mut batch_ingest_ms = 0u64;
        for session in &sessions[start_idx..end_idx] {
            let start = Instant::now();
            let stats = system.ingest_session(session).await?;
            batch_ingest_ms += start.elapsed().as_millis() as u64;
            total_memories += stats.memories_stored;
            total_ingested += 1;
        }

        let avg_ingest_ms = if total_ingested > 0 {
            batch_ingest_ms as f64 / (end_idx - start_idx) as f64
        } else {
            0.0
        };

        // Evaluate
        let mut correct = 0usize;
        let mut total_retrieval_ms = 0u64;

        for question in &eval_questions {
            let start = Instant::now();
            let retrieval = system.retrieve_context(
                &question.question, None, config.token_budget,
            ).await?;
            total_retrieval_ms += start.elapsed().as_millis() as u64;

            let prompt = crate::runner::build_generation_prompt(
                &retrieval.context, &question.question, None,
                &question.question_type, question.is_abstention,
            );
            let hypothesis = gen_llm.generate(&prompt, 256).await?;
            let ground_truth = question.ground_truth.join(", ");

            let is_correct = judge::judge_answer(
                &question.question_type, &question.question,
                &ground_truth, &hypothesis,
                question.is_abstention, judge_llm.as_ref(),
            ).await?;

            if is_correct { correct += 1; }
        }

        let total_eval = eval_questions.len();
        let accuracy = if total_eval > 0 { correct as f64 / total_eval as f64 } else { 0.0 };
        let avg_retrieval = if total_eval > 0 { total_retrieval_ms as f64 / total_eval as f64 } else { 0.0 };

        tracing::info!(
            "Checkpoint {}/{}: {} sessions, {:.1}% accuracy, {:.1}ms avg retrieval",
            checkpoint_idx + 1, config.checkpoints, total_ingested, accuracy * 100.0, avg_retrieval,
        );

        checkpoints.push(CheckpointResult {
            sessions_ingested: total_ingested,
            estimated_memories: total_memories,
            accuracy,
            correct,
            total: total_eval,
            avg_retrieval_latency_ms: avg_retrieval,
            avg_ingest_latency_ms: avg_ingest_ms,
        });
    }

    Ok(LongevityResult {
        system_name: system.name().to_string(),
        checkpoints,
    })
}

/// Generate synthetic conversation sessions for longevity testing.
fn generate_sessions(count: usize) -> Vec<ConversationSession> {
    let topics = [
        "weather", "food", "travel", "work", "hobbies",
        "movies", "books", "music", "sports", "technology",
    ];

    (0..count).map(|i| {
        let topic = topics[i % topics.len()];
        ConversationSession {
            id: format!("longevity_session_{i}"),
            date: Some(format!("2024/{:02}/{:02}", (i / 28) % 12 + 1, (i % 28) + 1)),
            turns: vec![
                Turn {
                    role: "user".to_string(),
                    content: format!("Let's talk about {topic}. Session {i} content here."),
                },
                Turn {
                    role: "assistant".to_string(),
                    content: format!("Sure, I'd love to discuss {topic}!"),
                },
            ],
        }
    }).collect()
}

/// Generate evaluation questions for longevity testing.
fn generate_eval_questions(count: usize) -> Vec<BenchmarkQuestion> {
    (0..count).map(|i| {
        BenchmarkQuestion {
            id: format!("longevity_q_{i}"),
            question_type: "recall".to_string(),
            question: format!("What did the user discuss in session {i}?"),
            ground_truth: vec![format!("Session {i} content")],
            question_date: None,
            sessions: vec![],
            is_abstention: false,
            metadata: std::collections::HashMap::new(),
        }
    }).collect()
}

/// Render longevity results as a table.
pub fn render_longevity_table(result: &LongevityResult) -> String {
    use comfy_table::{Table, ContentArrangement, Cell, Attribute};

    let mut output = format!("RecallBench Longevity — {}\n", result.system_name);
    output.push_str(&"═".repeat(60));
    output.push('\n');

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Sessions").add_attribute(Attribute::Bold),
        Cell::new("Memories").add_attribute(Attribute::Bold),
        Cell::new("Accuracy").add_attribute(Attribute::Bold),
        Cell::new("Retrieval (ms)").add_attribute(Attribute::Bold),
        Cell::new("Ingest (ms)").add_attribute(Attribute::Bold),
    ]);

    for cp in &result.checkpoints {
        table.add_row(vec![
            Cell::new(cp.sessions_ingested),
            Cell::new(cp.estimated_memories),
            Cell::new(format!("{:.1}%", cp.accuracy * 100.0)),
            Cell::new(format!("{:.1}", cp.avg_retrieval_latency_ms)),
            Cell::new(format!("{:.1}", cp.avg_ingest_latency_ms)),
        ]);
    }

    output.push_str(&table.to_string());
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_sessions_correct_count() {
        let sessions = generate_sessions(100);
        assert_eq!(sessions.len(), 100);
        assert!(sessions[0].turns.len() > 0);
    }

    #[test]
    fn generate_questions_correct_count() {
        let questions = generate_eval_questions(50);
        assert_eq!(questions.len(), 50);
    }

    #[test]
    fn render_table_works() {
        let result = LongevityResult {
            system_name: "test".to_string(),
            checkpoints: vec![
                CheckpointResult {
                    sessions_ingested: 100,
                    estimated_memories: 200,
                    accuracy: 0.92,
                    correct: 46,
                    total: 50,
                    avg_retrieval_latency_ms: 5.2,
                    avg_ingest_latency_ms: 1.3,
                },
            ],
        };
        let output = render_longevity_table(&result);
        assert!(output.contains("test"));
        assert!(output.contains("92.0%"));
    }
}
