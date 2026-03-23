#![allow(dead_code)] // Library code is used through CLI subcommands, not all paths exercised in every build

mod checkpoint;
mod config;
mod datasets;
#[cfg(feature = "mindcore-adapter")]
mod embedding_cache;
mod errors;
mod judge;
mod llm;
mod longevity;
mod metrics;
mod report;
mod resume;
mod retrieval_test;
mod runner;
mod sampling;
mod systems;
mod traits;
mod types;
mod verify;
mod web;

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
        /// Stress test: run N times and report variance
        #[arg(long)]
        stress: Option<usize>,
        /// Budget sweep: run at multiple token budgets
        #[arg(long)]
        budget_sweep: bool,
        /// Note describing the purpose of this run
        #[arg(long)]
        note: Option<String>,
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

    /// Run longitudinal degradation test
    Longevity {
        /// System name
        #[arg(long, default_value = "echo")]
        system: String,
        /// Total sessions to ingest
        #[arg(long, default_value = "1000")]
        sessions: usize,
        /// Number of evaluation checkpoints
        #[arg(long, default_value = "10")]
        checkpoints: usize,
        /// Questions to evaluate at each checkpoint
        #[arg(long, default_value = "50")]
        eval_questions: usize,
        /// Model for generation
        #[arg(long)]
        gen_model: Option<String>,
        /// Model for judging
        #[arg(long)]
        judge_model: Option<String>,
        /// Output file
        #[arg(long)]
        output: Option<PathBuf>,
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

    /// Add or update a note on an existing run
    Annotate {
        /// Path to results file
        path: PathBuf,
        /// Note text to add
        #[arg(long)]
        note: String,
    },

    /// Test retrieval quality without LLM calls (zero cost, instant feedback)
    RetrievalTest {
        /// System name
        #[arg(long, default_value = "mindcore-api")]
        system: String,
        /// Dataset name
        #[arg(long, default_value = "longmemeval")]
        dataset: String,
        /// Dataset variant
        #[arg(long, default_value = "small")]
        variant: String,
        /// Token budget for context retrieval
        #[arg(long)]
        budget: Option<usize>,
        /// Filter by question types (comma-separated)
        #[arg(long)]
        filter: Option<String>,
        /// Quick mode: test a random subset
        #[arg(long)]
        quick: bool,
        /// Number of questions in quick mode
        #[arg(long, default_value = "20")]
        quick_size: Option<usize>,
        /// Show detailed per-question failures
        #[arg(long)]
        verbose: bool,
    },

    /// Launch local web UI to browse results
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "8888")]
        port: u16,
        /// Results directory to serve
        #[arg(long, default_value = "results")]
        results_dir: PathBuf,
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
            quick, quick_size, stress, budget_sweep, note,
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

            if let Some(n) = stress {
                cmd_stress(
                    system_name, &dataset, &variant, conc, bgt,
                    gen_m, jdg, &out, qsize, n, &cfg,
                ).await
            } else if budget_sweep {
                cmd_budget_sweep(
                    system_name, &dataset, &variant, conc,
                    gen_m, jdg, &out, qsize, &cfg,
                ).await
            } else {
                cmd_run(
                    system_name, system_config.as_deref(),
                    &dataset, &variant, conc, bgt,
                    gen_m, jdg, &out, filter_types, resume, qsize, note, &cfg,
                ).await
            }
        }
        Commands::Compare { systems, dataset, variant, output: _ } => {
            cmd_compare(&systems, &dataset, &variant, &cfg).await
        }
        Commands::Calibrate { judge_model, dataset: _ } => {
            cmd_calibrate(&judge_model, &cfg).await
        }
        Commands::Longevity {
            system, sessions, checkpoints, eval_questions,
            gen_model, judge_model, output,
        } => {
            let gen_m = gen_model.as_deref().unwrap_or(&cfg.defaults.gen_model);
            let jdg = judge_model.as_deref().unwrap_or(&cfg.defaults.judge_model);

            let mem_system: Box<dyn traits::MemorySystem> = match system.as_str() {
                "echo" => Box::new(systems::echo::EchoSystem::new()),
                #[cfg(feature = "mindcore")]
                "mindcore" => Box::new(recallbench_mindcore::MindCoreAdapter::new()?),
                _ => anyhow::bail!("Unknown system: {system}"),
            };

            let gen_llm: Arc<dyn traits::LLMClient> = Arc::new(
                llm::cli::CliLLMClient::new(&llm::LLMRegistry::resolve_provider(gen_m).0, gen_m),
            );
            let judge_llm: Arc<dyn traits::LLMClient> = Arc::new(
                llm::cli::CliLLMClient::new(&llm::LLMRegistry::resolve_provider(jdg).0, jdg),
            );

            let longevity_config = longevity::LongevityConfig {
                total_sessions: sessions,
                checkpoints,
                eval_questions,
                token_budget: cfg.defaults.token_budget,
            };

            let result = longevity::run_longevity(
                mem_system.as_ref(), gen_llm, judge_llm, &longevity_config,
            ).await?;

            println!("{}", longevity::render_longevity_table(&result));

            if let Some(out_path) = output {
                let json = serde_json::to_string_pretty(&result)?;
                std::fs::write(&out_path, json)?;
                println!("Results written to {}", out_path.display());
            }

            Ok(())
        }
        Commands::Annotate { path, note } => {
            let meta_path = path.with_extension("meta.json");
            let mut meta: serde_json::Value = if meta_path.exists() {
                let content = std::fs::read_to_string(&meta_path)?;
                serde_json::from_str(&content)?
            } else {
                serde_json::json!({})
            };
            meta["note"] = serde_json::Value::String(note.clone());
            std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
            println!("Note updated on {}", meta_path.display());
            Ok(())
        }
        Commands::RetrievalTest {
            system: system_name, dataset, variant, budget, filter,
            quick, quick_size, verbose,
        } => {
            cmd_retrieval_test(
                &system_name, &dataset, &variant, budget, filter.as_deref(),
                quick, quick_size, verbose,
            ).await
        }
        Commands::Serve { port, results_dir } => {
            web::serve(port, results_dir).await
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
    note: Option<String>,
    cfg: &config::Config,
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
            #[cfg(feature = "mindcore-adapter")]
            "mindcore" => Box::new(systems::mindcore_adapter::MindCoreAdapter::new()?),
            #[cfg(feature = "mindcore-adapter")]
            "mindcore-api" | "mindcore-fallback" => {
                let key_output = std::process::Command::new("sh")
                    .args(["-c", "security find-generic-password -w -s 'DeepInfra API Key' -a 'deepinfra'"])
                    .output()?;
                let api_key = String::from_utf8_lossy(&key_output.stdout).trim().to_string();
                if api_key.is_empty() {
                    anyhow::bail!("Failed to fetch DeepInfra API key from keychain");
                }
                if system_name == "mindcore-api" {
                    Box::new(systems::mindcore_adapter::MindCoreAdapter::with_deepinfra_api(&api_key)?)
                } else {
                    Box::new(systems::mindcore_adapter::MindCoreAdapter::with_api_and_local_fallback(&api_key)?)
                }
            },
            #[cfg(feature = "memloft-adapter")]
            "memloft" => Box::new(systems::memloft_adapter::MemloftAdapter::new()?),
            _ => anyhow::bail!("Unknown system: {system_name}. Use --system-config for custom systems."),
        }
    };

    // Create LLM clients (resolves custom/local endpoints from config)
    let gen_llm = create_llm_client(gen_model, &cfg)?;
    let judge_llm = create_llm_client(judge_model, &cfg)?;

    // Build or load embedding cache if system supports it
        let embedding_cache = if system.supports_precomputed() {
        let model_name = "sentence-transformers/all-MiniLM-L6-v2";
        if embedding_cache::EmbeddingCache::exists(&dataset, &variant, model_name) {
            tracing::info!("Using cached embeddings for {dataset}/{variant}");
            Some(embedding_cache::EmbeddingCache::open(&dataset, &variant, model_name)?)
        } else {
            tracing::info!("Building embedding cache for {dataset}/{variant} (one-time)...");
            // Get API key for DeepInfra embedding
            let key_output = std::process::Command::new("sh")
                .args(["-c", "security find-generic-password -w -s 'DeepInfra API Key' -a 'deepinfra'"])
                .output();
            let api_key = key_output.ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|k| !k.is_empty());

            if let Some(key) = api_key {
                let backend = mindcore::embeddings::ApiBackend::deepinfra_minilm(&key);
                let cache = embedding_cache::EmbeddingCache::build(
                    &dataset, &variant, ds.questions(), &backend, 1000, 10,
                )?;
                Some(cache)
            } else {
                tracing::warn!("No DeepInfra API key found, building cache with local model...");
                let backend = mindcore::embeddings::CandleNativeBackend::new()?;
                let cache = embedding_cache::EmbeddingCache::build(
                    &dataset, &variant, ds.questions(), &backend, 1000, 10,
                )?;
                Some(cache)
            }
        }
    } else {
        None
    };

    // Run benchmark
    let run_config = runner::RunConfig {
        concurrency,
        token_budget: budget,
        output_path: output.to_path_buf(),
        filter_types,
        resume: do_resume,
        quick_size,
        note,
                embedding_cache_path: embedding_cache.as_ref().map(|c| c.path().to_path_buf()),
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

/// Create an LLM client from a model string, resolving named endpoints from config.
fn create_llm_client(model: &str, cfg: &config::Config) -> Result<Arc<dyn traits::LLMClient>> {
    // Check for named endpoint in config (e.g., --gen-model deepinfra matches [llm.deepinfra])
    if let Some(ep) = cfg.llm.endpoints.get(model) {
        let client = llm::compatible::CompatibleClient::from_config(model, ep)?;
        return Ok(Arc::new(client));
    }

    // Default: use CLI adapter for known providers (claude, chatgpt, gemini, codex)
    match model {
        _ => {
            let (provider, _) = llm::LLMRegistry::resolve_provider(model);
            Ok(Arc::new(llm::cli::CliLLMClient::new(provider, model)))
        }
    }
}

async fn cmd_stress(
    system_name: &str,
    dataset: &str,
    variant: &str,
    concurrency: usize,
    budget: usize,
    gen_model: &str,
    judge_model: &str,
    output: &std::path::Path,
    quick_size: Option<usize>,
    runs: usize,
    cfg: &config::Config,
) -> Result<()> {
    println!("Stress test: {runs} runs of {dataset}/{variant} on {system_name}\n");

    let registry = datasets::DatasetRegistry::new();
    let ds = registry.load(dataset, variant, false).await?;

    let system: Box<dyn traits::MemorySystem> = match system_name {
        "echo" => Box::new(systems::echo::EchoSystem::new()),
        #[cfg(feature = "mindcore")]
        "mindcore" => Box::new(recallbench_mindcore::MindCoreAdapter::new()?),
        _ => anyhow::bail!("Unknown system: {system_name}"),
    };

    let gen_llm = create_llm_client(gen_model, cfg)?;
    let judge_llm = create_llm_client(judge_model, cfg)?;

    let mut accuracies = Vec::new();

    for i in 0..runs {
        let run_output = output.with_extension(format!("run{i}.jsonl"));
        let run_config = runner::RunConfig {
            concurrency,
            token_budget: budget,
            output_path: run_output,
            filter_types: None,
            resume: false,
            quick_size,
            note: None,
                        embedding_cache_path: None,
        };

        let results = runner::run_benchmark(
            system.as_ref(), ds.as_ref(),
            gen_llm.clone(), judge_llm.clone(), &run_config,
        ).await?;

        let acc = metrics::compute_accuracy(&results);
        println!("  Run {}: {:.1}% ({}/{})", i + 1, acc.overall * 100.0, acc.total_correct, acc.total_questions);
        accuracies.push(acc.overall);
    }

    let mean = accuracies.iter().sum::<f64>() / accuracies.len() as f64;
    let variance = accuracies.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / accuracies.len() as f64;
    let stddev = variance.sqrt();

    println!("\nStress Test Summary ({runs} runs):");
    println!("  Mean accuracy:  {:.1}%", mean * 100.0);
    println!("  Std deviation:  {:.2}%", stddev * 100.0);
    println!("  Variance:       {:.4}", variance);
    println!("  Min:            {:.1}%", accuracies.iter().cloned().fold(f64::INFINITY, f64::min) * 100.0);
    println!("  Max:            {:.1}%", accuracies.iter().cloned().fold(f64::NEG_INFINITY, f64::max) * 100.0);

    Ok(())
}

async fn cmd_budget_sweep(
    system_name: &str,
    dataset: &str,
    variant: &str,
    concurrency: usize,
    gen_model: &str,
    judge_model: &str,
    output: &std::path::Path,
    quick_size: Option<usize>,
    cfg: &config::Config,
) -> Result<()> {
    let budgets = [4096, 8192, 16384, 32768];
    println!("Budget sweep: {} budgets on {dataset}/{variant}\n", budgets.len());

    let registry = datasets::DatasetRegistry::new();
    let ds = registry.load(dataset, variant, false).await?;

    let system: Box<dyn traits::MemorySystem> = match system_name {
        "echo" => Box::new(systems::echo::EchoSystem::new()),
        #[cfg(feature = "mindcore")]
        "mindcore" => Box::new(recallbench_mindcore::MindCoreAdapter::new()?),
        _ => anyhow::bail!("Unknown system: {system_name}"),
    };

    let gen_llm = create_llm_client(gen_model, cfg)?;
    let judge_llm = create_llm_client(judge_model, cfg)?;

    let mut results_table = Vec::new();

    for budget in &budgets {
        let run_output = output.with_extension(format!("budget{budget}.jsonl"));
        let run_config = runner::RunConfig {
            concurrency,
            token_budget: *budget,
            output_path: run_output,
            filter_types: None,
            resume: false,
            quick_size,
            note: None,
                        embedding_cache_path: None,
        };

        let results = runner::run_benchmark(
            system.as_ref(), ds.as_ref(),
            gen_llm.clone(), judge_llm.clone(), &run_config,
        ).await?;

        let acc = metrics::compute_accuracy(&results);
        results_table.push((*budget, acc.overall, acc.total_correct, acc.total_questions));
    }

    println!("\nBudget Sweep Results:");
    println!("  {:<12} {:<12} {:<10}", "Budget", "Accuracy", "Correct");
    println!("  {}", "-".repeat(34));
    for (budget, accuracy, correct, total) in &results_table {
        println!("  {:<12} {:<12} {}/{}", budget, format!("{:.1}%", accuracy * 100.0), correct, total);
    }

    Ok(())
}

async fn cmd_calibrate(judge_model: &str, cfg: &config::Config) -> Result<()> {
    const CALIBRATION_JSON: &str = include_str!("../calibration/longmemeval_50.json");
    let pairs = judge::calibration::load_calibration_pairs(CALIBRATION_JSON)?;
    println!("Running calibration with {} pairs against {judge_model}...\n", pairs.len());

    let judge_llm = create_llm_client(judge_model, cfg)?;
    let result = judge::calibration::run_calibration(&pairs, judge_llm.as_ref()).await?;

    println!("Calibration Results:");
    println!("  Total pairs: {}", result.total);
    println!("  Correct:     {}", result.correct);
    println!("  Accuracy:    {:.1}%", result.accuracy * 100.0);

    if !result.mismatches.is_empty() {
        println!("\nMismatches:");
        for m in &result.mismatches {
            let dir = if m.expected { "expected YES got NO" } else { "expected NO got YES" };
            println!("  [{}] {} — {}", m.index, m.question, dir);
        }
    }

    if result.accuracy >= 0.9 {
        println!("\nCalibration PASSED (>= 90%)");
    } else {
        println!("\nCalibration FAILED (< 90%). Judge may produce unreliable results.");
    }

    Ok(())
}

async fn cmd_compare(
    systems_str: &str,
    dataset: &str,
    variant: &str,
    cfg: &config::Config,
) -> Result<()> {
    let system_names: Vec<&str> = systems_str.split(',').map(|s| s.trim()).collect();
    if system_names.is_empty() {
        anyhow::bail!("No systems specified. Use --systems system1,system2");
    }

    let registry = datasets::DatasetRegistry::new();
    let ds = registry.load(dataset, variant, false).await?;

    let gen_model = &cfg.defaults.gen_model;
    let judge_model = &cfg.defaults.judge_model;
    let gen_llm: Arc<dyn traits::LLMClient> = Arc::new(
        llm::cli::CliLLMClient::new(&llm::LLMRegistry::resolve_provider(gen_model).0, gen_model),
    );
    let judge_llm: Arc<dyn traits::LLMClient> = Arc::new(
        llm::cli::CliLLMClient::new(&llm::LLMRegistry::resolve_provider(judge_model).0, judge_model),
    );

    let mut all_acc: Vec<(String, metrics::AccuracyMetrics)> = Vec::new();
    let mut all_lat: Vec<(String, metrics::LatencyMetrics)> = Vec::new();
    let mut all_cost: Vec<(String, metrics::CostMetrics)> = Vec::new();

    for sys_name in &system_names {
        let system: Box<dyn traits::MemorySystem> = match *sys_name {
            "echo" => Box::new(systems::echo::EchoSystem::new()),
            #[cfg(feature = "mindcore-adapter")]
            "mindcore" => match systems::mindcore_adapter::MindCoreAdapter::new() {
                Ok(a) => Box::new(a),
                Err(e) => { tracing::error!("Failed to create MindCore adapter: {e}"); continue; }
            },
            #[cfg(feature = "mindcore-adapter")]
            "mindcore-api" | "mindcore-fallback" => {
                // Fetch DeepInfra API key from keychain
                let key_result = std::process::Command::new("sh")
                    .args(["-c", "security find-generic-password -w -s 'DeepInfra API Key' -a 'deepinfra'"])
                    .output();
                let api_key = match key_result {
                    Ok(output) if output.status.success() => {
                        String::from_utf8_lossy(&output.stdout).trim().to_string()
                    }
                    _ => {
                        tracing::error!("Failed to fetch DeepInfra API key from keychain");
                        continue;
                    }
                };
                let adapter = if *sys_name == "mindcore-api" {
                    systems::mindcore_adapter::MindCoreAdapter::with_deepinfra_api(&api_key)
                } else {
                    systems::mindcore_adapter::MindCoreAdapter::with_api_and_local_fallback(&api_key)
                };
                match adapter {
                    Ok(a) => Box::new(a),
                    Err(e) => { tracing::error!("Failed to create MindCore API adapter: {e}"); continue; }
                }
            },
            _ => {
                tracing::warn!("Unknown system '{sys_name}', skipping.");
                continue;
            }
        };

        let output_dir = PathBuf::from(&cfg.defaults.output_dir);
        std::fs::create_dir_all(&output_dir)?;
        let output_path = output_dir.join(format!("{sys_name}-{dataset}-{variant}.jsonl"));

        let run_config = runner::RunConfig {
            concurrency: cfg.defaults.concurrency,
            token_budget: cfg.defaults.token_budget,
            output_path,
            filter_types: None,
            resume: false,
            quick_size: None,
            note: None,
                        embedding_cache_path: None,
        };

        println!("\nBenchmarking {sys_name} ...");
        let results = runner::run_benchmark(
            system.as_ref(), ds.as_ref(),
            gen_llm.clone(), judge_llm.clone(), &run_config,
        ).await?;

        if !results.is_empty() {
            all_acc.push((sys_name.to_string(), metrics::compute_accuracy(&results)));
            all_lat.push((sys_name.to_string(), metrics::compute_latency(&results)));
            all_cost.push((sys_name.to_string(), metrics::compute_cost(&results, &metrics::Pricing::default())));
        }
    }

    if !all_acc.is_empty() {
        let acc_refs: Vec<(&str, &metrics::AccuracyMetrics)> = all_acc.iter().map(|(n, a)| (n.as_str(), a)).collect();
        let lat_refs: Vec<(&str, &metrics::LatencyMetrics)> = all_lat.iter().map(|(n, l)| (n.as_str(), l)).collect();
        let cost_refs: Vec<(&str, &metrics::CostMetrics)> = all_cost.iter().map(|(n, c)| (n.as_str(), c)).collect();

        println!("\n{}", report::table::render_accuracy_table(&acc_refs, dataset, variant));
        println!("{}", report::table::render_latency_table(&lat_refs));
        println!("{}", report::table::render_cost_table(&cost_refs));
    }

    Ok(())
}

async fn cmd_retrieval_test(
    system_name: &str,
    dataset: &str,
    variant: &str,
    budget: Option<usize>,
    filter: Option<&str>,
    quick: bool,
    quick_size: Option<usize>,
    verbose: bool,
) -> Result<()> {
    let cfg = config::Config::load_default()?;
    let token_budget = budget.unwrap_or(cfg.defaults.token_budget);

    // Load dataset
    let registry = datasets::DatasetRegistry::new();
    let ds = registry.load(dataset, variant, false).await?;

    // Filter questions
    let all_questions = ds.questions();
    let questions: Vec<&types::BenchmarkQuestion> = all_questions.iter()
        .filter(|q| {
            if let Some(f) = filter {
                f.split(',').any(|t| t.trim() == q.question_type)
            } else {
                true
            }
        })
        .filter(|q| {
            // Must have answer_session_ids
            q.metadata.contains_key("answer_session_ids")
        })
        .collect();

    let questions: Vec<&types::BenchmarkQuestion> = if quick {
        let size = quick_size.unwrap_or(20);
        // Simple random selection for quick mode
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut indexed: Vec<(u64, &types::BenchmarkQuestion)> = questions.iter().map(|q| {
            let mut h = DefaultHasher::new();
            q.id.hash(&mut h);
            (h.finish(), *q)
        }).collect();
        indexed.sort_by_key(|(h, _)| *h);
        indexed.truncate(size);
        indexed.into_iter().map(|(_, q)| q).collect()
    } else {
        questions
    };

    println!("Retrieval Test — {} {} ({} questions, budget: {})",
        dataset, variant, questions.len(), token_budget);
    println!("══════════════════════════════════════════════════════");

    // Create system
    let system: Box<dyn traits::MemorySystem> = match system_name {
        "echo" => Box::new(systems::echo::EchoSystem::new()),
        #[cfg(feature = "mindcore-adapter")]
        "mindcore" => Box::new(systems::mindcore_adapter::MindCoreAdapter::new()?),
        #[cfg(feature = "mindcore-adapter")]
        "mindcore-api" | "mindcore-fallback" => {
            let key_output = std::process::Command::new("sh")
                .args(["-c", "security find-generic-password -w -s 'DeepInfra API Key' -a 'deepinfra'"])
                .output()?;
            let api_key = String::from_utf8_lossy(&key_output.stdout).trim().to_string();
            if system_name == "mindcore-api" {
                Box::new(systems::mindcore_adapter::MindCoreAdapter::with_deepinfra_api(&api_key)?)
            } else {
                Box::new(systems::mindcore_adapter::MindCoreAdapter::with_api_and_local_fallback(&api_key)?)
            }
        },
        _ => anyhow::bail!("Unknown system: {system_name}"),
    };

    // Check for embedding cache
    let cache_path = {
        let model_name = "sentence-transformers/all-MiniLM-L6-v2";
        if embedding_cache::EmbeddingCache::exists(dataset, variant, model_name) {
            Some(embedding_cache::EmbeddingCache::cache_path(dataset, variant, model_name))
        } else {
            tracing::warn!("No embedding cache found — will use live embedding (slow)");
            None
        }
    };

    // Run retrieval test
    let pb = indicatif::ProgressBar::new(questions.len() as u64);
    pb.set_style(indicatif::ProgressStyle::default_bar()
        .template("{msg}\n{wide_bar:.cyan/dim} {pos}/{len} ({eta})")
        .unwrap());
    pb.set_message("Testing retrieval...");

    let mut results = Vec::new();
    for question in &questions {
        let result = retrieval_test::test_retrieval(
            system.as_ref(),
            question,
            token_budget,
            cache_path.as_deref(),
        ).await?;
        results.push(result);
        pb.inc(1);
    }
    pb.finish_with_message("Done");

    // Compute and display metrics
    let metrics = retrieval_test::RetrievalMetrics::compute(&results);
    println!();
    metrics.print_report();

    if verbose {
        retrieval_test::print_failures(&results);
    }

    // Always show summary of worst questions
    let mut worst: Vec<_> = results.iter()
        .filter(|r| !r.answer_session_ids.is_empty())
        .collect();
    worst.sort_by(|a, b| {
        let a_found = a.found_at(1000) as f64 / a.answer_session_ids.len().max(1) as f64;
        let b_found = b.found_at(1000) as f64 / b.answer_session_ids.len().max(1) as f64;
        a_found.partial_cmp(&b_found).unwrap()
    });

    println!("\nWorst 5 questions (fewest answer sessions found):");
    for r in worst.iter().take(5) {
        let found = r.found_at(1000);
        let total = r.answer_session_ids.len();
        let missing = r.missing_sessions();
        println!("  {} [{}]: {}/{} sessions, missing: {:?}",
            &r.question_id[..r.question_id.len().min(12)],
            &r.question_type[..r.question_type.len().min(15)],
            found, total, missing);
    }

    Ok(())
}
