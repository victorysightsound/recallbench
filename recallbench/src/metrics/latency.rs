use hdrhistogram::Histogram;

use crate::types::EvalResult;

/// Latency percentile stats for a single operation.
#[derive(Debug, Clone)]
pub struct LatencyStats {
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub mean: f64,
    pub count: u64,
}

/// Latency metrics broken down by pipeline stage.
#[derive(Debug, Clone)]
pub struct LatencyMetrics {
    pub ingest: LatencyStats,
    pub retrieval: LatencyStats,
    pub generation: LatencyStats,
    pub judge: LatencyStats,
    pub total: LatencyStats,
}

/// Compute latency metrics from evaluation results.
pub fn compute_latency(results: &[EvalResult]) -> LatencyMetrics {
    let mut ingest_hist = Histogram::<u64>::new(3).unwrap();
    let mut retrieval_hist = Histogram::<u64>::new(3).unwrap();
    let mut gen_hist = Histogram::<u64>::new(3).unwrap();
    let mut judge_hist = Histogram::<u64>::new(3).unwrap();
    let mut total_hist = Histogram::<u64>::new(3).unwrap();

    for r in results {
        let _ = ingest_hist.record(r.ingest_latency_ms);
        let _ = retrieval_hist.record(r.retrieval_latency_ms);
        let _ = gen_hist.record(r.generation_latency_ms);
        let _ = judge_hist.record(r.judge_latency_ms);
        let total = r.ingest_latency_ms + r.retrieval_latency_ms
            + r.generation_latency_ms + r.judge_latency_ms;
        let _ = total_hist.record(total);
    }

    LatencyMetrics {
        ingest: stats_from_hist(&ingest_hist),
        retrieval: stats_from_hist(&retrieval_hist),
        generation: stats_from_hist(&gen_hist),
        judge: stats_from_hist(&judge_hist),
        total: stats_from_hist(&total_hist),
    }
}

fn stats_from_hist(hist: &Histogram<u64>) -> LatencyStats {
    if hist.is_empty() {
        return LatencyStats { p50: 0.0, p95: 0.0, p99: 0.0, mean: 0.0, count: 0 };
    }
    LatencyStats {
        p50: hist.value_at_quantile(0.50) as f64,
        p95: hist.value_at_quantile(0.95) as f64,
        p99: hist.value_at_quantile(0.99) as f64,
        mean: hist.mean(),
        count: hist.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(ingest: u64, retrieval: u64, generation: u64, judge: u64) -> EvalResult {
        EvalResult {
            question_id: "q".to_string(),
            system_name: "test".to_string(),
            question_type: "default".to_string(),
            is_abstention: false,
            hypothesis: "h".to_string(),
            ground_truth: "g".to_string(),
            is_correct: true,
            ingest_latency_ms: ingest,
            retrieval_latency_ms: retrieval,
            generation_latency_ms: generation,
            judge_latency_ms: judge,
            tokens_used: 0,
            tokens_generated: 0,
            timestamp: None,
        }
    }

    #[test]
    fn empty_results() {
        let metrics = compute_latency(&[]);
        assert_eq!(metrics.ingest.count, 0);
        assert_eq!(metrics.total.p50, 0.0);
    }

    #[test]
    fn single_result() {
        let results = vec![make_result(10, 20, 2000, 500)];
        let metrics = compute_latency(&results);
        assert_eq!(metrics.ingest.p50, 10.0);
        assert_eq!(metrics.retrieval.p50, 20.0);
        assert_eq!(metrics.generation.p50, 2000.0);
        assert_eq!(metrics.judge.p50, 500.0);
        assert!((metrics.total.p50 - 2530.0).abs() < 5.0); // HDR histogram rounding
    }

    #[test]
    fn multiple_results() {
        let results: Vec<_> = (1..=100).map(|i| make_result(i, i * 2, i * 10, i * 5)).collect();
        let metrics = compute_latency(&results);
        assert!(metrics.ingest.p50 > 0.0);
        assert!(metrics.ingest.p95 > metrics.ingest.p50);
        assert!(metrics.ingest.p99 >= metrics.ingest.p95);
        assert_eq!(metrics.ingest.count, 100);
    }
}
