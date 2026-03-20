use crate::types::EvalResult;

/// Token usage and estimated cost metrics.
#[derive(Debug, Clone)]
pub struct CostMetrics {
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub estimated_cost_usd: f64,
}

/// Per-provider pricing (USD per 1M tokens).
#[derive(Debug, Clone)]
pub struct Pricing {
    pub input_per_million: f64,
    pub output_per_million: f64,
}

impl Default for Pricing {
    fn default() -> Self {
        // Default to Claude Sonnet pricing
        Self {
            input_per_million: 3.0,
            output_per_million: 15.0,
        }
    }
}

/// Compute cost metrics from evaluation results.
pub fn compute_cost(results: &[EvalResult], pricing: &Pricing) -> CostMetrics {
    let total_in: u64 = results.iter().map(|r| r.tokens_used as u64).sum();
    let total_out: u64 = results.iter().map(|r| r.tokens_generated as u64).sum();

    let cost = (total_in as f64 / 1_000_000.0) * pricing.input_per_million
        + (total_out as f64 / 1_000_000.0) * pricing.output_per_million;

    CostMetrics {
        total_tokens_in: total_in,
        total_tokens_out: total_out,
        estimated_cost_usd: cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(tokens_in: u32, tokens_out: u32) -> EvalResult {
        EvalResult {
            question_id: "q".to_string(),
            system_name: "test".to_string(),
            question_type: "default".to_string(),
            is_abstention: false,
            hypothesis: "h".to_string(),
            ground_truth: "g".to_string(),
            is_correct: true,
            ingest_latency_ms: 0,
            retrieval_latency_ms: 0,
            generation_latency_ms: 0,
            judge_latency_ms: 0,
            tokens_used: tokens_in,
            tokens_generated: tokens_out,
            timestamp: None,
        }
    }

    #[test]
    fn empty_results() {
        let cost = compute_cost(&[], &Pricing::default());
        assert_eq!(cost.total_tokens_in, 0);
        assert_eq!(cost.estimated_cost_usd, 0.0);
    }

    #[test]
    fn compute_costs() {
        let results = vec![
            make_result(1000, 50),
            make_result(2000, 100),
        ];
        let cost = compute_cost(&results, &Pricing::default());
        assert_eq!(cost.total_tokens_in, 3000);
        assert_eq!(cost.total_tokens_out, 150);
        assert!(cost.estimated_cost_usd > 0.0);
    }

    #[test]
    fn one_million_tokens() {
        let results = vec![make_result(1_000_000, 0)];
        let pricing = Pricing { input_per_million: 3.0, output_per_million: 15.0 };
        let cost = compute_cost(&results, &pricing);
        assert!((cost.estimated_cost_usd - 3.0).abs() < 0.001);
    }
}
