use crate::metrics::AccuracyMetrics;

/// Render accuracy metrics as GitHub-ready markdown.
pub fn render_accuracy_markdown(
    systems: &[(&str, &AccuracyMetrics)],
    dataset: &str,
    variant: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("## RecallBench — {dataset} {variant}\n\n"));

    // Collect all question types
    let mut all_types: Vec<String> = systems.iter()
        .flat_map(|(_, m)| m.per_type.keys().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    all_types.sort();

    // Header
    out.push_str("| System | Task-Avg | Overall |");
    for t in &all_types {
        out.push_str(&format!(" {} |", t));
    }
    out.push('\n');

    // Separator
    out.push_str("|--------|----------|---------|");
    for _ in &all_types {
        out.push_str("------|");
    }
    out.push('\n');

    // Rows
    for (name, metrics) in systems {
        out.push_str(&format!(
            "| {} | {:.1}% | {:.1}% |",
            name,
            metrics.task_averaged * 100.0,
            metrics.overall * 100.0,
        ));
        for t in &all_types {
            let acc = metrics.per_type.get(t).copied().unwrap_or(0.0);
            out.push_str(&format!(" {:.1}% |", acc * 100.0));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn markdown_output() {
        let metrics = AccuracyMetrics {
            per_type: HashMap::from([("temporal-reasoning".to_string(), 0.94)]),
            task_averaged: 0.94,
            overall: 0.887,
            abstention: None,
            total_questions: 100,
            total_correct: 89,
        };
        let md = render_accuracy_markdown(&[("MindCore", &metrics)], "longmemeval", "oracle");
        assert!(md.contains("| MindCore |"));
        assert!(md.contains("94.0%"));
        assert!(md.contains("## RecallBench"));
    }
}
