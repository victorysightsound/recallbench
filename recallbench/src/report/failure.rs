use std::collections::HashMap;

use serde::Serialize;

use crate::types::EvalResult;

/// A failed question with context for debugging.
#[derive(Debug, Clone, Serialize)]
pub struct FailureEntry {
    pub question_id: String,
    pub question_type: String,
    pub is_abstention: bool,
    pub hypothesis: String,
    pub ground_truth: String,
}

/// Failure analysis grouped by question type.
#[derive(Debug, Serialize)]
pub struct FailureAnalysis {
    pub total_failures: usize,
    pub total_questions: usize,
    pub by_type: HashMap<String, Vec<FailureEntry>>,
    pub failures: Vec<FailureEntry>,
}

/// Extract and analyze failures from evaluation results.
pub fn analyze_failures(results: &[EvalResult]) -> FailureAnalysis {
    let mut by_type: HashMap<String, Vec<FailureEntry>> = HashMap::new();
    let mut failures = Vec::new();

    for r in results {
        if !r.is_correct {
            let entry = FailureEntry {
                question_id: r.question_id.clone(),
                question_type: r.question_type.clone(),
                is_abstention: r.is_abstention,
                hypothesis: r.hypothesis.clone(),
                ground_truth: r.ground_truth.clone(),
            };
            by_type.entry(r.question_type.clone()).or_default().push(entry.clone());
            failures.push(entry);
        }
    }

    FailureAnalysis {
        total_failures: failures.len(),
        total_questions: results.len(),
        by_type,
        failures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(id: &str, qtype: &str, correct: bool) -> EvalResult {
        EvalResult {
            question_id: id.to_string(),
            system_name: "test".to_string(),
            question_type: qtype.to_string(),
            is_abstention: false,
            hypothesis: "h".to_string(),
            ground_truth: "g".to_string(),
            is_correct: correct,
            ingest_latency_ms: 0,
            retrieval_latency_ms: 0,
            generation_latency_ms: 0,
            judge_latency_ms: 0,
            tokens_used: 0,
            tokens_generated: 0,
            timestamp: None,
        }
    }

    #[test]
    fn no_failures() {
        let results = vec![make_result("q1", "a", true)];
        let analysis = analyze_failures(&results);
        assert_eq!(analysis.total_failures, 0);
        assert!(analysis.failures.is_empty());
    }

    #[test]
    fn mixed_failures() {
        let results = vec![
            make_result("q1", "temporal", true),
            make_result("q2", "temporal", false),
            make_result("q3", "multi", false),
            make_result("q4", "multi", false),
        ];
        let analysis = analyze_failures(&results);
        assert_eq!(analysis.total_failures, 3);
        assert_eq!(analysis.total_questions, 4);
        assert_eq!(analysis.by_type["temporal"].len(), 1);
        assert_eq!(analysis.by_type["multi"].len(), 2);
    }
}
