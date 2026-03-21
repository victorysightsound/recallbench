/// Generate a judge prompt for evaluating an answer.
/// Ported from mindcore-bench v3 for result parity.
pub fn judge_prompt(
    question_type: &str,
    question: &str,
    ground_truth: &str,
    hypothesis: &str,
    is_abstention: bool,
) -> String {
    let instruction = if is_abstention {
        "You are evaluating a conversational AI assistant's response. \
         The question is designed to be unanswerable based on the chat history. \
         Answer \"yes\" if the model's PRIMARY response indicates it cannot answer \
         from the chat history. This includes responses that:\n\
         - Say \"I don't know\", \"not available\", \"cannot determine\", or similar\n\
         - State the information is not in the conversations\n\
         - Correctly identify WHY it can't answer (e.g., \"You mentioned tennis, not table tennis\")\n\
         - Explain what IS in the chat history while still concluding they can't answer the specific question\n\
         The model may provide context about what it DID find — this is fine as long as the \
         conclusion is that it CANNOT answer the specific question asked.\n\
         Answer \"no\" ONLY if the model provides a specific, direct answer to the question \
         without indicating it can't answer. A hedge like \"I'm not sure but...\" followed \
         by a concrete answer counts as \"no\"."
            .to_string()
    } else {
        match question_type {
            "temporal-reasoning" => {
                "You are evaluating a conversational AI assistant's response. \
                 The model's response may contain detailed step-by-step date calculations. \
                 Look for the final computed answer anywhere in the response. \
                 Accept equivalent representations (e.g., '3 weeks' vs '21 days', \
                 'two' vs '2'). Do not penalize off-by-one errors for days, weeks, or months. \
                 If the required answer is a list or sequence, check that ALL items appear \
                 somewhere in the response. \
                 Answer \"yes\" if the response contains the correct answer or is equivalent \
                 to the correct answer. Answer \"no\" otherwise."
                    .to_string()
            }
            "knowledge-update" => {
                "You are evaluating a conversational AI assistant's response. \
                 The response may discuss both old and new values at length. \
                 As long as the updated/most-recent value appears as the primary or final \
                 answer, mark it correct. It is acceptable if the response also mentions \
                 old information. \
                 Answer \"yes\" if the most recent/updated value is present as the answer. \
                 Answer \"no\" otherwise."
                    .to_string()
            }
            "single-session-preference" => {
                "You are evaluating a conversational AI assistant's response. \
                 The response should describe the user's preferences. \
                 Answer \"yes\" if the response captures the essential preference described \
                 in the required answer, even if phrased differently or with different \
                 specific examples. The key is whether the CORE PREFERENCE (e.g., the topic, \
                 the style, the type of content) matches. Word-for-word match is NOT required. \
                 Answer \"no\" if the core preference is missed, contradicted, or the response \
                 is a direct answer to the question rather than a preference description."
                    .to_string()
            }
            "multi-session" => {
                "You are evaluating a conversational AI assistant's response. \
                 The model's response may be long and contain step-by-step reasoning. \
                 Search the ENTIRE response for the required answer. \
                 If the required answer is a number, accept it if the correct number appears \
                 in the response's final answer or conclusion. \
                 If the required answer is a list of items, verify ALL items are mentioned \
                 somewhere in the response. \
                 Accept equivalent phrasings and minor variations. \
                 Answer \"yes\" if all required information is present. Answer \"no\" otherwise."
                    .to_string()
            }
            _ => {
                "You are evaluating a conversational AI assistant's response. \
                 The model's response may contain step-by-step reasoning. \
                 Search the ENTIRE response for the required answer — it may appear \
                 in the middle of the reasoning, not just at the end. \
                 Accept equivalent phrasings and representations. \
                 Answer \"yes\" if the response contains the correct answer or is equivalent \
                 to the correct answer. Answer \"no\" otherwise."
                    .to_string()
            }
        }
    };

    format!(
        "{instruction}\n\n\
         Question: {question}\n\
         Required Answer: {ground_truth}\n\
         Model's Response: {hypothesis}\n\n\
         Is the model's response correct? Answer ONLY \"yes\" or \"no\", nothing else."
    )
}

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
        assert!(prompt.contains("off-by-one"));
        assert!(prompt.contains("equivalent representations"));
    }

    #[test]
    fn knowledge_update_prompt() {
        let prompt = judge_prompt("knowledge-update", "Phone?", "iPhone 15", "iPhone 15", false);
        assert!(prompt.contains("most-recent value"));
    }

    #[test]
    fn preference_prompt() {
        let prompt = judge_prompt("single-session-preference", "Color?", "blue", "blue", false);
        assert!(prompt.contains("CORE PREFERENCE"));
    }

    #[test]
    fn multi_session_prompt() {
        let prompt = judge_prompt("multi-session", "Summary?", "A and B", "A and B", false);
        assert!(prompt.contains("ENTIRE response"));
    }

    #[test]
    fn default_prompt() {
        let prompt = judge_prompt("single-session-user", "Name?", "John", "John", false);
        assert!(prompt.contains("equivalent"));
    }
}
