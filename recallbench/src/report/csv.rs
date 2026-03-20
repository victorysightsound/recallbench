use std::io::Write;

use anyhow::Result;

use crate::types::EvalResult;

/// Write evaluation results as CSV.
pub fn write_csv<W: Write>(writer: W, results: &[EvalResult]) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(writer);

    wtr.write_record([
        "question_id",
        "system",
        "question_type",
        "is_abstention",
        "hypothesis",
        "ground_truth",
        "correct",
        "ingest_ms",
        "retrieval_ms",
        "generation_ms",
        "judge_ms",
        "tokens_used",
        "tokens_generated",
    ])?;

    for r in results {
        wtr.write_record([
            &r.question_id,
            &r.system_name,
            &r.question_type,
            &r.is_abstention.to_string(),
            &r.hypothesis,
            &r.ground_truth,
            &r.is_correct.to_string(),
            &r.ingest_latency_ms.to_string(),
            &r.retrieval_latency_ms.to_string(),
            &r.generation_latency_ms.to_string(),
            &r.judge_latency_ms.to_string(),
            &r.tokens_used.to_string(),
            &r.tokens_generated.to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_output() {
        let results = vec![EvalResult {
            question_id: "q001".to_string(),
            system_name: "test".to_string(),
            question_type: "default".to_string(),
            is_abstention: false,
            hypothesis: "answer".to_string(),
            ground_truth: "answer".to_string(),
            is_correct: true,
            ingest_latency_ms: 10,
            retrieval_latency_ms: 20,
            generation_latency_ms: 2000,
            judge_latency_ms: 500,
            tokens_used: 1024,
            tokens_generated: 42,
            timestamp: None,
        }];

        let mut buf = Vec::new();
        write_csv(&mut buf, &results).unwrap();
        let csv_str = String::from_utf8(buf).unwrap();
        assert!(csv_str.contains("question_id"));
        assert!(csv_str.contains("q001"));
        assert!(csv_str.contains("true"));
    }
}
