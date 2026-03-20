use std::collections::HashMap;

use crate::types::EvalResult;

/// Accuracy metrics computed from evaluation results.
#[derive(Debug, Clone)]
pub struct AccuracyMetrics {
    /// Accuracy per question type.
    pub per_type: HashMap<String, f64>,
    /// Task-averaged accuracy (mean of per-type accuracies).
    pub task_averaged: f64,
    /// Overall accuracy (mean of all questions).
    pub overall: f64,
    /// Abstention accuracy (separate category).
    pub abstention: Option<f64>,
    /// Total questions evaluated.
    pub total_questions: usize,
    /// Total correct answers.
    pub total_correct: usize,
}

/// Compute accuracy metrics from evaluation results.
pub fn compute_accuracy(results: &[EvalResult]) -> AccuracyMetrics {
    if results.is_empty() {
        return AccuracyMetrics {
            per_type: HashMap::new(),
            task_averaged: 0.0,
            overall: 0.0,
            abstention: None,
            total_questions: 0,
            total_correct: 0,
        };
    }

    // Per-type counts
    let mut type_correct: HashMap<String, usize> = HashMap::new();
    let mut type_total: HashMap<String, usize> = HashMap::new();

    // Abstention counts
    let mut abs_correct = 0usize;
    let mut abs_total = 0usize;

    // Overall counts
    let mut total_correct = 0usize;

    for result in results {
        if result.is_abstention {
            abs_total += 1;
            if result.is_correct {
                abs_correct += 1;
            }
        }

        *type_total.entry(result.question_type.clone()).or_insert(0) += 1;
        if result.is_correct {
            *type_correct.entry(result.question_type.clone()).or_insert(0) += 1;
            total_correct += 1;
        }
    }

    // Per-type accuracy
    let per_type: HashMap<String, f64> = type_total.iter().map(|(qtype, total)| {
        let correct = type_correct.get(qtype).copied().unwrap_or(0);
        let accuracy = if *total > 0 { correct as f64 / *total as f64 } else { 0.0 };
        (qtype.clone(), accuracy)
    }).collect();

    // Task-averaged (mean of per-type)
    let task_averaged = if per_type.is_empty() {
        0.0
    } else {
        let sum: f64 = per_type.values().sum();
        sum / per_type.len() as f64
    };

    // Overall
    let overall = total_correct as f64 / results.len() as f64;

    // Abstention
    let abstention = if abs_total > 0 {
        Some(abs_correct as f64 / abs_total as f64)
    } else {
        None
    };

    AccuracyMetrics {
        per_type,
        task_averaged,
        overall,
        abstention,
        total_questions: results.len(),
        total_correct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(qtype: &str, correct: bool, abstention: bool) -> EvalResult {
        EvalResult {
            question_id: "q".to_string(),
            system_name: "test".to_string(),
            question_type: qtype.to_string(),
            is_abstention: abstention,
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
    fn empty_results() {
        let metrics = compute_accuracy(&[]);
        assert_eq!(metrics.total_questions, 0);
        assert_eq!(metrics.overall, 0.0);
    }

    #[test]
    fn all_correct() {
        let results = vec![
            make_result("a", true, false),
            make_result("a", true, false),
            make_result("b", true, false),
        ];
        let metrics = compute_accuracy(&results);
        assert_eq!(metrics.overall, 1.0);
        assert_eq!(metrics.task_averaged, 1.0);
        assert_eq!(metrics.total_correct, 3);
    }

    #[test]
    fn mixed_results() {
        let results = vec![
            make_result("a", true, false),
            make_result("a", false, false),
            make_result("b", true, false),
            make_result("b", true, false),
        ];
        let metrics = compute_accuracy(&results);
        assert_eq!(metrics.overall, 0.75);
        assert_eq!(metrics.per_type["a"], 0.5);
        assert_eq!(metrics.per_type["b"], 1.0);
        // Task-averaged = (0.5 + 1.0) / 2 = 0.75
        assert!((metrics.task_averaged - 0.75).abs() < 0.001);
    }

    #[test]
    fn abstention_accuracy() {
        let results = vec![
            make_result("a", true, true),
            make_result("a", false, true),
            make_result("b", true, false),
        ];
        let metrics = compute_accuracy(&results);
        assert_eq!(metrics.abstention, Some(0.5));
    }

    #[test]
    fn no_abstention_questions() {
        let results = vec![make_result("a", true, false)];
        let metrics = compute_accuracy(&results);
        assert_eq!(metrics.abstention, None);
    }
}
