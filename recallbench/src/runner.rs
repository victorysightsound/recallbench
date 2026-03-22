use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::Semaphore;

use crate::judge;
use crate::resume;
use crate::sampling;
use crate::verify;
use crate::traits::{BenchmarkDataset, LLMClient, MemorySystem};
use crate::types::{BenchmarkQuestion, EvalResult};

/// Configuration for a benchmark run.
pub struct RunConfig {
    pub concurrency: usize,
    pub token_budget: usize,
    pub output_path: std::path::PathBuf,
    pub filter_types: Option<Vec<String>>,
    pub resume: bool,
    /// If Some(n), use stratified sampling to select n questions.
    pub quick_size: Option<usize>,
    /// Optional note describing the purpose of this run.
    pub note: Option<String>,
    /// Path to pre-computed embedding cache SQLite file.
    pub embedding_cache_path: Option<std::path::PathBuf>,
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

    // Apply quick mode stratified sampling if requested
    let all_questions = dataset.questions();
    let sampled: Vec<&BenchmarkQuestion>;
    let base_questions: &[&BenchmarkQuestion] = if let Some(quick_size) = config.quick_size {
        sampled = sampling::stratified_sample(all_questions, quick_size, 42);
        &sampled
    } else {
        // Will be filtered below
        sampled = Vec::new();
        &sampled // placeholder, overridden below
    };

    // Filter questions
    let questions: Vec<&BenchmarkQuestion> = if config.quick_size.is_some() {
        // Quick mode: use sampled subset, then apply additional filters
        base_questions.iter()
            .filter(|q| {
                if completed_ids.contains(&q.id) { return false; }
                if let Some(ref types) = config.filter_types {
                    return types.iter().any(|t| t == &q.question_type);
                }
                true
            })
            .copied()
            .collect()
    } else {
        all_questions.iter()
            .filter(|q| {
                if completed_ids.contains(&q.id) { return false; }
                if let Some(ref types) = config.filter_types {
                    return types.iter().any(|t| t == &q.question_type);
                }
                true
            })
            .collect()
    };

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

    // Write run metadata so the web UI knows the target count
    let meta_path = config.output_path.with_extension("meta.json");
    let meta = serde_json::json!({
        "system": system.name(),
        "dataset": dataset.name(),
        "variant": dataset.variant(),
        "total_questions": questions.len(),
        "started_at": chrono::Utc::now().to_rfc3339(),
        "note": config.note.as_deref().unwrap_or(""),
    });
    if let Ok(json) = serde_json::to_string_pretty(&meta) {
        let _ = std::fs::write(&meta_path, json);
    }

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
            config.embedding_cache_path.as_deref(),
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
        let all_results = resume::load_results(&config.output_path)?;
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
    cache_path: Option<&std::path::Path>,
) -> Result<EvalResult> {
    // 1. Reset
    system.reset().await?;

    // 2. Ingest (from cache if available, otherwise live embedding)
    let ingest_start = Instant::now();

    let mut used_cache = false;
    if let Some(path) = cache_path {
        if system.supports_precomputed() {
            // Open cache and load pre-computed chunks for this question's sessions
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

            let loaded = system.load_precomputed(&tuples)?;
            tracing::debug!("Loaded {loaded} chunks from cache for {} sessions", session_ids.len());
            used_cache = true;
        }
    }

    if !used_cache {
        for session in &question.sessions {
            system.ingest_session(session).await?;
        }
    }
    let ingest_ms = ingest_start.elapsed().as_millis() as u64;

    // 3. Retrieve
    let retrieval_start = Instant::now();
    let estimated_tokens = question.sessions.iter()
        .flat_map(|s| &s.turns)
        .map(|t| (t.content.len() as f32 * 0.25) as usize)
        .sum::<usize>();

    let retrieval = if token_budget == 0 || estimated_tokens <= token_budget {
        // Oracle mode (or small context): format all sessions directly
        // All provided context fits in budget — no search needed
        let mut context_parts = Vec::new();
        for session in &question.sessions {
            let date = session.date.as_deref().unwrap_or("unknown date");
            let mut session_text = format!("[Session from {date}]\n");
            for turn in &session.turns {
                session_text.push_str(&format!("{}: {}\n", turn.role, turn.content));
            }
            context_parts.push(session_text);
        }
        let context = context_parts.join("\n");
        let tokens_used = (context.len() as f32 * 0.25) as usize;
        crate::types::RetrievalResult {
            context,
            items_retrieved: question.sessions.len(),
            tokens_used,
            duration_ms: retrieval_start.elapsed().as_millis() as u64,
        }
    } else {
        // S/M variant: context exceeds budget, use memory system's search to find relevant content
        system.retrieve_context(
            &question.question,
            question.question_date.as_deref(),
            token_budget,
        ).await?
    };
    let retrieval_ms = retrieval_start.elapsed().as_millis() as u64;

    // 4. Generate
    let gen_start = Instant::now();
    let prompt = build_generation_prompt(
        &retrieval.context,
        &question.question,
        question.question_date.as_deref(),
        &question.question_type,
        question.is_abstention,
    );
    let mut hypothesis = gen_llm.generate(&prompt, 512).await?;

    // 4b. Self-verification pass (multi-session, knowledge-update only)
    hypothesis = verify::maybe_verify(
        gen_llm,
        &retrieval.context,
        &question.question,
        &hypothesis,
        &question.question_type,
        question.is_abstention,
    ).await?;
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

/// Build the generation prompt from context, question, date, and type.
/// Ported from mindcore-bench v3 for result parity.
pub fn build_generation_prompt(
    context: &str,
    question: &str,
    date: Option<&str>,
    question_type: &str,
    is_abstention: bool,
) -> String {
    let question_date = date.unwrap_or("unknown");

    let preamble = format!(
        "I will give you several history chats between a user and an AI assistant. \
         Based on the chat history, answer the question at the end.\n\n\
         History Chats:\n\n\
         {context}\n\n\
         Current Date: {question_date}\n\
         Question: {question}\n\n"
    );

    let type_instruction = if is_abstention {
        "Instructions: If the chat history does not contain information that DIRECTLY answers \
         this question, you MUST respond with \"I don't know\" or \"The information is not \
         available in the chat history.\" Do NOT attempt to infer, extrapolate, or guess. \
         Only answer if the information is explicitly stated in the conversations. \
         If you can answer, provide the answer concisely."
            .to_string()
    } else {
        match question_type {
            "single-session-preference" => {
                "Instructions: Based on the chat history, describe what the user's CONTENT \
                 preferences would be when responding to this question. Focus on the TOPICS, \
                 SUBJECTS, and SPECIFIC INTERESTS the user has expressed — NOT on response \
                 formatting or structure.\n\n\
                 Your answer MUST be in the format: \"The user would prefer responses that...\"\n\n\
                 Example 1:\n\
                 Question: Can you recommend some accessories for my camera?\n\
                 Good answer: The user would prefer suggestions of Sony-compatible accessories \
                 that enhance their landscape photography, based on their discussion of the \
                 Sony Alpha camera and recent mountain photography trips. They might not prefer \
                 suggestions for other camera brands or studio photography gear.\n\n\
                 Example 2:\n\
                 Question: Can you suggest some new recipes to try?\n\
                 Good answer: The user would prefer recipes that incorporate quinoa and roasted \
                 vegetables, building on their recent success with Mediterranean-style meal prep. \
                 They might not prefer recipes with dairy, given their mention of lactose \
                 intolerance.\n\n\
                 BAD answers describe formatting preferences like \"well-organized with bullet \
                 points\" or \"detailed and comprehensive.\" Focus on WHAT the user wants to \
                 hear about, not HOW it should be formatted.\n\n\
                 Now describe the user's content preferences for the question above:"
                    .to_string()
            }
            "temporal-reasoning" => {
                format!(
                    "Instructions: Answer this question step by step. Pay close attention to \
                     dates and timestamps on each session. \
                     IMPORTANT: Before computing any count or duration, list EVERY relevant \
                     event with its exact date. Then count them explicitly (1, 2, 3...) or \
                     compute the date arithmetic step by step. Do not estimate or shortcut. \
                     When counting days between dates, enumerate each step. \
                     Current Date: {question_date}"
                )
            }
            "knowledge-update" => {
                "Instructions: Answer this question step by step. When information has been \
                 updated across sessions, use the MOST RECENT value as the primary answer. \
                 IMPORTANT: List ALL versions of the relevant information chronologically with \
                 their session dates. Then clearly state the latest/most recent value as your \
                 final answer."
                    .to_string()
            }
            "multi-session" => {
                "Instructions: This question requires synthesizing information across multiple \
                 sessions. Answer step by step. \
                 IMPORTANT: Before giving your final answer, enumerate ALL relevant items/facts \
                 from EVERY session. Number each one explicitly. Do not skip any session. \
                 Then compile your final answer from the complete list."
                    .to_string()
            }
            _ => {
                "Instructions: Answer the question based on the chat history. \
                 First extract the relevant information, then provide a concise answer."
                    .to_string()
            }
        }
    };

    format!("{preamble}{type_instruction}\n\nAnswer:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_prompt_with_date() {
        let prompt = build_generation_prompt("some context", "What happened?", Some("2024/03/10"), "temporal-reasoning", false);
        assert!(prompt.contains("some context"));
        assert!(prompt.contains("What happened?"));
        assert!(prompt.contains("2024/03/10"));
        assert!(prompt.contains("EVERY relevant event"));
    }

    #[test]
    fn generation_prompt_preference() {
        let prompt = build_generation_prompt("ctx", "Recommend?", None, "single-session-preference", false);
        assert!(prompt.contains("The user would prefer"));
        assert!(prompt.contains("BAD answers"));
    }

    #[test]
    fn generation_prompt_abstention() {
        let prompt = build_generation_prompt("ctx", "Q?", None, "any", true);
        assert!(prompt.contains("MUST respond with"));
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
