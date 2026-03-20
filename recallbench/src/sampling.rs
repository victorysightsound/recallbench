use std::collections::HashMap;

use crate::types::BenchmarkQuestion;

/// Select a stratified random subset of questions.
///
/// Maintains proportional representation from each question type.
/// For example, if 20% of questions are temporal reasoning and subset_size=50,
/// ~10 temporal reasoning questions will be selected.
pub fn stratified_sample<'a>(
    questions: &'a [BenchmarkQuestion],
    subset_size: usize,
    seed: u64,
) -> Vec<&'a BenchmarkQuestion> {
    if questions.is_empty() || subset_size >= questions.len() {
        return questions.iter().collect();
    }

    // Group by question type
    let mut by_type: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, q) in questions.iter().enumerate() {
        by_type.entry(&q.question_type).or_default().push(i);
    }

    // Calculate proportional allocation per type
    let total = questions.len() as f64;
    let mut allocations: Vec<(&str, usize, Vec<usize>)> = Vec::new();
    let mut allocated = 0usize;

    let mut types: Vec<_> = by_type.into_iter().collect();
    types.sort_by(|a, b| a.0.cmp(b.0)); // deterministic order

    for (qtype, indices) in &types {
        let proportion = indices.len() as f64 / total;
        let count = (proportion * subset_size as f64).round() as usize;
        let count = count.max(1).min(indices.len()); // at least 1, at most available
        allocations.push((qtype, count, indices.clone()));
        allocated += count;
    }

    // Adjust if rounding caused over/under allocation
    while allocated > subset_size && !allocations.is_empty() {
        // Remove from the largest group
        if let Some(max_idx) = allocations.iter().position(|(_, c, _)| *c == allocations.iter().map(|(_, c, _)| *c).max().unwrap_or(0)) {
            if allocations[max_idx].1 > 1 {
                allocations[max_idx].1 -= 1;
                allocated -= 1;
            } else {
                break;
            }
        }
    }
    while allocated < subset_size {
        // Add to the smallest group that has room
        let mut added = false;
        for alloc in &mut allocations {
            if alloc.1 < alloc.2.len() {
                alloc.1 += 1;
                allocated += 1;
                added = true;
                if allocated >= subset_size { break; }
            }
        }
        if !added { break; }
    }

    // Select from each group using deterministic pseudo-random shuffle
    let mut selected_indices: Vec<usize> = Vec::with_capacity(subset_size);
    for (_qtype, count, indices) in &allocations {
        let mut shuffled = indices.clone();
        deterministic_shuffle(&mut shuffled, seed);
        selected_indices.extend(shuffled.into_iter().take(*count));
    }

    // Sort by original order for consistency
    selected_indices.sort();
    selected_indices.iter().map(|&i| &questions[i]).collect()
}

/// Simple deterministic shuffle using xorshift64.
fn deterministic_shuffle(items: &mut [usize], seed: u64) {
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    let len = items.len();
    if len <= 1 { return; }

    for i in (1..len).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let j = (state as usize) % (i + 1);
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as StdHashMap;

    fn make_questions(types_and_counts: &[(&str, usize)]) -> Vec<BenchmarkQuestion> {
        let mut questions = Vec::new();
        let mut id = 0;
        for (qtype, count) in types_and_counts {
            for _ in 0..*count {
                questions.push(BenchmarkQuestion {
                    id: format!("q{id:03}"),
                    question_type: qtype.to_string(),
                    question: "test?".to_string(),
                    ground_truth: vec!["answer".to_string()],
                    question_date: None,
                    sessions: vec![],
                    is_abstention: false,
                    metadata: StdHashMap::new(),
                });
                id += 1;
            }
        }
        questions
    }

    #[test]
    fn returns_all_if_subset_larger() {
        let questions = make_questions(&[("a", 10)]);
        let subset = stratified_sample(&questions, 20, 42);
        assert_eq!(subset.len(), 10);
    }

    #[test]
    fn returns_empty_for_empty() {
        let questions: Vec<BenchmarkQuestion> = vec![];
        let subset = stratified_sample(&questions, 10, 42);
        assert!(subset.is_empty());
    }

    #[test]
    fn maintains_proportions() {
        // 60% type A, 40% type B out of 100 questions
        let questions = make_questions(&[("a", 60), ("b", 40)]);
        let subset = stratified_sample(&questions, 50, 42);

        assert_eq!(subset.len(), 50);

        let mut type_counts: StdHashMap<&str, usize> = StdHashMap::new();
        for q in &subset {
            *type_counts.entry(&q.question_type).or_insert(0) += 1;
        }

        // Should be approximately 30 A and 20 B
        let a_count = type_counts.get("a").copied().unwrap_or(0);
        let b_count = type_counts.get("b").copied().unwrap_or(0);
        assert!(a_count >= 28 && a_count <= 32, "Expected ~30 type A, got {a_count}");
        assert!(b_count >= 18 && b_count <= 22, "Expected ~20 type B, got {b_count}");
    }

    #[test]
    fn deterministic_with_same_seed() {
        let questions = make_questions(&[("a", 50), ("b", 30), ("c", 20)]);
        let s1 = stratified_sample(&questions, 30, 42);
        let s2 = stratified_sample(&questions, 30, 42);

        let ids1: Vec<_> = s1.iter().map(|q| q.id.as_str()).collect();
        let ids2: Vec<_> = s2.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids1, ids2);
    }

    #[test]
    fn different_seed_different_selection() {
        let questions = make_questions(&[("a", 50), ("b", 50)]);
        let s1 = stratified_sample(&questions, 20, 42);
        let s2 = stratified_sample(&questions, 20, 99);

        let ids1: Vec<_> = s1.iter().map(|q| q.id.as_str()).collect();
        let ids2: Vec<_> = s2.iter().map(|q| q.id.as_str()).collect();
        assert_ne!(ids1, ids2);
    }

    #[test]
    fn at_least_one_per_type() {
        // 5 types, each with different counts, subset of 10
        let questions = make_questions(&[("a", 50), ("b", 30), ("c", 10), ("d", 5), ("e", 5)]);
        let subset = stratified_sample(&questions, 10, 42);

        let mut type_counts: StdHashMap<&str, usize> = StdHashMap::new();
        for q in &subset {
            *type_counts.entry(&q.question_type).or_insert(0) += 1;
        }

        // Each type should have at least 1
        assert!(type_counts.contains_key("a"));
        assert!(type_counts.contains_key("b"));
        assert!(type_counts.contains_key("c"));
        assert!(type_counts.contains_key("d"));
        assert!(type_counts.contains_key("e"));
    }
}
