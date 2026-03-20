use comfy_table::{Table, ContentArrangement, Cell, Attribute};

use crate::metrics::{AccuracyMetrics, LatencyMetrics, CostMetrics};

/// Render accuracy metrics as a terminal table.
pub fn render_accuracy_table(
    systems: &[(&str, &AccuracyMetrics)],
    dataset: &str,
    variant: &str,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("RecallBench — {dataset} {variant}\n"));
    output.push_str(&"═".repeat(70));
    output.push('\n');

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    // Collect all question types across all systems
    let mut all_types: Vec<String> = systems.iter()
        .flat_map(|(_, m)| m.per_type.keys().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    all_types.sort();

    // Header
    let mut header = vec![
        Cell::new("System").add_attribute(Attribute::Bold),
        Cell::new("Task-Avg").add_attribute(Attribute::Bold),
        Cell::new("Overall").add_attribute(Attribute::Bold),
    ];
    for t in &all_types {
        let short = abbreviate_type(t);
        header.push(Cell::new(short).add_attribute(Attribute::Bold));
    }
    if systems.iter().any(|(_, m)| m.abstention.is_some()) {
        header.push(Cell::new("ABS").add_attribute(Attribute::Bold));
    }
    table.set_header(header);

    // Rows
    for (name, metrics) in systems {
        let mut row = vec![
            Cell::new(name),
            Cell::new(format!("{:.1}%", metrics.task_averaged * 100.0)),
            Cell::new(format!("{:.1}%", metrics.overall * 100.0)),
        ];
        for t in &all_types {
            let acc = metrics.per_type.get(t).copied().unwrap_or(0.0);
            row.push(Cell::new(format!("{:.1}%", acc * 100.0)));
        }
        if systems.iter().any(|(_, m)| m.abstention.is_some()) {
            let abs = metrics.abstention.unwrap_or(0.0);
            row.push(Cell::new(format!("{:.1}%", abs * 100.0)));
        }
        table.add_row(row);
    }

    output.push_str(&table.to_string());
    output
}

/// Render latency metrics as a terminal table.
pub fn render_latency_table(systems: &[(&str, &LatencyMetrics)]) -> String {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("System").add_attribute(Attribute::Bold),
        Cell::new("Ingest (p50/p95/p99)").add_attribute(Attribute::Bold),
        Cell::new("Retrieval").add_attribute(Attribute::Bold),
        Cell::new("Generation").add_attribute(Attribute::Bold),
        Cell::new("Judge").add_attribute(Attribute::Bold),
        Cell::new("Total").add_attribute(Attribute::Bold),
    ]);

    for (name, m) in systems {
        table.add_row(vec![
            Cell::new(name),
            Cell::new(format!("{:.0}/{:.0}/{:.0}", m.ingest.p50, m.ingest.p95, m.ingest.p99)),
            Cell::new(format!("{:.0}/{:.0}/{:.0}", m.retrieval.p50, m.retrieval.p95, m.retrieval.p99)),
            Cell::new(format!("{:.0}/{:.0}/{:.0}", m.generation.p50, m.generation.p95, m.generation.p99)),
            Cell::new(format!("{:.0}/{:.0}/{:.0}", m.judge.p50, m.judge.p95, m.judge.p99)),
            Cell::new(format!("{:.0}/{:.0}/{:.0}", m.total.p50, m.total.p95, m.total.p99)),
        ]);
    }

    format!("\nLatency (ms) — p50/p95/p99\n{}", table)
}

/// Render cost metrics as a terminal table.
pub fn render_cost_table(systems: &[(&str, &CostMetrics)]) -> String {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("System").add_attribute(Attribute::Bold),
        Cell::new("Tokens In").add_attribute(Attribute::Bold),
        Cell::new("Tokens Out").add_attribute(Attribute::Bold),
        Cell::new("Est. Cost").add_attribute(Attribute::Bold),
    ]);

    for (name, c) in systems {
        table.add_row(vec![
            Cell::new(name),
            Cell::new(format_number(c.total_tokens_in)),
            Cell::new(format_number(c.total_tokens_out)),
            Cell::new(format!("${:.2}", c.estimated_cost_usd)),
        ]);
    }

    format!("\nCost\n{}", table)
}

fn abbreviate_type(t: &str) -> &str {
    match t {
        "single-session-user" => "SS-User",
        "single-session-assistant" => "SS-Asst",
        "single-session-preference" => "SS-Pref",
        "multi-session" => "MS",
        "knowledge-update" => "KU",
        "temporal-reasoning" => "TR",
        _ => t,
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn render_accuracy() {
        let metrics = AccuracyMetrics {
            per_type: HashMap::from([
                ("temporal-reasoning".to_string(), 0.94),
                ("knowledge-update".to_string(), 0.91),
            ]),
            task_averaged: 0.925,
            overall: 0.887,
            abstention: Some(0.85),
            total_questions: 500,
            total_correct: 444,
        };
        let output = render_accuracy_table(
            &[("MindCore", &metrics)],
            "longmemeval",
            "oracle",
        );
        assert!(output.contains("MindCore"));
        assert!(output.contains("92.5%"));
        assert!(output.contains("longmemeval"));
    }

    #[test]
    fn format_numbers() {
        assert_eq!(format_number(500), "500");
        assert_eq!(format_number(1500), "1.5K");
        assert_eq!(format_number(1_500_000), "1.5M");
    }
}
