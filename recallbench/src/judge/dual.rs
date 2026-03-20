use anyhow::Result;

use crate::traits::LLMClient;
use super::parse_judgment;

/// Result of a dual-model judgment.
#[derive(Debug, Clone)]
pub struct DualJudgment {
    pub is_correct: bool,
    pub primary_judgment: bool,
    pub tiebreaker_used: bool,
    pub tiebreaker_judgment: Option<bool>,
    pub disagreement: bool,
}

/// Run dual-model judging with a primary and tiebreaker judge.
///
/// The tiebreaker is only invoked if the primary judge returns an
/// ambiguous response (not clearly "yes" or "no").
pub async fn dual_judge(
    prompt: &str,
    primary: &dyn LLMClient,
    tiebreaker: Option<&dyn LLMClient>,
) -> Result<DualJudgment> {
    let primary_response = primary.generate(prompt, 10).await?;
    let primary_result = parse_judgment(&primary_response);

    match primary_result {
        Some(is_correct) => {
            Ok(DualJudgment {
                is_correct,
                primary_judgment: is_correct,
                tiebreaker_used: false,
                tiebreaker_judgment: None,
                disagreement: false,
            })
        }
        None => {
            // Ambiguous primary response — invoke tiebreaker
            if let Some(tiebreaker) = tiebreaker {
                let tiebreaker_response = tiebreaker.generate(prompt, 10).await?;
                let tiebreaker_result = parse_judgment(&tiebreaker_response);
                let is_correct = tiebreaker_result.unwrap_or(false);

                Ok(DualJudgment {
                    is_correct,
                    primary_judgment: false, // primary was ambiguous, default false
                    tiebreaker_used: true,
                    tiebreaker_judgment: Some(is_correct),
                    disagreement: true,
                })
            } else {
                // No tiebreaker, default to incorrect
                tracing::warn!("Ambiguous judge response with no tiebreaker: {primary_response}");
                Ok(DualJudgment {
                    is_correct: false,
                    primary_judgment: false,
                    tiebreaker_used: false,
                    tiebreaker_judgment: None,
                    disagreement: false,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockJudge(String);

    #[async_trait]
    impl LLMClient for MockJudge {
        fn name(&self) -> &str { "mock" }
        async fn generate(&self, _prompt: &str, _max_tokens: usize) -> Result<String> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn clear_yes() {
        let primary = MockJudge("yes".to_string());
        let result = dual_judge("test", &primary, None).await.unwrap();
        assert!(result.is_correct);
        assert!(!result.tiebreaker_used);
    }

    #[tokio::test]
    async fn clear_no() {
        let primary = MockJudge("no".to_string());
        let result = dual_judge("test", &primary, None).await.unwrap();
        assert!(!result.is_correct);
        assert!(!result.tiebreaker_used);
    }

    #[tokio::test]
    async fn ambiguous_with_tiebreaker() {
        let primary = MockJudge("maybe sort of".to_string());
        let tiebreaker = MockJudge("yes".to_string());
        let result = dual_judge("test", &primary, Some(&tiebreaker)).await.unwrap();
        assert!(result.is_correct);
        assert!(result.tiebreaker_used);
        assert!(result.disagreement);
    }

    #[tokio::test]
    async fn ambiguous_without_tiebreaker() {
        let primary = MockJudge("unclear".to_string());
        let result = dual_judge("test", &primary, None).await.unwrap();
        assert!(!result.is_correct);
        assert!(!result.tiebreaker_used);
    }
}
