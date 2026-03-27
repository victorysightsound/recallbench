//! Self-verification pass for benchmark answers.
//!
//! Makes a second LLM call to re-check counting, arithmetic, and version
//! selection for question types where these errors are common.
//! Ported from the historical femind benchmark verification flow for parity.

use anyhow::Result;

use crate::traits::LLMClient;

const ENABLE_SELF_VERIFICATION: bool = false;

/// Check if a question type benefits from self-verification.
pub fn needs_verification(question_type: &str, is_abstention: bool) -> bool {
    if !ENABLE_SELF_VERIFICATION {
        return false;
    }
    if is_abstention {
        return false;
    }
    matches!(question_type, "multi-session" | "knowledge-update")
}

/// Optionally verify a hypothesis by asking the model to re-check its work.
///
/// Returns the original hypothesis unchanged for question types that don't
/// need verification. For multi-session and knowledge-update questions,
/// makes a second LLM call to verify counts and computations.
pub async fn maybe_verify(
    llm: &dyn LLMClient,
    context: &str,
    question: &str,
    hypothesis: &str,
    question_type: &str,
    is_abstention: bool,
) -> Result<String> {
    if !needs_verification(question_type, is_abstention) {
        return Ok(hypothesis.to_string());
    }

    let prompt = build_verify_prompt(context, question, hypothesis, question_type);
    let verified = llm.generate(&prompt, 512).await?;

    if verified.trim().is_empty() {
        Ok(hypothesis.to_string())
    } else {
        Ok(verified)
    }
}

fn build_verify_prompt(
    context: &str,
    question: &str,
    hypothesis: &str,
    question_type: &str,
) -> String {
    let type_check = match question_type {
        "multi-session" => {
            "Your job: if the answer involves a count, re-enumerate every relevant item \
             from the chat history numbered 1, 2, 3... and recount. If the answer is a \
             list, verify every item from the chat history is accounted for and none are \
             duplicated or missing. If it involves a sum, recompute the arithmetic."
        }
        "knowledge-update" => {
            "Your job: verify the answer uses the value from the MOST RECENT \
             session in the chat history. Re-list all versions chronologically \
             from the chat history and confirm the final one is correct."
        }
        _ => "",
    };

    format!(
        "You are verifying an answer. You have the original chat history and the \
         model's answer. Check whether the final answer is correct by going back \
         to the source material.\n\n\
         Chat History:\n{context}\n\n\
         Question: {question}\n\n\
         Model's answer:\n{hypothesis}\n\n\
         {type_check}\n\n\
         If the answer is correct, output it again unchanged. \
         If you find an error (wrong count, wrong arithmetic, wrong item, wrong \
         version), output ONLY the corrected final answer — no explanation, no \
         reasoning, just the answer."
    )
}
