/// Generate a judge prompt for evaluating an answer.
///
/// Returns a prompt that asks the judge LLM for a binary "yes" or "no" response.
pub fn judge_prompt(
    question_type: &str,
    question: &str,
    ground_truth: &str,
    hypothesis: &str,
    is_abstention: bool,
) -> String {
    let instruction = if is_abstention {
        ABSTENTION_INSTRUCTION
    } else {
        match question_type {
            "temporal-reasoning" => TEMPORAL_INSTRUCTION,
            "knowledge-update" => KNOWLEDGE_UPDATE_INSTRUCTION,
            "single-session-preference" => PREFERENCE_INSTRUCTION,
            "multi-session" => MULTI_SESSION_INSTRUCTION,
            _ => DEFAULT_INSTRUCTION,
        }
    };

    format!(
        r#"You are a strict evaluator judging whether a model's response correctly answers a question.

Question: {question}
Ground Truth Answer: {ground_truth}
Model's Response: {hypothesis}

Evaluation Criteria:
{instruction}

Based on the criteria above, does the model's response correctly answer the question?
Respond with exactly "yes" or "no" and nothing else."#
    )
}

const ABSTENTION_INSTRUCTION: &str =
    "The question is unanswerable given the available information. \
     The model should have indicated that it cannot answer or doesn't have enough information. \
     Did the model correctly identify this question as unanswerable?";

const TEMPORAL_INSTRUCTION: &str =
    "Focus on temporal correctness. Allow off-by-one errors for days, weeks, or months. \
     The key is whether the temporal relationship and approximate timing are correct, \
     not exact precision. Accept semantically equivalent time references.";

const KNOWLEDGE_UPDATE_INSTRUCTION: &str =
    "The ground truth reflects the most recently updated information. \
     Accept the response if the updated (most recent) answer is presented as the primary answer. \
     The model should not give an outdated answer.";

const PREFERENCE_INSTRUCTION: &str =
    "Does the response correctly recall and utilize the user's stated personal preference? \
     The response should demonstrate awareness of the specific preference mentioned.";

const MULTI_SESSION_INSTRUCTION: &str =
    "The question requires synthesizing information from across multiple conversation sessions. \
     Does the response correctly combine relevant details from different sessions to provide \
     a complete and accurate answer?";

const DEFAULT_INSTRUCTION: &str =
    "Does the response contain the correct answer or a semantically equivalent statement? \
     Minor phrasing differences are acceptable as long as the core information is correct.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abstention_prompt() {
        let prompt = judge_prompt("any", "Q?", "N/A", "I don't know", true);
        assert!(prompt.contains("unanswerable"));
        assert!(prompt.contains("yes\" or \"no\""));
    }

    #[test]
    fn temporal_prompt() {
        let prompt = judge_prompt("temporal-reasoning", "When?", "March", "March 5th", false);
        assert!(prompt.contains("temporal"));
        assert!(prompt.contains("off-by-one"));
    }

    #[test]
    fn knowledge_update_prompt() {
        let prompt = judge_prompt("knowledge-update", "Phone?", "iPhone 15", "iPhone 15", false);
        assert!(prompt.contains("updated"));
    }

    #[test]
    fn preference_prompt() {
        let prompt = judge_prompt("single-session-preference", "Color?", "blue", "blue", false);
        assert!(prompt.contains("preference"));
    }

    #[test]
    fn multi_session_prompt() {
        let prompt = judge_prompt("multi-session", "Summary?", "A and B", "A and B", false);
        assert!(prompt.contains("multiple conversation sessions"));
    }

    #[test]
    fn default_prompt() {
        let prompt = judge_prompt("single-session-user", "Name?", "John", "John", false);
        assert!(prompt.contains("semantically equivalent"));
    }

    #[test]
    fn prompt_includes_all_parts() {
        let prompt = judge_prompt("default", "Q?", "A", "R", false);
        assert!(prompt.contains("Q?"));
        assert!(prompt.contains("A"));
        assert!(prompt.contains("R"));
    }
}
