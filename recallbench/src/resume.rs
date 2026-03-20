use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};

use crate::types::EvalResult;

/// Load completed question IDs from an existing JSONL results file.
pub fn load_completed_ids(path: &Path) -> Result<HashSet<String>> {
    if !path.exists() {
        return Ok(HashSet::new());
    }

    let content = std::fs::read_to_string(path)
        .context("Failed to read results file for resume")?;

    let mut ids = HashSet::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<EvalResult>(line) {
            Ok(result) => {
                ids.insert(result.question_id);
            }
            Err(e) => {
                tracing::warn!("Skipping malformed JSONL line: {e}");
            }
        }
    }

    Ok(ids)
}

/// Append a single result to a JSONL file.
pub fn append_result(path: &Path, result: &EvalResult) -> Result<()> {
    use std::io::Write;

    let line = serde_json::to_string(result)
        .context("Failed to serialize EvalResult")?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .context("Failed to open results file for appending")?;

    writeln!(file, "{line}")?;
    Ok(())
}

/// Load all results from a JSONL file.
pub fn load_results(path: &Path) -> Result<Vec<EvalResult>> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read results file")?;

    let mut results = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<EvalResult>(line) {
            Ok(result) => results.push(result),
            Err(e) => tracing::warn!("Skipping malformed JSONL line: {e}"),
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn make_result(id: &str) -> EvalResult {
        EvalResult {
            question_id: id.to_string(),
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
            tokens_used: 0,
            tokens_generated: 0,
            timestamp: None,
        }
    }

    #[test]
    fn load_nonexistent() {
        let ids = load_completed_ids(Path::new("/nonexistent/file.jsonl")).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn append_and_load() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        append_result(path, &make_result("q001")).unwrap();
        append_result(path, &make_result("q002")).unwrap();

        let ids = load_completed_ids(path).unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains("q001"));
        assert!(ids.contains("q002"));

        let results = load_results(path).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn handles_malformed_lines() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        std::fs::write(path, "{\"question_id\":\"q1\",\"system_name\":\"t\",\"question_type\":\"d\",\"hypothesis\":\"h\",\"ground_truth\":\"g\",\"is_correct\":true,\"ingest_latency_ms\":0,\"retrieval_latency_ms\":0,\"generation_latency_ms\":0,\"judge_latency_ms\":0,\"tokens_used\":0,\"tokens_generated\":0}\nnot json\n").unwrap();

        let ids = load_completed_ids(path).unwrap();
        assert_eq!(ids.len(), 1);
    }
}
