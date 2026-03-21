use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Pipeline stages for checkpointing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    Ingest,
    Retrieve,
    Generate,
    Judge,
    Complete,
}

/// Checkpoint data saved after each pipeline stage.
#[derive(Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    pub question_id: String,
    pub stage: Stage,
    pub ingest_ms: Option<u64>,
    pub retrieval_context: Option<String>,
    pub retrieval_ms: Option<u64>,
    pub hypothesis: Option<String>,
    pub generation_ms: Option<u64>,
    pub is_correct: Option<bool>,
    pub judge_ms: Option<u64>,
}

/// Get the checkpoint directory for a results file.
pub fn checkpoint_dir(results_path: &Path) -> PathBuf {
    let parent = results_path.parent().unwrap_or(Path::new("."));
    parent.join(".checkpoints")
}

/// Save a checkpoint for a question.
pub fn save_checkpoint(results_path: &Path, checkpoint: &Checkpoint) -> Result<()> {
    let dir = checkpoint_dir(results_path);
    std::fs::create_dir_all(&dir)?;

    let path = dir.join(format!("{}.json", checkpoint.question_id));
    let json = serde_json::to_string_pretty(checkpoint)?;
    std::fs::write(&path, json).context("Failed to write checkpoint")?;
    Ok(())
}

/// Load a checkpoint for a question, if one exists.
pub fn load_checkpoint(results_path: &Path, question_id: &str) -> Result<Option<Checkpoint>> {
    let path = checkpoint_dir(results_path).join(format!("{question_id}.json"));
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let checkpoint: Checkpoint = serde_json::from_str(&content)?;
    Ok(Some(checkpoint))
}

/// Remove a checkpoint after question is fully evaluated.
pub fn remove_checkpoint(results_path: &Path, question_id: &str) -> Result<()> {
    let path = checkpoint_dir(results_path).join(format!("{question_id}.json"));
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let results_path = dir.path().join("results.jsonl");

        let cp = Checkpoint {
            question_id: "q001".to_string(),
            stage: Stage::Retrieve,
            ingest_ms: Some(12),
            retrieval_context: Some("context here".to_string()),
            retrieval_ms: Some(8),
            hypothesis: None,
            generation_ms: None,
            is_correct: None,
            judge_ms: None,
        };

        save_checkpoint(&results_path, &cp).unwrap();
        let loaded = load_checkpoint(&results_path, "q001").unwrap().unwrap();
        assert_eq!(loaded.question_id, "q001");
        assert_eq!(loaded.stage, Stage::Retrieve);
        assert_eq!(loaded.retrieval_context, Some("context here".to_string()));

        remove_checkpoint(&results_path, "q001").unwrap();
        assert!(load_checkpoint(&results_path, "q001").unwrap().is_none());
    }

    #[test]
    fn load_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let results_path = dir.path().join("results.jsonl");
        assert!(load_checkpoint(&results_path, "nonexistent").unwrap().is_none());
    }
}
