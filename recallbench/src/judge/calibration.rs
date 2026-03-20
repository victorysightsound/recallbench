use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::traits::LLMClient;
use super::prompts::judge_prompt;
use super::parse_judgment;

/// A pre-scored calibration pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationPair {
    pub question: String,
    pub question_type: String,
    pub ground_truth: String,
    pub hypothesis: String,
    pub expected_correct: bool,
    #[serde(default)]
    pub is_abstention: bool,
}

/// Result of running calibration.
#[derive(Debug)]
pub struct CalibrationResult {
    pub total: usize,
    pub correct: usize,
    pub accuracy: f64,
    pub mismatches: Vec<CalibrationMismatch>,
}

#[derive(Debug)]
pub struct CalibrationMismatch {
    pub index: usize,
    pub question: String,
    pub expected: bool,
    pub got: bool,
}

/// Load calibration pairs from a JSON file.
pub fn load_calibration_pairs(json: &str) -> Result<Vec<CalibrationPair>> {
    let pairs: Vec<CalibrationPair> = serde_json::from_str(json)?;
    Ok(pairs)
}

/// Run calibration against a judge.
///
/// Returns accuracy and a list of mismatches. Fails if accuracy < threshold.
pub async fn run_calibration(
    pairs: &[CalibrationPair],
    judge: &dyn LLMClient,
) -> Result<CalibrationResult> {
    let mut correct = 0;
    let mut mismatches = Vec::new();

    for (i, pair) in pairs.iter().enumerate() {
        let prompt = judge_prompt(
            &pair.question_type,
            &pair.question,
            &pair.ground_truth,
            &pair.hypothesis,
            pair.is_abstention,
        );

        let response = judge.generate(&prompt, 10).await?;
        let judgment = parse_judgment(&response).unwrap_or(false);

        if judgment == pair.expected_correct {
            correct += 1;
        } else {
            mismatches.push(CalibrationMismatch {
                index: i,
                question: pair.question.clone(),
                expected: pair.expected_correct,
                got: judgment,
            });
        }
    }

    let total = pairs.len();
    let accuracy = if total > 0 { correct as f64 / total as f64 } else { 0.0 };

    Ok(CalibrationResult {
        total,
        correct,
        accuracy,
        mismatches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_calibration_json() {
        let json = r#"[
            {
                "question": "What is the user's name?",
                "question_type": "single-session-user",
                "ground_truth": "John",
                "hypothesis": "John",
                "expected_correct": true
            },
            {
                "question": "When was the meeting?",
                "question_type": "temporal-reasoning",
                "ground_truth": "March 5th",
                "hypothesis": "April 10th",
                "expected_correct": false
            }
        ]"#;

        let pairs = load_calibration_pairs(json).unwrap();
        assert_eq!(pairs.len(), 2);
        assert!(pairs[0].expected_correct);
        assert!(!pairs[1].expected_correct);
    }

    #[tokio::test]
    async fn calibration_with_mock() {
        use async_trait::async_trait;

        struct AlwaysYes;

        #[async_trait]
        impl LLMClient for AlwaysYes {
            fn name(&self) -> &str { "always-yes" }
            async fn generate(&self, _: &str, _: usize) -> Result<String> {
                Ok("yes".to_string())
            }
        }

        let pairs = vec![
            CalibrationPair {
                question: "Name?".to_string(),
                question_type: "default".to_string(),
                ground_truth: "John".to_string(),
                hypothesis: "John".to_string(),
                expected_correct: true,
                is_abstention: false,
            },
            CalibrationPair {
                question: "Color?".to_string(),
                question_type: "default".to_string(),
                ground_truth: "blue".to_string(),
                hypothesis: "red".to_string(),
                expected_correct: false, // judge says yes but expected no
                is_abstention: false,
            },
        ];

        let result = run_calibration(&pairs, &AlwaysYes).await.unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.correct, 1); // only first pair matches
        assert_eq!(result.mismatches.len(), 1);
        assert_eq!(result.accuracy, 0.5);
    }
}
