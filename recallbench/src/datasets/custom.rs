use anyhow::{Context, Result};

use crate::traits::BenchmarkDataset;
use crate::types::BenchmarkQuestion;

/// A user-defined dataset loaded from a JSON file.
pub struct CustomDataset {
    dataset_name: String,
    questions: Vec<BenchmarkQuestion>,
}

impl CustomDataset {
    /// Load a custom dataset from a JSON file.
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read custom dataset file")?;
        Self::from_json(
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("custom"),
            &content,
        )
    }

    /// Parse from a JSON string.
    pub fn from_json(name: &str, json: &str) -> Result<Self> {
        let questions: Vec<BenchmarkQuestion> = serde_json::from_str(json)
            .context("Failed to parse custom dataset JSON")?;
        Ok(Self {
            dataset_name: name.to_string(),
            questions,
        })
    }

    /// Validate the dataset structure. Returns a list of errors (empty = valid).
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.questions.is_empty() {
            errors.push("Dataset has no questions".to_string());
        }

        for (i, q) in self.questions.iter().enumerate() {
            if q.id.is_empty() {
                errors.push(format!("Question {i}: missing id"));
            }
            if q.question_type.is_empty() {
                errors.push(format!("Question {} (index {i}): missing question_type", q.id));
            }
            if q.question.is_empty() {
                errors.push(format!("Question {}: missing question text", q.id));
            }
            if q.ground_truth.is_empty() {
                errors.push(format!("Question {}: missing ground_truth", q.id));
            }
        }

        errors
    }
}

impl BenchmarkDataset for CustomDataset {
    fn name(&self) -> &str {
        &self.dataset_name
    }

    fn variant(&self) -> &str {
        "default"
    }

    fn description(&self) -> &str {
        "User-defined custom dataset"
    }

    fn questions(&self) -> &[BenchmarkQuestion] {
        &self.questions
    }

    fn question_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.questions.iter()
            .map(|q| q.question_type.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        types.sort();
        types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_JSON: &str = r#"[
        {
            "id": "q001",
            "question_type": "factual",
            "question": "What color is the sky?",
            "ground_truth": ["blue"],
            "sessions": []
        }
    ]"#;

    #[test]
    fn parse_valid_custom() {
        let ds = CustomDataset::from_json("test", VALID_JSON).unwrap();
        assert_eq!(ds.questions().len(), 1);
        assert_eq!(ds.name(), "test");
        assert!(ds.validate().is_empty());
    }

    #[test]
    fn validate_missing_fields() {
        let json = r#"[{"id":"","question_type":"","question":"","ground_truth":[],"sessions":[]}]"#;
        let ds = CustomDataset::from_json("test", json).unwrap();
        let errors = ds.validate();
        assert!(!errors.is_empty());
    }

    #[test]
    fn validate_empty_dataset() {
        let ds = CustomDataset::from_json("test", "[]").unwrap();
        let errors = ds.validate();
        assert!(errors.iter().any(|e| e.contains("no questions")));
    }
}
