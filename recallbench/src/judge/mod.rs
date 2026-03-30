pub mod calibration;
pub mod dual;
pub mod prompts;

use anyhow::Result;

use crate::traits::LLMClient;

/// Judge a single answer using an LLM.
///
/// Selects the appropriate prompt template based on question type,
/// sends it to the judge LLM, and parses the yes/no response.
pub async fn judge_answer(
    question_type: &str,
    question: &str,
    ground_truth: &str,
    hypothesis: &str,
    is_abstention: bool,
    judge_llm: &dyn LLMClient,
) -> Result<bool> {
    let prompt = prompts::judge_prompt(
        question_type,
        question,
        ground_truth,
        hypothesis,
        is_abstention,
    );

    let response = judge_llm.generate(&prompt, 64).await?;

    Ok(parse_judgment(&response).unwrap_or(false))
}

/// Parse a judge response into a boolean.
///
/// Returns Some(true) for "yes", Some(false) for "no", None for ambiguous.
pub fn parse_judgment(response: &str) -> Option<bool> {
    let lower = response.to_lowercase().trim().to_string();

    if lower == "yes" || lower.starts_with("yes.") || lower.starts_with("yes,") || lower.starts_with("yes ") {
        Some(true)
    } else if lower == "no" || lower.starts_with("no.") || lower.starts_with("no,") || lower.starts_with("no ") {
        Some(false)
    } else if lower.contains("yes") && !lower.contains("no") {
        Some(true)
    } else if lower.contains("no") && !lower.contains("yes") {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockJudge(String);

    #[async_trait]
    impl LLMClient for MockJudge {
        fn name(&self) -> &str { "mock-judge" }
        async fn generate(&self, _prompt: &str, _max_tokens: usize) -> Result<String> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn parse_yes_variants() {
        assert_eq!(parse_judgment("yes"), Some(true));
        assert_eq!(parse_judgment("Yes"), Some(true));
        assert_eq!(parse_judgment("YES"), Some(true));
        assert_eq!(parse_judgment("yes."), Some(true));
        assert_eq!(parse_judgment("yes, the answer is correct"), Some(true));
    }

    #[test]
    fn parse_no_variants() {
        assert_eq!(parse_judgment("no"), Some(false));
        assert_eq!(parse_judgment("No"), Some(false));
        assert_eq!(parse_judgment("NO"), Some(false));
        assert_eq!(parse_judgment("no."), Some(false));
        assert_eq!(parse_judgment("no, the answer is wrong"), Some(false));
    }

    #[test]
    fn parse_ambiguous() {
        assert_eq!(parse_judgment("maybe"), None);
        assert_eq!(parse_judgment("I'm unsure"), None);
        assert_eq!(parse_judgment(""), None);
    }

    #[test]
    fn parse_contains_yes() {
        assert_eq!(parse_judgment("The answer is yes"), Some(true));
    }

    #[test]
    fn parse_contains_no() {
        assert_eq!(parse_judgment("The answer is no"), Some(false));
    }

    #[tokio::test]
    async fn judge_answer_yes() {
        let judge = MockJudge("yes".to_string());
        let result = judge_answer("default", "Q?", "A", "A", false, &judge).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn judge_answer_no() {
        let judge = MockJudge("no".to_string());
        let result = judge_answer("default", "Q?", "A", "B", false, &judge).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn judge_ambiguous_defaults_false() {
        let judge = MockJudge("unclear".to_string());
        let result = judge_answer("default", "Q?", "A", "B", false, &judge).await.unwrap();
        assert!(!result);
    }
}
