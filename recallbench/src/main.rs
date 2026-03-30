#![allow(dead_code)] // Library code is used through CLI subcommands, not all paths exercised in every build

mod checkpoint;
mod config;
mod datasets;
#[cfg(feature = "femind-adapter")]
mod embedding_cache;
mod errors;
mod extraction_test;
mod judge;
mod pipeline_test;
mod llm;
mod longevity;
mod metrics;
mod report;
mod resume;
mod retrieval_test;
mod runner;
mod sampling;
mod session_cache;
mod systems;
mod traits;
mod types;
mod verify;
mod web;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;

struct LocalTimestamp;

impl FormatTime for LocalTimestamp {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("%Y-%m-%dT%H:%M:%S%.6f%:z"))
    }
}

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
        #[arg(long, default_value = "femind-api")]
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
        /// Chunk size for session chunking (default: 1000)
        #[arg(long, default_value = "1000")]
        chunk_size: usize,
    },

    /// Build raw session caches for datasets (no embedding, instant)
    CacheSessions {
        /// Dataset name (or "all" for LongMemEval + ConvoMem + MemoryAgentBench)
        #[arg(long, default_value = "all")]
        dataset: String,
    },

    /// Test LLM extraction quality without search (extraction cost only)
    ExtractionTest {
        /// Dataset name
        #[arg(long, default_value = "memoryagentbench")]
        dataset: String,
        /// Dataset variant
        #[arg(long, default_value = "conflict_resolution")]
        variant: String,
        /// LLM model for extraction (via DeepInfra API)
        #[arg(long, default_value = "")]
        model: String,
        /// Quick mode: test a subset
        #[arg(long)]
        quick: bool,
        /// Number of sessions in quick mode
        #[arg(long, default_value = "2")]
        quick_size: Option<usize>,
        /// Max chars per extraction call (split large sessions)
        #[arg(long, default_value = "8000")]
        max_chars: usize,
    },

    /// Test full pipeline: extraction → storage → graph → search (modular, each step toggleable)
    PipelineTest {
        /// System name (femind-extract for full pipeline)
        #[arg(long, default_value = "femind-extract")]
        system: String,
        /// Dataset name
        #[arg(long, default_value = "memoryagentbench")]
        dataset: String,
        /// Dataset variant
        #[arg(long, default_value = "conflict_resolution")]
        variant: String,
        /// Token budget
        #[arg(long, default_value = "16384")]
        budget: usize,
        /// Enable LLM extraction (requires --extract-model)
        #[arg(long)]
        extraction: bool,
        /// LLM model for extraction (DeepInfra model name or "haiku" for CLI)
        #[arg(long)]
        extract_model: Option<String>,
        /// Enable graph edge creation
        #[arg(long)]
        graph: bool,
        /// Enable vector embedding
        #[arg(long, default_value = "true")]
        embedding: bool,
        /// Enable deduplication
        #[arg(long, default_value = "true")]
        dedup: bool,
        /// Recency boost (0.0 = off)
        #[arg(long, default_value = "0.0")]
        recency: f32,
        /// Max chunks per session (0 = unlimited)
        #[arg(long, default_value = "1")]
        max_per_session: usize,
        /// Chunk size for non-extraction mode
        #[arg(long, default_value = "1000")]
        chunk_size: usize,
        /// Quick mode
        #[arg(long)]
        quick: bool,
        /// Quick mode question count
        #[arg(long, default_value = "10")]
        quick_size: Option<usize>,
        /// Show verbose output
        #[arg(long)]
        verbose: bool,
    },

    /// Prebuild persistent local corpora for the enhanced femind benchmark lane
    PrebuildCorpora {
        /// Dataset name
        #[arg(long, default_value = "longmemeval")]
        dataset: String,
        /// Dataset variant
        #[arg(long, default_value = "small")]
        variant: String,
        /// Quick mode: prebuild only a stratified subset
        #[arg(long)]
        quick: bool,
        /// Number of questions in quick mode
        #[arg(long, default_value = "10")]
        quick_size: Option<usize>,
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
        .with_timer(LocalTimestamp)
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
                #[cfg(feature = "femind-adapter")]
                "femind" => Box::new(systems::femind_adapter::FemindAdapter::new()?),
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
            quick, quick_size, verbose, chunk_size,
        } => {
            cmd_retrieval_test(
                &system_name, &dataset, &variant, budget, filter.as_deref(),
                quick, quick_size, verbose, chunk_size,
            ).await
        }
        Commands::ExtractionTest {
            dataset, variant, model, quick, quick_size, max_chars,
        } => {
            cmd_extraction_test(&dataset, &variant, &model, quick, quick_size, max_chars).await
        }
        Commands::PipelineTest {
            system: system_name, dataset, variant, budget, extraction,
            extract_model, graph, embedding, dedup, recency, max_per_session,
            chunk_size, quick, quick_size, verbose,
        } => {
            cmd_pipeline_test(
                &system_name, &dataset, &variant, budget, extraction,
                extract_model.as_deref(), graph, embedding, dedup, recency,
                max_per_session, chunk_size, quick, quick_size, verbose,
            ).await
        }
        Commands::PrebuildCorpora { dataset, variant, quick, quick_size } => {
            cmd_prebuild_corpora(&dataset, &variant, quick, quick_size).await
        }
        Commands::CacheSessions { dataset: ds_name } => {
            cmd_cache_sessions(&ds_name).await
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
            #[cfg(feature = "femind-adapter")]
            "femind" | "femind-remote" => Box::new(
                systems::femind_adapter::FemindAdapter::with_backend(
                    build_femind_chunk_backend(system_name, &cfg)?,
                )?,
            ),
            #[cfg(feature = "femind-adapter")]
            "femind-api" | "femind-fallback" | "femind-enhanced" => {
                let api_key = fetch_deepinfra_api_key()?;
                if system_name == "femind-api" {
                    Box::new(systems::femind_adapter::FemindAdapter::with_backend(
                        build_femind_chunk_backend(system_name, &cfg)?,
                    )?)
                } else if system_name == "femind-enhanced" {
                    Box::new(build_femind_enhanced(&api_key)?)
                } else {
                    Box::new(systems::femind_adapter::FemindAdapter::with_backend(
                        build_femind_chunk_backend(system_name, &cfg)?,
                    )?)
                }
            },
            #[cfg(feature = "femind-adapter")]
            "femind-mab" => {
                Box::new(
                    systems::femind_adapter::FemindAdapter::with_backend(
                        build_femind_chunk_backend("femind-api", &cfg)?
                    )?
                        .with_assembly_config(femind::context::AssemblyConfig::single_document())
                )
            },
            #[cfg(feature = "femind-adapter")]
            "femind-extract" => {
                let api_key = fetch_deepinfra_api_key()?;
                let llm = Box::new(femind::llm::ApiLlmCallback::new(
                    "https://api.deepinfra.com/v1/openai",
                    &api_key,
                    "",
                ));
                Box::new(
                    systems::femind_adapter::FemindAdapter::with_backend(
                        build_femind_chunk_backend("femind-api", &cfg)?
                    )?
                        .with_assembly_config(femind::context::AssemblyConfig::single_document())
                        .with_llm(llm)
                )
            },
            #[cfg(feature = "memloft-adapter")]
            "memloft" => Box::new(systems::memloft_adapter::MemloftAdapter::new()?),
            _ => anyhow::bail!("Unknown system: {system_name}. Use --system-config for custom systems."),
        }
    };

    // Create LLM clients (resolves custom/local endpoints from config)
    let gen_llm = create_llm_client(gen_model, &cfg)?;
    let judge_llm = create_llm_client(judge_model, &cfg)?;

    #[cfg(feature = "femind-adapter")]
    let femind_cache_backend = match system_name {
        "femind" | "femind-api" | "femind-fallback" | "femind-remote" => {
            Some(build_femind_chunk_backend(system_name, &cfg)?)
        }
        _ => None,
    };

    // Build or load embedding cache if system supports it
    // Skip cache for extraction-based systems (they do their own ingest)
    let embedding_cache = if system.supports_precomputed()
        && system_name != "femind-extract"
        && system_name != "femind-enhanced"
    {
        #[cfg(feature = "femind-adapter")]
        if let Some(backend) = femind_cache_backend.as_ref() {
            let compatible_models = backend.compatibility_model_names();
            if let Some(cache) = embedding_cache::EmbeddingCache::open_compatible(
                &dataset,
                &variant,
                &compatible_models,
            )? {
                tracing::info!("Using cached embeddings for {dataset}/{variant}");
                Some(cache)
            } else {
                tracing::info!("Building embedding cache for {dataset}/{variant} (one-time)...");
                Some(embedding_cache::EmbeddingCache::build(
                    &dataset,
                    &variant,
                    ds.questions(),
                    backend.as_ref(),
                    1000,
                    10,
                )?)
            }
        } else {
            None
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
    if let Some(rest) = model.strip_prefix("codex:") {
        let (model_name, effort) = match rest.rsplit_once('@') {
            Some((name, eff)) if !name.is_empty() && !eff.is_empty() => (name, Some(eff)),
            _ => (rest, None),
        };
        return Ok(Arc::new(llm::codex::CodexClient::with_effort(model_name, effort)));
    }

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

#[cfg(feature = "femind-adapter")]
fn build_femind_enhanced(api_key: &str) -> Result<systems::femind_adapter::FemindAdapter> {
    let mut engine_config = femind::engine::EngineConfig::default();
    engine_config.assembly = femind::context::AssemblyConfig {
        max_per_session: 1,
        recency_boost: 0.3,
        search_limit: 200,
        graph_depth: 2,
    };
    let llm = Box::new(femind::llm::ApiLlmCallback::new(
        "https://api.deepinfra.com/v1/openai",
        api_key,
        "openai/gpt-oss-120b",
    ));
    systems::femind_adapter::FemindAdapter::with_backend_config_named(
        std::sync::Arc::new(femind::embeddings::ApiBackend::deepinfra_minilm(api_key)),
        engine_config,
        "femind-enhanced",
    )
    .map(|adapter| {
        adapter
            .with_assembly_config(femind::context::AssemblyConfig {
                max_per_session: 1,
                recency_boost: 0.3,
                search_limit: 200,
                graph_depth: 2,
            })
            .with_persistent_corpora()
            .with_ingest_mode(systems::femind_adapter::IngestMode::Hybrid)
            .with_llm_tagged(llm, "openai/gpt-oss-120b")
    })
}

#[cfg(feature = "femind-adapter")]
#[derive(Debug, Clone)]
struct FemindRemoteEmbeddingConfig {
    base_url: String,
    auth_token: Option<String>,
    timeout_secs: Option<u64>,
    fallback_to_local: bool,
}

#[cfg(feature = "femind-adapter")]
fn fetch_deepinfra_api_key() -> Result<String> {
    let key_output = std::process::Command::new("sh")
        .args(["-c", "security find-generic-password -w -s 'DeepInfra API Key' -a 'deepinfra'"])
        .output()?;
    let api_key = String::from_utf8_lossy(&key_output.stdout).trim().to_string();
    if api_key.is_empty() {
        anyhow::bail!("Failed to fetch DeepInfra API key from keychain");
    }
    Ok(api_key)
}

#[cfg(feature = "femind-adapter")]
fn resolve_secret_from_env_or_file(
    env_name: Option<&str>,
    env_file: Option<&std::path::Path>,
) -> Result<Option<String>> {
    if let Some(env_name) = env_name
        && let Ok(value) = std::env::var(env_name)
        && !value.trim().is_empty()
    {
        return Ok(Some(value));
    }
    if let (Some(env_name), Some(env_file)) = (env_name, env_file) {
        let text = std::fs::read_to_string(env_file)?;
        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let line = line.strip_prefix("export ").unwrap_or(line);
            let Some((candidate_key, candidate_value)) = line.split_once('=') else {
                continue;
            };
            if candidate_key.trim() == env_name {
                let value = candidate_value.trim().trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    return Ok(Some(value.to_string()));
                }
            }
        }
        anyhow::bail!(
            "env file '{}' does not define {}",
            env_file.display(),
            env_name
        );
    }
    Ok(None)
}

#[cfg(feature = "femind-adapter")]
fn resolve_femind_remote_embedding_config(
    cfg: &config::Config,
) -> Result<Option<FemindRemoteEmbeddingConfig>> {
    let Some(remote) = cfg.embeddings.remote_service.as_ref() else {
        return Ok(None);
    };
    if remote.enabled == Some(false) {
        return Ok(None);
    }
    let Some(base_url) = remote.base_url.clone() else {
        return Ok(None);
    };
    let auth_token = resolve_secret_from_env_or_file(
        remote.auth_token_env.as_deref(),
        remote.auth_token_env_file.as_deref(),
    )?;
    Ok(Some(FemindRemoteEmbeddingConfig {
        base_url,
        auth_token,
        timeout_secs: remote.timeout_secs,
        fallback_to_local: remote.fallback_to_local.unwrap_or(true),
    }))
}

#[cfg(feature = "femind-adapter")]
fn build_femind_chunk_backend(
    system_name: &str,
    cfg: &config::Config,
) -> Result<std::sync::Arc<dyn femind::embeddings::EmbeddingBackend>> {
    use std::time::Duration;

    use femind::embeddings::{
        ApiBackend, CandleNativeBackend, EmbeddingBackend, FallbackBackend, RemoteEmbeddingBackend,
    };

    match system_name {
        "femind" => Ok(std::sync::Arc::new(CandleNativeBackend::new()?)),
        "femind-api" => {
            let api_key = fetch_deepinfra_api_key()?;
            Ok(std::sync::Arc::new(ApiBackend::deepinfra_minilm(&api_key)))
        }
        "femind-fallback" => {
            let api_key = fetch_deepinfra_api_key()?;
            let api = Box::new(ApiBackend::deepinfra_minilm(&api_key)) as Box<dyn EmbeddingBackend>;
            let local = Box::new(CandleNativeBackend::new()?) as Box<dyn EmbeddingBackend>;
            Ok(std::sync::Arc::new(FallbackBackend::api_with_local_fallback(api, local)))
        }
        "femind-remote" => {
            let remote = resolve_femind_remote_embedding_config(cfg)?
                .ok_or_else(|| anyhow::anyhow!("missing [embeddings.remote_service] config for femind-remote"))?;
            let timeout = remote.timeout_secs.map(Duration::from_secs);
            if remote.fallback_to_local {
                Ok(std::sync::Arc::new(
                    RemoteEmbeddingBackend::minilm_with_local_fallback_and_timeout(
                        remote.base_url,
                        remote.auth_token,
                        Box::new(CandleNativeBackend::new()?),
                        timeout,
                    )?,
                ))
            } else {
                Ok(std::sync::Arc::new(RemoteEmbeddingBackend::minilm_with_timeout(
                    remote.base_url,
                    remote.auth_token,
                    timeout,
                )?))
            }
        }
        other => anyhow::bail!("unsupported femind embedding backend for system '{other}'"),
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
        #[cfg(feature = "femind-adapter")]
        "femind" => Box::new(systems::femind_adapter::FemindAdapter::new()?),
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
        #[cfg(feature = "femind-adapter")]
        "femind" => Box::new(systems::femind_adapter::FemindAdapter::new()?),
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
            #[cfg(feature = "femind-adapter")]
            "femind" | "femind-remote" => match systems::femind_adapter::FemindAdapter::with_backend(
                match build_femind_chunk_backend(sys_name, &cfg) {
                    Ok(backend) => backend,
                    Err(e) => {
                        tracing::error!("Failed to create Femind embedding backend: {e}");
                        continue;
                    }
                }
            ) {
                Ok(a) => Box::new(a),
                Err(e) => { tracing::error!("Failed to create Femind adapter: {e}"); continue; }
            },
            #[cfg(feature = "femind-adapter")]
            "femind-api" | "femind-fallback" => {
                let adapter = systems::femind_adapter::FemindAdapter::with_backend(
                    match build_femind_chunk_backend(sys_name, &cfg) {
                        Ok(backend) => backend,
                        Err(e) => {
                            tracing::error!("Failed to create Femind embedding backend: {e}");
                            continue;
                        }
                    }
                );
                match adapter {
                    Ok(a) => Box::new(a),
                    Err(e) => { tracing::error!("Failed to create Femind API adapter: {e}"); continue; }
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
    chunk_size: usize,
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

    println!("Retrieval Test — {} {} ({} questions, budget: {}, chunk_size: {})",
        dataset, variant, questions.len(), token_budget, chunk_size);
    println!("══════════════════════════════════════════════════════");

    // Create system
    let system: Box<dyn traits::MemorySystem> = match system_name {
        "echo" => Box::new(systems::echo::EchoSystem::new()),
        #[cfg(feature = "femind-adapter")]
        "femind" | "femind-remote" => Box::new(
            systems::femind_adapter::FemindAdapter::with_backend(
                build_femind_chunk_backend(system_name, &cfg)?
            )?
        ),
        #[cfg(feature = "femind-adapter")]
        "femind-api" | "femind-fallback" | "femind-mab" => {
            if system_name == "femind-mab" {
                Box::new(
                    systems::femind_adapter::FemindAdapter::with_backend(
                        build_femind_chunk_backend("femind-api", &cfg)?
                    )?
                        .with_assembly_config(femind::context::AssemblyConfig::single_document())
                )
            } else {
                Box::new(
                    systems::femind_adapter::FemindAdapter::with_backend(
                        build_femind_chunk_backend(system_name, &cfg)?
                    )?
                )
            }
        },
        _ => anyhow::bail!("Unknown system: {system_name}"),
    };

    // Check for embedding cache matching the chunk size
    let cache_path = {
        let backend = build_femind_chunk_backend(system_name, &cfg)?;
        let compatible_models = backend.compatibility_model_names();
        if let Some(cache) = embedding_cache::EmbeddingCache::open_compatible_with_chunk_size(
            dataset,
            variant,
            &compatible_models,
            chunk_size,
        )? {
            tracing::info!("Using cached embeddings (chunk_size={chunk_size})");
            Some(cache.path().to_path_buf())
        } else {
            tracing::warn!("No embedding cache found for chunk_size={chunk_size} — will use live embedding");
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

    // Build the run record with config + results
    let run = retrieval_test::RetrievalRun {
        config: retrieval_test::RetrievalRunConfig {
            system: system_name.to_string(),
            dataset: dataset.to_string(),
            variant: variant.to_string(),
            chunk_size,
            token_budget,
            total_questions: results.len(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            note: None,
        },
        results,
    };

    // Save results to file
    let results_dir = std::path::PathBuf::from("results/retrieval");
    std::fs::create_dir_all(&results_dir)?;
    let filename = format!("{}-{}-c{}-b{}.json",
        dataset, variant, chunk_size, token_budget);
    let save_path = results_dir.join(&filename);
    run.save(&save_path)?;

    // Print full report with per-type breakdown
    println!();
    run.print_full_report();

    if verbose {
        retrieval_test::print_failures(&run.results);
    }

    // Show worst questions
    let mut worst: Vec<_> = run.results.iter()
        .filter(|r| !r.answer_session_ids.is_empty() && !r.is_abstention)
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

async fn cmd_cache_sessions(dataset_filter: &str) -> Result<()> {
    let registry = datasets::DatasetRegistry::new();

    // Define the priority datasets and their variants
    let targets: Vec<(&str, Vec<&str>)> = match dataset_filter {
        "all" => vec![
            ("longmemeval", vec!["small"]),
            ("convomem", vec!["user_evidence", "assistant_facts", "changing", "abstention", "preference", "implicit_connection"]),
            ("memoryagentbench", vec!["conflict_resolution", "accurate_retrieval", "long_range", "test_time_learning"]),
        ],
        other => {
            // Load default variant
            if let Some(info) = registry.get(other) {
                vec![(other, info.variants.iter().map(|s| s.as_str()).collect())]
            } else {
                anyhow::bail!("Unknown dataset: {other}");
            }
        }
    };

    for (dataset, variants) in &targets {
        for variant in variants {
            if session_cache::SessionCache::exists(dataset, variant) {
                let cache = session_cache::SessionCache::open(dataset, variant)?;
                let stats = cache.stats()?;
                println!("✓ {stats}");
                continue;
            }

            println!("Downloading {dataset}/{variant}...");
            match registry.load(dataset, variant, false).await {
                Ok(ds) => {
                    let cache = session_cache::SessionCache::build(
                        dataset, variant, ds.questions(),
                    )?;
                    let stats = cache.stats()?;
                    println!("✓ {stats}");
                }
                Err(e) => {
                    tracing::error!("Failed to load {dataset}/{variant}: {e}");
                    println!("✗ {dataset}/{variant}: {e}");
                }
            }
        }
    }

    // Summary
    println!("\nSession caches:");
    let cache_dir = session_cache::SessionCache::cache_path("_", "_")
        .parent().unwrap().to_path_buf();
    if cache_dir.exists() {
        for entry in std::fs::read_dir(&cache_dir)? {
            let entry = entry?;
            let size = entry.metadata()?.len();
            println!("  {} ({:.1}MB)", entry.file_name().to_string_lossy(), size as f64 / 1024.0 / 1024.0);
        }
    }

    Ok(())
}

#[cfg(feature = "femind-adapter")]
async fn cmd_prebuild_corpora(
    dataset: &str,
    variant: &str,
    quick: bool,
    quick_size: Option<usize>,
) -> Result<()> {
    let key_output = std::process::Command::new("sh")
        .args(["-c", "security find-generic-password -w -s 'DeepInfra API Key' -a 'deepinfra'"])
        .output()?;
    let api_key = String::from_utf8_lossy(&key_output.stdout).trim().to_string();
    if api_key.is_empty() {
        anyhow::bail!("Failed to fetch DeepInfra API key from keychain");
    }

    let system: Box<dyn traits::MemorySystem> = Box::new(build_femind_enhanced(&api_key)?);

    let registry = datasets::DatasetRegistry::new();
    let ds = registry.load(dataset, variant, false).await?;
    let questions: Vec<&types::BenchmarkQuestion> = if quick {
        sampling::stratified_sample(ds.questions(), quick_size.unwrap_or(10), 42)
    } else {
        ds.questions().iter().collect()
    };

    println!(
        "Prebuilding femind-enhanced corpora — {} {} ({} questions)",
        dataset,
        variant,
        questions.len()
    );
    println!("══════════════════════════════════════════════════════");

    let pb = indicatif::ProgressBar::new(questions.len() as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg}\n{wide_bar:.cyan/dim} {pos}/{len} ({eta})")
            .unwrap(),
    );
    pb.set_message("Preparing persistent corpora...");

    for question in &questions {
        system.prepare_question(dataset, variant, question).await?;
        system.reset().await?;
        pb.inc(1);
    }
    pb.finish_with_message("Persistent corpora ready");

    println!(
        "Stored corpora under {}",
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("recallbench")
            .join("femind-corpora")
            .display()
    );

    Ok(())
}

#[cfg(not(feature = "femind-adapter"))]
async fn cmd_prebuild_corpora(
    _dataset: &str,
    _variant: &str,
    _quick: bool,
    _quick_size: Option<usize>,
) -> Result<()> {
    anyhow::bail!("femind adapter support is not enabled in this build")
}

async fn cmd_extraction_test(
    dataset: &str,
    variant: &str,
    model: &str,
    quick: bool,
    quick_size: Option<usize>,
    max_chars: usize,
) -> Result<()> {
    // Get API key for DeepInfra
    let key_output = std::process::Command::new("sh")
        .args(["-c", "security find-generic-password -w -s 'DeepInfra API Key' -a 'deepinfra'"])
        .output()?;
    let api_key = String::from_utf8_lossy(&key_output.stdout).trim().to_string();
    if api_key.is_empty() {
        anyhow::bail!("Failed to fetch DeepInfra API key from keychain");
    }

    let llm = femind::llm::ApiLlmCallback::new(
        "https://api.deepinfra.com/v1/openai",
        &api_key,
        model,
    );

    println!("Extraction Test — {} {} (model: {})", dataset, variant, model);
    println!("══════════════════════════════════════════════════════");

    // Load sessions from session cache
    if !session_cache::SessionCache::exists(dataset, variant) {
        anyhow::bail!("No session cache for {dataset}/{variant}. Run: recallbench cache-sessions");
    }
    let cache = session_cache::SessionCache::open(dataset, variant)?;
    let stats = cache.stats()?;
    println!("Sessions: {}, Turns: {}", stats.total_sessions, stats.total_turns);

    // Load all session IDs
    let conn = rusqlite::Connection::open(cache.path())?;
    let mut stmt = conn.prepare("SELECT session_id, total_chars FROM sessions ORDER BY total_chars ASC")?;
    let sessions: Vec<(String, usize)> = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
    })?.filter_map(|r| r.ok()).collect();

    let sessions_to_test = if quick {
        let n = quick_size.unwrap_or(2);
        sessions.into_iter().take(n).collect::<Vec<_>>()
    } else {
        sessions
    };

    println!("Testing {} sessions...\n", sessions_to_test.len());

    let mut all_results = Vec::new();

    for (session_id, total_chars) in &sessions_to_test {
        let loaded = cache.load_sessions(&[session_id.as_str()])?;
        if loaded.is_empty() {
            continue;
        }

        let full_text: String = loaded[0].turns.iter()
            .map(|t| format!("{}: {}", t.role, t.content))
            .collect::<Vec<_>>()
            .join("\n");

        // Split into chunks if too large
        let chunks: Vec<String> = if full_text.len() > max_chars {
            let mut result = Vec::new();
            let mut remaining = full_text.as_str();
            while !remaining.is_empty() {
                let split_at = if remaining.len() <= max_chars {
                    remaining.len()
                } else {
                    remaining[..max_chars].rfind('\n')
                        .map(|p| p + 1)
                        .unwrap_or(max_chars)
                };
                result.push(remaining[..split_at].to_string());
                remaining = &remaining[split_at..];
            }
            result
        } else {
            vec![full_text]
        };

        println!("  Session {} ({} chars, {} extraction chunks)", session_id, total_chars, chunks.len());

        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_id = format!("{}_{}", session_id, i);
            match extraction_test::test_extraction(&chunk_id, chunk, &llm) {
                Ok(result) => {
                    println!("    Chunk {}: {} facts, {} entities, {} rels ({}ms)",
                        i, result.facts_extracted, result.entities_found.len(),
                        result.relationships_found.len(), result.extraction_ms);
                    all_results.push(result);
                }
                Err(e) => {
                    tracing::error!("Extraction failed for {}: {e}", chunk_id);
                }
            }
        }
    }

    // Compute and display metrics
    let metrics = extraction_test::ExtractionMetrics::compute(&all_results);
    println!();
    metrics.print_report();

    // Save results
    let results_dir = std::path::PathBuf::from("results/extraction");
    std::fs::create_dir_all(&results_dir)?;
    let filename = format!("{}-{}.json", dataset, variant);
    extraction_test::save_results(&all_results, &results_dir.join(&filename))?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn cmd_pipeline_test(
    _system_name: &str,
    dataset: &str,
    variant: &str,
    budget: usize,
    extraction: bool,
    extract_model: Option<&str>,
    graph: bool,
    embedding: bool,
    dedup: bool,
    recency: f32,
    max_per_session: usize,
    chunk_size: usize,
    quick: bool,
    quick_size: Option<usize>,
    verbose: bool,
) -> Result<()> {
    fn session_fingerprint(question: &types::BenchmarkQuestion) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for session in &question.sessions {
            session.id.hash(&mut hasher);
            session.date.hash(&mut hasher);
            for turn in &session.turns {
                turn.role.hash(&mut hasher);
                turn.content.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    // Build the pipeline config
    let config = pipeline_test::PipelineConfig {
        dataset: dataset.to_string(),
        variant: variant.to_string(),
        use_extraction: extraction,
        use_graph: graph,
        use_embedding: embedding,
        recency_weight: recency,
        max_per_session,
        token_budget: budget,
        llm_model: extract_model.unwrap_or("").to_string(),
    };

    println!("Pipeline Test — {} {}", dataset, variant);
    println!("══════════════════════════════════════════════════════");
    println!("  Extraction:      {}", if extraction { format!("ON (model: {})", config.llm_model) } else { "OFF (chunk mode)".to_string() });
    println!("  Graph:           {}", if graph { "ON" } else { "OFF" });
    println!("  Embedding:       {}", if embedding { "ON" } else { "OFF (FTS5 only)" });
    println!("  Dedup:           {}", if dedup { "ON" } else { "OFF" });
    println!("  Recency:         {}", if recency > 0.0 { format!("{recency}") } else { "OFF".to_string() });
    println!("  Max/session:     {}", if max_per_session == 0 { "unlimited".to_string() } else { max_per_session.to_string() });
    println!("  Token budget:    {budget}");
    if !extraction {
        println!("  Chunk size:      {chunk_size}");
    }
    println!();

    // Validate: extraction requires a model
    if extraction && config.llm_model.is_empty() {
        anyhow::bail!("Extraction enabled but no --extract-model specified. Use --extract-model <model> or --extract-model haiku");
    }

    // Get API key
    let key_output = std::process::Command::new("sh")
        .args(["-c", "security find-generic-password -w -s 'DeepInfra API Key' -a 'deepinfra'"])
        .output()?;
    let api_key = String::from_utf8_lossy(&key_output.stdout).trim().to_string();

    // Build the Femind adapter with configured features
    let mut adapter = systems::femind_adapter::FemindAdapter::with_deepinfra_api(&api_key)?
        .with_assembly_config(femind::context::AssemblyConfig {
            max_per_session,
            recency_boost: recency,
            search_limit: 200,
            graph_depth: if graph { 2 } else { 0 },
        });

    // Set engine config toggles
    {
        let engine = adapter.engine_mut();
        engine.config.embedding_enabled = embedding;
        engine.config.graph_enabled = graph;
        engine.config.dedup_enabled = dedup;
    }

    // Add LLM if extraction is enabled
    if extraction {
        let llm: Box<dyn femind::traits::LlmCallback> = if config.llm_model == "haiku" {
            Box::new(femind::llm::CliLlmCallback::claude("haiku"))
        } else {
            Box::new(femind::llm::ApiLlmCallback::new(
                "https://api.deepinfra.com/v1/openai",
                &api_key,
                &config.llm_model,
            ))
        };
        adapter = adapter.with_llm(llm);
    }

    let system: Box<dyn traits::MemorySystem> = Box::new(adapter);

    // Load dataset
    let registry = datasets::DatasetRegistry::new();
    let ds = registry.load(dataset, variant, false).await?;

    let all_questions = ds.questions();
    let mut questions: Vec<&types::BenchmarkQuestion> = if quick {
        let n = quick_size.unwrap_or(10);
        sampling::stratified_sample(all_questions, n, 42)
    } else {
        all_questions.iter().collect()
    };
    questions.sort_by_key(|q| session_fingerprint(q));

    println!("Testing {} questions...\n", questions.len());

    // Run each question through the pipeline
    let mut results = Vec::new();
    let mut active_fingerprint: Option<u64> = None;

    for (index, question) in questions.iter().enumerate() {
        let session_chars: usize = question.sessions.iter()
            .flat_map(|s| s.turns.iter())
            .map(|t| t.content.len())
            .sum();
        let fingerprint = session_fingerprint(question);
        let reuse_ingest = active_fingerprint == Some(fingerprint);
        println!(
            "[{}/{}] {} [{}] sessions={} chars={} reuse_ingest={}",
            index + 1,
            questions.len(),
            &question.id[..question.id.len().min(24)],
            &question.question_type[..question.question_type.len().min(24)],
            question.sessions.len(),
            session_chars,
            if reuse_ingest { "yes" } else { "no" },
        );

        // Ingest
        let ingest_ms = if reuse_ingest {
            0
        } else {
            system.reset().await?;
            let ingest_start = std::time::Instant::now();
            for session in &question.sessions {
                system.ingest_session(session).await?;
            }
            active_fingerprint = Some(fingerprint);
            ingest_start.elapsed().as_millis() as u64
        };

        // Retrieve
        let retrieval_start = std::time::Instant::now();
        let retrieval = system.retrieve_context(
            &question.question,
            question.question_date.as_deref(),
            budget,
        ).await?;
        let retrieval_ms = retrieval_start.elapsed().as_millis() as u64;

        // Check if answer is in retrieved context
        let gt = question.ground_truth.join(", ");
        let answer_in_context = gt.split_whitespace()
            .filter(|w| w.len() > 3)
            .any(|word| retrieval.context.to_lowercase().contains(&word.to_lowercase()));

        let qr = pipeline_test::QuestionResult {
            question_id: question.id.clone(),
            question_text: question.question.clone(),
            ground_truth: gt.clone(),
            answer_in_context,
            tokens_used: retrieval.tokens_used,
            retrieval_ms,
        };

        let status = if answer_in_context { "✓" } else { "✗" };
        println!(
            "  {} tokens={} retrieval_ms={} ingest_ms={}",
            status,
            retrieval.tokens_used,
            retrieval_ms,
            ingest_ms,
        );
        if verbose {
            println!(
                "  question: {}",
                question.question
            );
        }

        results.push(qr);
    }

    // Compute stats
    let total = results.len();
    let found = results.iter().filter(|r| r.answer_in_context).count();
    let avg_tokens: usize = if total > 0 { results.iter().map(|r| r.tokens_used).sum::<usize>() / total } else { 0 };
    let avg_ms: u64 = if total > 0 { results.iter().map(|r| r.retrieval_ms).sum::<u64>() / total as u64 } else { 0 };

    let pipeline_result = pipeline_test::PipelineResult {
        config: config.clone(),
        extraction_stats: pipeline_test::ExtractionStats {
            total_facts: 0, // TODO: track from extraction
            total_entities: 0,
            total_relationships: 0,
            graph_edges: 0,
            extraction_ms: 0,
        },
        retrieval_stats: pipeline_test::RetrievalStats {
            total_questions: total,
            answer_in_context: found,
            answer_accuracy: if total > 0 { found as f64 / total as f64 } else { 0.0 },
            avg_tokens_used: avg_tokens,
            avg_retrieval_ms: avg_ms,
        },
        per_question: results,
    };

    println!();
    pipeline_result.print_report();

    // Save results
    let results_dir = std::path::PathBuf::from("results/pipeline");
    std::fs::create_dir_all(&results_dir)?;
    let filename = format!("{}-{}-{}.json", dataset, variant,
        chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    pipeline_result.save(&results_dir.join(&filename))?;

    Ok(())
}
