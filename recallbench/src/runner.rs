use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::Semaphore;

use crate::judge;
use crate::resume;
use crate::traits::{BenchmarkDataset, LLMClient, MemorySystem};
use crate::types::{BenchmarkQuestion, EvalResult};

/// Configuration for a benchmark run.
pub struct RunConfig {
    pub concurrency: usize,
    pub token_budget: usize,
    pub output_path: std::path::PathBuf,
    pub filter_types: Option<Vec<String>>,
    pub resume: bool,
}

/// Run a benchmark: evaluate all questions against a memory system.
pub async fn run_benchmark(
    system: &dyn MemorySystem,
    dataset: &dyn BenchmarkDataset,
    gen_llm: Arc<dyn LLMClient>,
    judge_llm: Arc<dyn LLMClient>,
    config: &RunConfig,
) -> Result<Vec<EvalResult>> {
    // Load completed IDs for resume
    let completed_ids = if config.resume {
        resume::load_completed_ids(&config.output_path)?
    } else {
        std::collections::HashSet::new()
    };

    // Filter questions
    let questions: Vec<&BenchmarkQuestion> = dataset.questions().iter()
        .filter(|q| {
            if completed_ids.contains(&q.id) {
                return false;
            }
            if let Some(ref types) = config.filter_types {
                return types.iter().any(|t| t == &q.question_type);
            }
            true
        })
        .collect();

    if questions.is_empty() {
        tracing::info!("No questions to evaluate (all completed or filtered).");
        if config.resume {
            return resume::load_results(&config.output_path);
        }
        return Ok(Vec::new());
    }

    tracing::info!(
        "Evaluating {} questions against {} ({})",
        questions.len(),
        system.name(),
        system.version(),
    );

    let pb = ProgressBar::new(questions.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{wide_bar:.green/dim} {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message(format!("Running {} on {}", dataset.name(), system.name()));

    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    let results = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let output_path = config.output_path.clone();
    let correct_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let total_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Process questions — currently sequential per question since MemorySystem
    // requires reset() between questions for isolation.
    for question in &questions {
        let _permit = semaphore.acquire().await?;

        let result = evaluate_question(
            system,
            question,
            gen_llm.as_ref(),
            judge_llm.as_ref(),
            config.token_budget,
        ).await;

        match result {
            Ok(eval_result) => {
                if eval_result.is_correct {
                    correct_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                total_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                // Append to JSONL
                if let Err(e) = resume::append_result(&output_path, &eval_result) {
                    tracing::error!("Failed to write result: {e}");
                }

                results.lock().await.push(eval_result);
            }
            Err(e) => {
                tracing::error!("Error evaluating question {}: {e}", question.id);
                total_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }

        let total = total_count.load(std::sync::atomic::Ordering::Relaxed);
        let correct = correct_count.load(std::sync::atomic::Ordering::Relaxed);
        let accuracy = if total > 0 { correct as f64 / total as f64 * 100.0 } else { 0.0 };
        pb.set_message(format!(
            "Running {} on {} — {:.1}% ({}/{})",
            dataset.name(), system.name(), accuracy, correct, total,
        ));
        pb.inc(1);
    }

    pb.finish_with_message("Evaluation complete");

    let final_results = results.lock().await.clone();

    // If resuming, merge with previously completed results
    if config.resume && !completed_ids.is_empty() {
        let mut all_results = resume::load_results(&config.output_path)?;
        // Results already appended to file, so load_results has everything
        return Ok(all_results);
    }

    Ok(final_results)
}

/// Evaluate a single question against a memory system.
async fn evaluate_question(
    system: &dyn MemorySystem,
    question: &BenchmarkQuestion,
    gen_llm: &dyn LLMClient,
    judge_llm: &dyn LLMClient,
    token_budget: usize,
) -> Result<EvalResult> {
    // 1. Reset
    system.reset().await?;

    // 2. Ingest
    let ingest_start = Instant::now();
    for session in &question.sessions {
        system.ingest_session(session).await?;
    }
    let ingest_ms = ingest_start.elapsed().as_millis() as u64;

    // 3. Retrieve
    let retrieval_start = Instant::now();
    let retrieval = system.retrieve_context(
        &question.question,
        question.question_date.as_deref(),
        token_budget,
    ).await?;
    let retrieval_ms = retrieval_start.elapsed().as_millis() as u64;

    // 4. Generate
    let gen_start = Instant::now();
    let prompt = build_generation_prompt(
        &retrieval.context,
        &question.question,
        question.question_date.as_deref(),
    );
    let hypothesis = gen_llm.generate(&prompt, 256).await?;
    let generation_ms = gen_start.elapsed().as_millis() as u64;

    // 5. Judge
    let judge_start = Instant::now();
    let ground_truth = question.ground_truth.join(", ");
    let is_correct = judge::judge_answer(
        &question.question_type,
        &question.question,
        &ground_truth,
        &hypothesis,
        question.is_abstention,
        judge_llm,
    ).await?;
    let judge_ms = judge_start.elapsed().as_millis() as u64;

    Ok(EvalResult {
        question_id: question.id.clone(),
        system_name: system.name().to_string(),
        question_type: question.question_type.clone(),
        is_abstention: question.is_abstention,
        hypothesis,
        ground_truth,
        is_correct,
        ingest_latency_ms: ingest_ms,
        retrieval_latency_ms: retrieval_ms,
        generation_latency_ms: generation_ms,
        judge_latency_ms: judge_ms,
        tokens_used: retrieval.tokens_used as u32,
        tokens_generated: 0, // TODO: track from LLM response
        timestamp: Some(chrono::Utc::now()),
    })
}

/// Build the generation prompt from context, question, and date.
pub fn build_generation_prompt(context: &str, question: &str, date: Option<&str>) -> String {
    let date_line = date.map(|d| format!("\nCurrent date: {d}")).unwrap_or_default();

    format!(
        r#"You are a helpful assistant with access to conversation history. Use the following context to answer the question accurately and concisely.

Context from memory:
{context}
{date_line}
Question: {question}

Answer the question based on the context above. If the information is not available in the context, say "I don't have enough information to answer this question.""#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_prompt_with_date() {
        let prompt = build_generation_prompt("some context", "What happened?", Some("2024/03/10"));
        assert!(prompt.contains("some context"));
        assert!(prompt.contains("What happened?"));
        assert!(prompt.contains("2024/03/10"));
    }

    #[test]
    fn generation_prompt_without_date() {
        let prompt = build_generation_prompt("ctx", "Q?", None);
        assert!(prompt.contains("ctx"));
        assert!(prompt.contains("Q?"));
        assert!(!prompt.contains("Current date"));
    }

    #[tokio::test]
    async fn evaluate_with_echo_and_mock() {
        use crate::systems::echo::EchoSystem;
        use crate::types::{ConversationSession, Turn};

        struct MockLLM;

        #[async_trait::async_trait]
        impl LLMClient for MockLLM {
            fn name(&self) -> &str { "mock" }
            async fn generate(&self, _: &str, _: usize) -> Result<String> {
                Ok("yes".to_string())
            }
        }

        let system = EchoSystem::new();
        let question = BenchmarkQuestion {
            id: "q001".to_string(),
            question_type: "default".to_string(),
            question: "What is the answer?".to_string(),
            ground_truth: vec!["yes".to_string()],
            question_date: None,
            sessions: vec![ConversationSession {
                id: "s1".to_string(),
                date: None,
                turns: vec![Turn { role: "user".to_string(), content: "test".to_string() }],
            }],
            is_abstention: false,
            metadata: std::collections::HashMap::new(),
        };

        let mock = MockLLM;
        let result = evaluate_question(&system, &question, &mock, &mock, 16384).await.unwrap();
        assert_eq!(result.question_id, "q001");
        assert!(result.is_correct);
        assert!(result.ingest_latency_ms < 1000);
    }
}
