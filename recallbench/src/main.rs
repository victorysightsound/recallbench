mod config;
mod datasets;
mod errors;
mod judge;
mod llm;
mod metrics;
mod report;
mod resume;
mod runner;
mod sampling;
mod systems;
mod traits;
mod types;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "recallbench")]
#[command(about = "A universal benchmark harness for AI memory systems")]
#[command(version)]
struct Cli {
    /// Path to recallbench.toml config file
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available datasets
    Datasets,

    /// Download a dataset
    Download {
        /// Dataset name (e.g., longmemeval)
        dataset: String,
        /// Dataset variant (e.g., oracle, small, medium)
        #[arg(long, default_value = "oracle")]
        variant: String,
        /// Force re-download even if cached
        #[arg(long)]
        force: bool,
    },

    /// Run benchmark against a memory system
    Run {
        /// System name or config file
        #[arg(long)]
        system: Option<String>,
        /// Path to system TOML config (for HTTP/subprocess adapters)
        #[arg(long)]
        system_config: Option<PathBuf>,
        /// Dataset name
        #[arg(long, default_value = "longmemeval")]
        dataset: String,
        /// Dataset variant
        #[arg(long, default_value = "oracle")]
        variant: String,
        /// Concurrency level
        #[arg(long)]
        concurrency: Option<usize>,
        /// Token budget for context retrieval
        #[arg(long)]
        budget: Option<usize>,
        /// Model for answer generation
        #[arg(long)]
        gen_model: Option<String>,
        /// Model for judging answers
        #[arg(long)]
        judge_model: Option<String>,
        /// Output file path
        #[arg(long)]
        output: Option<PathBuf>,
        /// Random seed for reproducibility
        #[arg(long)]
        seed: Option<u64>,
        /// Filter by question types (comma-separated)
        #[arg(long)]
        filter: Option<String>,
        /// Resume from existing results
        #[arg(long)]
        resume: bool,
        /// Quick mode: evaluate a stratified random subset
        #[arg(long, alias = "dev")]
        quick: bool,
        /// Number of questions in quick mode subset
        #[arg(long)]
        quick_size: Option<usize>,
    },

    /// Compare multiple systems
    Compare {
        /// Comma-separated system names
        #[arg(long)]
        systems: String,
        /// Dataset name
        #[arg(long, default_value = "longmemeval")]
        dataset: String,
        /// Dataset variant
        #[arg(long, default_value = "oracle")]
        variant: String,
        /// Output file
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Generate report from results
    Report {
        /// Path to results file (JSONL or JSON)
        path: PathBuf,
        /// Output format
        #[arg(long, default_value = "table")]
        format: String,
        /// Output file (default: stdout)
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Show dataset statistics
    Stats {
        /// Dataset name
        dataset: String,
        /// Dataset variant
        #[arg(long, default_value = "oracle")]
        variant: String,
    },

    /// Validate a custom dataset
    Validate {
        /// Path to dataset JSON file
        path: PathBuf,
    },

    /// Run judge calibration
    Calibrate {
        /// Judge model to calibrate
        #[arg(long, default_value = "claude-sonnet")]
        judge_model: String,
        /// Dataset to use calibration pairs from
        #[arg(long, default_value = "longmemeval")]
        dataset: String,
    },

    /// Export failure analysis from results
    Failures {
        /// Path to results file
        path: PathBuf,
        /// Export to file
        #[arg(long)]
        export: Option<PathBuf>,
        /// Filter by question type
        #[arg(long)]
        type_filter: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .init();

    // Load config
    let config_path = cli.config.unwrap_or_else(|| PathBuf::from("recallbench.toml"));
    let cfg = config::Config::load(&config_path)?.with_env_overrides();

    match cli.command {
        Commands::Datasets => cmd_datasets(),
        Commands::Download { dataset, variant, force } => {
            cmd_download(&dataset, &variant, force).await
        }
        Commands::Stats { dataset, variant } => {
            cmd_stats(&dataset, &variant).await
        }
        Commands::Validate { path } => cmd_validate(&path),
        Commands::Report { path, format, output } => {
            cmd_report(&path, &format, output.as_deref())
        }
        Commands::Failures { path, export, type_filter } => {
            cmd_failures(&path, export.as_deref(), type_filter.as_deref())
        }
        Commands::Run {
            system, system_config, dataset, variant,
            concurrency, budget, gen_model, judge_model,
            output, seed: _seed, filter, resume,
            quick, quick_size,
        } => {
            let system_name = system.as_deref().unwrap_or("echo");
            let gen_m = gen_model.as_deref().unwrap_or(&cfg.defaults.gen_model);
            let jdg = judge_model.as_deref().unwrap_or(&cfg.defaults.judge_model);
            let conc = concurrency.unwrap_or(cfg.defaults.concurrency);
            let bgt = budget.unwrap_or(cfg.defaults.token_budget);
            let out = output.unwrap_or_else(|| {
                PathBuf::from(&cfg.defaults.output_dir)
                    .join(format!("{system_name}-{dataset}-{variant}.jsonl"))
            });
            let filter_types = filter.map(|f| {
                f.split(',').map(|s| s.trim().to_string()).collect()
            });
            let qsize = if quick {
                Some(quick_size.unwrap_or(cfg.defaults.quick_size))
            } else {
                None
            };

            cmd_run(
                system_name, system_config.as_deref(),
                &dataset, &variant, conc, bgt,
                gen_m, jdg, &out, filter_types, resume, qsize,
            ).await
        }
        Commands::Compare { systems, dataset, variant, output } => {
            println!("Compare: {systems} on {dataset}/{variant}");
            println!("(Not yet implemented — use 'run' for individual systems)");
            Ok(())
        }
        Commands::Calibrate { judge_model, dataset } => {
            println!("Calibrate: {judge_model} on {dataset}");
            println!("(Calibration data not yet bundled)");
            Ok(())
        }
    }
}

fn cmd_datasets() -> Result<()> {
    let registry = datasets::DatasetRegistry::new();
    println!("Available datasets:\n");
    for info in registry.list() {
        println!("  {} — {}", info.name, info.description);
        println!("    Variants: {}", info.variants.join(", "));
        println!();
    }
    Ok(())
}

async fn cmd_download(dataset: &str, variant: &str, force: bool) -> Result<()> {
    let registry = datasets::DatasetRegistry::new();
    registry.load(dataset, variant, force).await?;
    println!("Dataset ready.");
    Ok(())
}

async fn cmd_stats(dataset: &str, variant: &str) -> Result<()> {
    let registry = datasets::DatasetRegistry::new();
    let ds = registry.load(dataset, variant, false).await?;

    println!("Dataset: {} ({})", ds.name(), ds.variant());
    println!("Description: {}", ds.description());
    println!("Questions: {}", ds.questions().len());
    println!("Question types: {}", ds.question_types().join(", "));

    if !ds.questions().is_empty() {
        // Print type distribution if available
        let mut type_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for q in ds.questions() {
            *type_counts.entry(&q.question_type).or_insert(0) += 1;
        }
        println!("\nType distribution:");
        let mut counts: Vec<_> = type_counts.into_iter().collect();
        counts.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        for (t, c) in counts {
            println!("  {t}: {c}");
        }
    }

    Ok(())
}

fn cmd_validate(path: &std::path::Path) -> Result<()> {
    use crate::traits::BenchmarkDataset;
    let ds = datasets::custom::CustomDataset::load(path)?;
    let errors = ds.validate();
    if errors.is_empty() {
        println!("Dataset is valid. {} questions.", ds.questions().len());
    } else {
        println!("Validation errors:");
        for e in &errors {
            println!("  - {e}");
        }
        anyhow::bail!("{} validation errors found", errors.len());
    }
    Ok(())
}

fn cmd_report(path: &std::path::Path, format: &str, output: Option<&std::path::Path>) -> Result<()> {
    let results = resume::load_results(path)?;
    if results.is_empty() {
        println!("No results found in {}", path.display());
        return Ok(());
    }

    let acc = metrics::compute_accuracy(&results);
    let lat = metrics::compute_latency(&results);
    let cost = metrics::compute_cost(&results, &metrics::Pricing::default());

    let system_name = results[0].system_name.as_str();
    let fmt = report::ReportFormat::from_str(format)?;

    let output_str = match fmt {
        report::ReportFormat::Table => {
            let mut out = report::table::render_accuracy_table(
                &[(system_name, &acc)], "benchmark", "results",
            );
            out.push_str(&report::table::render_latency_table(&[(system_name, &lat)]));
            out.push_str(&report::table::render_cost_table(&[(system_name, &cost)]));
            out
        }
        report::ReportFormat::Markdown => {
            report::markdown::render_accuracy_markdown(
                &[(system_name, &acc)], "benchmark", "results",
            )
        }
        report::ReportFormat::Json => {
            let json_report = report::json::build_json_report(
                "benchmark", "results",
                &[(system_name, &acc, Some(&lat), Some(&cost))],
            );
            serde_json::to_string_pretty(&json_report)?
        }
        report::ReportFormat::Csv => {
            let mut buf = Vec::new();
            report::csv::write_csv(&mut buf, &results)?;
            String::from_utf8(buf)?
        }
    };

    if let Some(out_path) = output {
        std::fs::write(out_path, &output_str)?;
        println!("Report written to {}", out_path.display());
    } else {
        println!("{output_str}");
    }

    Ok(())
}

fn cmd_failures(path: &std::path::Path, export: Option<&std::path::Path>, type_filter: Option<&str>) -> Result<()> {
    let results = resume::load_results(path)?;
    let analysis = report::failure::analyze_failures(&results);

    println!("Failures: {} / {} questions", analysis.total_failures, analysis.total_questions);

    for (qtype, failures) in &analysis.by_type {
        if let Some(filter) = type_filter {
            if qtype != filter { continue; }
        }
        println!("\n  {} ({} failures):", qtype, failures.len());
        for f in failures.iter().take(5) {
            println!("    {} — expected: {}, got: {}", f.question_id, f.ground_truth, f.hypothesis);
        }
        if failures.len() > 5 {
            println!("    ... and {} more", failures.len() - 5);
        }
    }

    if let Some(path) = export {
        let json = serde_json::to_string_pretty(&analysis)?;
        std::fs::write(path, json)?;
        println!("\nExported to {}", path.display());
    }

    Ok(())
}

async fn cmd_run(
    system_name: &str,
    system_config: Option<&std::path::Path>,
    dataset: &str,
    variant: &str,
    concurrency: usize,
    budget: usize,
    gen_model: &str,
    judge_model: &str,
    output: &std::path::Path,
    filter_types: Option<Vec<String>>,
    do_resume: bool,
    quick_size: Option<usize>,
) -> Result<()> {
    // Create output directory
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Load dataset
    let registry = datasets::DatasetRegistry::new();
    let ds = registry.load(dataset, variant, false).await?;

    // Apply quick mode stratified sampling if requested
    if let Some(size) = quick_size {
        tracing::info!("Quick mode: stratified sampling {} questions from {}", size, ds.questions().len());
    }

    // Create system adapter
    let system: Box<dyn traits::MemorySystem> = if let Some(config_path) = system_config {
        let toml_str = std::fs::read_to_string(config_path)?;
        Box::new(systems::http::HttpSystemAdapter::from_toml(&toml_str)?)
    } else {
        match system_name {
            "echo" => Box::new(systems::echo::EchoSystem::new()),
            _ => anyhow::bail!("Unknown system: {system_name}. Use --system-config for custom systems."),
        }
    };

    // Create LLM clients
    let gen_llm: Arc<dyn traits::LLMClient> = Arc::new(
        llm::cli::CliLLMClient::new(
            &llm::LLMRegistry::resolve_provider(gen_model).0,
            gen_model,
        ),
    );
    let judge_llm: Arc<dyn traits::LLMClient> = Arc::new(
        llm::cli::CliLLMClient::new(
            &llm::LLMRegistry::resolve_provider(judge_model).0,
            judge_model,
        ),
    );

    // Run benchmark
    let run_config = runner::RunConfig {
        concurrency,
        token_budget: budget,
        output_path: output.to_path_buf(),
        filter_types,
        resume: do_resume,
        quick_size,
    };

    let results = runner::run_benchmark(
        system.as_ref(),
        ds.as_ref(),
        gen_llm,
        judge_llm,
        &run_config,
    ).await?;

    // Print summary
    if !results.is_empty() {
        let acc = metrics::compute_accuracy(&results);
        let lat = metrics::compute_latency(&results);
        let cost = metrics::compute_cost(&results, &metrics::Pricing::default());

        let mode_note = if let Some(size) = quick_size {
            format!(" (quick mode: {size} questions, stratified)")
        } else {
            String::new()
        };

        let report_output = report::table::render_accuracy_table(
            &[(system.name(), &acc)], dataset, &format!("{variant}{mode_note}"),
        );
        println!("\n{report_output}");
        println!("{}", report::table::render_latency_table(&[(system.name(), &lat)]));
        println!("{}", report::table::render_cost_table(&[(system.name(), &cost)]));
    }

    Ok(())
}
