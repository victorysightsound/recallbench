use std::path::PathBuf;
use std::sync::Arc;

use axum::{Router, Json};
use axum::extract::{Path, State};
use axum::routing::get;
use serde::Serialize;

use crate::metrics;
use crate::report;
use crate::resume;

#[derive(Clone)]
struct AppState {
    results_dir: PathBuf,
}

/// Create API routes for the web UI.
pub fn api_routes(results_dir: PathBuf) -> Router {
    let state = Arc::new(AppState { results_dir });

    Router::new()
        .route("/api/runs", get(list_runs))
        .route("/api/runs/{id}", get(get_run))
        .route("/api/runs/{id}/metrics", get(get_run_metrics))
        .route("/api/runs/{id}/questions", get(get_run_questions))
        .route("/api/runs/{id}/failures", get(get_run_failures))
        .with_state(state)
}

#[derive(Serialize)]
struct RunSummary {
    id: String,
    filename: String,
    system: Option<String>,
    total_questions: usize,
    accuracy: f64,
}

async fn list_runs(State(state): State<Arc<AppState>>) -> Json<Vec<RunSummary>> {
    let mut runs = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&state.results_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "jsonl") {
                if let Ok(results) = resume::load_results(&path) {
                    if !results.is_empty() {
                        let acc = metrics::compute_accuracy(&results);
                        runs.push(RunSummary {
                            id: path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
                            filename: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                            system: Some(results[0].system_name.clone()),
                            total_questions: results.len(),
                            accuracy: acc.overall,
                        });
                    }
                }
            }
        }
    }

    Json(runs)
}

async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let path = state.results_dir.join(format!("{id}.jsonl"));
    match resume::load_results(&path) {
        Ok(results) => Json(serde_json::to_value(&results).unwrap_or_default()),
        Err(_) => Json(serde_json::json!({"error": "Run not found"})),
    }
}

async fn get_run_metrics(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let path = state.results_dir.join(format!("{id}.jsonl"));
    match resume::load_results(&path) {
        Ok(results) => {
            let acc = metrics::compute_accuracy(&results);
            let lat = metrics::compute_latency(&results);
            let cost = metrics::compute_cost(&results, &metrics::Pricing::default());
            Json(serde_json::json!({
                "accuracy": {
                    "task_averaged": acc.task_averaged,
                    "overall": acc.overall,
                    "per_type": acc.per_type,
                    "abstention": acc.abstention,
                    "total_questions": acc.total_questions,
                    "total_correct": acc.total_correct,
                },
                "latency": {
                    "ingest_p50": lat.ingest.p50,
                    "retrieval_p50": lat.retrieval.p50,
                    "generation_p50": lat.generation.p50,
                    "total_p50": lat.total.p50,
                },
                "cost": {
                    "tokens_in": cost.total_tokens_in,
                    "tokens_out": cost.total_tokens_out,
                    "estimated_usd": cost.estimated_cost_usd,
                },
            }))
        }
        Err(_) => Json(serde_json::json!({"error": "Run not found"})),
    }
}

async fn get_run_questions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let path = state.results_dir.join(format!("{id}.jsonl"));
    match resume::load_results(&path) {
        Ok(results) => Json(serde_json::to_value(&results).unwrap_or_default()),
        Err(_) => Json(serde_json::json!({"error": "Run not found"})),
    }
}

async fn get_run_failures(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let path = state.results_dir.join(format!("{id}.jsonl"));
    match resume::load_results(&path) {
        Ok(results) => {
            let analysis = report::failure::analyze_failures(&results);
            Json(serde_json::to_value(&analysis).unwrap_or_default())
        }
        Err(_) => Json(serde_json::json!({"error": "Run not found"})),
    }
}
