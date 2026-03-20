use std::collections::HashMap;

use serde::Serialize;

use crate::metrics::{AccuracyMetrics, LatencyMetrics, CostMetrics};

/// JSON summary report structure.
#[derive(Serialize)]
pub struct JsonReport {
    pub recallbench_version: String,
    pub benchmark: String,
    pub variant: String,
    pub date: String,
    pub systems: Vec<SystemReport>,
}

#[derive(Serialize)]
pub struct SystemReport {
    pub name: String,
    pub task_averaged: f64,
    pub overall: f64,
    pub per_type: HashMap<String, f64>,
    pub abstention: Option<f64>,
    pub total_questions: usize,
    pub total_correct: usize,
    pub latency_ms: Option<LatencyReport>,
    pub cost: Option<CostReport>,
}

#[derive(Serialize)]
pub struct LatencyReport {
    pub ingest_p50: f64,
    pub retrieval_p50: f64,
    pub generation_p50: f64,
    pub judge_p50: f64,
    pub total_p50: f64,
}

#[derive(Serialize)]
pub struct CostReport {
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub estimated_usd: f64,
}

/// Build a JSON report from metrics.
pub fn build_json_report(
    dataset: &str,
    variant: &str,
    systems: &[(&str, &AccuracyMetrics, Option<&LatencyMetrics>, Option<&CostMetrics>)],
) -> JsonReport {
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let system_reports = systems.iter().map(|(name, acc, lat, cost)| {
        SystemReport {
            name: name.to_string(),
            task_averaged: acc.task_averaged,
            overall: acc.overall,
            per_type: acc.per_type.clone(),
            abstention: acc.abstention,
            total_questions: acc.total_questions,
            total_correct: acc.total_correct,
            latency_ms: lat.map(|l| LatencyReport {
                ingest_p50: l.ingest.p50,
                retrieval_p50: l.retrieval.p50,
                generation_p50: l.generation.p50,
                judge_p50: l.judge.p50,
                total_p50: l.total.p50,
            }),
            cost: cost.map(|c| CostReport {
                tokens_in: c.total_tokens_in,
                tokens_out: c.total_tokens_out,
                estimated_usd: c.estimated_cost_usd,
            }),
        }
    }).collect();

    JsonReport {
        recallbench_version: env!("CARGO_PKG_VERSION").to_string(),
        benchmark: dataset.to_string(),
        variant: variant.to_string(),
        date,
        systems: system_reports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_report() {
        let acc = AccuracyMetrics {
            per_type: HashMap::new(),
            task_averaged: 0.9,
            overall: 0.88,
            abstention: None,
            total_questions: 100,
            total_correct: 88,
        };
        let report = build_json_report("longmemeval", "oracle", &[("test", &acc, None, None)]);
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("longmemeval"));
        assert!(json.contains("test"));
    }
}
