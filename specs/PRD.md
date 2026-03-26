# RecallBench — Universal AI Memory System Benchmark

**Version:** 2.0
**Date:** 2026-03-20
**Status:** Implemented and under active maintenance
**Source of truth:** `.docs/recallbench_spec.db`

> Note: this document remains the human-readable product and architecture spec. The repo-local documentation database is the authoritative maintainer source of truth, and the build-phase sections near the end are retained as implementation history rather than current open work.

---

## 1. Problem

There is no vendor-neutral, high-quality benchmark harness for AI memory systems. The landscape as of March 2026:

- **supermemoryai/memorybench** (TypeScript, 202 stars) is the only pluggable multi-system harness, but it's vendor-owned (Supermemory) and TypeScript-only
- **LongMemEval** and **LoCoMo** are gold-standard datasets but ship as academic Python notebooks, not reusable tools
- Every vendor (Mem0, Letta, OMEGA, Mastra, Cognee) runs their own evaluation scripts with different judges, prompts, and methodology — scores are not comparable
- No tool measures operational characteristics (latency, throughput, memory usage) alongside accuracy
- No Rust-based benchmark tool exists in this space
- No formal standard for memory system evaluation exists

---

## 2. Vision

`cargo install recallbench` gives any developer a single command to benchmark their memory system against established academic datasets. Memory system authors implement one Rust trait (or configure an HTTP/subprocess adapter), and RecallBench handles dataset management, concurrent evaluation, LLM judging, latency profiling, and comparative reporting.

RecallBench is vendor-neutral, open-source, and designed to become the community standard — the way `criterion` is for Rust microbenchmarks or `MLPerf` is for ML hardware.

---

## 3. Goals

1. **Multi-system**: Benchmark any memory system through a pluggable trait interface, generic HTTP adapter, or subprocess adapter
2. **Multi-dataset**: Support LongMemEval, LoCoMo, MemBench, MemoryAgentBench, HaluMem, and custom datasets
3. **Reproducible**: Deterministic evaluation with pinned judge prompts, versioned datasets, and seeded LLM calls
4. **Fast**: Concurrent evaluation with configurable parallelism — 500 questions in minutes, not hours
5. **Comparative**: Side-by-side results tables across systems on identical questions
6. **Comprehensive**: Measure accuracy, latency (p50/p99), token costs, and failure patterns
7. **Publishable**: Output in terminal tables, markdown, JSON, and CSV
8. **Production-grade**: Resume support, rate limiting, error handling, calibration, CI integration

---

## 4. Non-Goals

- RecallBench does not implement a memory system — it tests them
- RecallBench does not train or fine-tune models
- RecallBench is not an LLM benchmark (it benchmarks memory/retrieval, not generation quality)
- RecallBench does not host a web leaderboard (may add later)
- RecallBench is the benchmark application for Femind and other memory systems

---

## 5. Competitive Landscape

### Benchmark Tools (Harnesses)

| Tool | Language | Stars | Pluggable | Datasets | Vendor-Neutral | Latency Profiling |
|------|----------|-------|-----------|----------|----------------|-------------------|
| supermemoryai/memorybench | TypeScript | 202 | Yes | LoCoMo, LongMemEval, ConvoMem | No (Supermemory) | No |
| letta-ai/letta-evals | Python | 60 | Partial | Custom | No (Letta) | No |
| omega-memory/memorystress | Python | 2 | Yes | Custom (583 facts) | No (OMEGA) | No |
| **recallbench** | **Rust** | **—** | **Yes** | **6+ datasets** | **Yes** | **Yes** |

### Benchmark Datasets

| Dataset | Venue | Questions | Focus | HuggingFace |
|---------|-------|-----------|-------|-------------|
| LongMemEval | ICLR 2025 | 500 | 5 memory abilities, 7 question types | xiaowu0162/longmemeval-cleaned |
| LoCoMo | Snap Research | 10 convos | Long-context QA, event summary, dialog gen | snap-research/locomo |
| MemBench | ACL 2025 | Multi-aspect | Effectiveness, efficiency, capacity | — |
| MemoryAgentBench | ICLR 2026 | EventQA, FactConsolidation | Selective forgetting, ACT-R alignment | HUST-AI-HYZ |
| HaluMem | MemTensor | 3,500 Q | Memory hallucination detection | — |
| MemoryBench | arxiv | 11 datasets | 3 domains, 4 task formats, 2 languages | THUIR/MemoryBench |

### Known System Scores

| System | LongMemEval | LoCoMo | Source |
|--------|-------------|--------|--------|
| OMEGA Memory | 95.4% | — | Official leaderboard |
| Mastra OM | 94.87% | — | Self-reported |
| Backboard | — | 93.4% | Self-reported |
| EverMemOS | — | 92.3% | arxiv 2601.02163 |
| Hindsight (Vectorize) | 91.4% | — | arxiv 2512.12818 |
| Femind | 88.7% | — | historical femind run (2026-03-20) |
| Emergence AI | 86% | — | Self-reported |

---

## 6. Architecture

### 6.1 Project Structure

```
recallbench/
├── Cargo.toml                  # Workspace root
├── recallbench/                # Main binary + library crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # CLI entry point (clap)
│       ├── lib.rs              # Public API re-exports
│       ├── traits.rs           # MemorySystem, LLMClient, BenchmarkDataset traits
│       ├── types.rs            # Shared types: BenchmarkQuestion, Turn, Session, etc.
│       ├── config.rs           # recallbench.toml parsing and defaults
│       ├── runner.rs           # Benchmark orchestration engine
│       ├── concurrency.rs      # Worker pool, rate limiting, progress
│       ├── resume.rs           # JSONL checkpoint/resume logic
│       ├── checkpoint.rs       # Stage-level checkpoint/resume (per-stage state)
│       ├── longevity.rs        # Longitudinal degradation testing engine
│       ├── datasets/
│       │   ├── mod.rs          # Dataset registry + trait
│       │   ├── download.rs     # HuggingFace/URL fetcher with cache + progress
│       │   ├── longmemeval.rs  # LongMemEval parser (oracle, S, M variants)
│       │   ├── locomo.rs       # LoCoMo parser
│       │   ├── convomem.rs     # ConvoMem parser
│       │   ├── membench.rs     # MemBench parser
│       │   ├── mab.rs          # MemoryAgentBench parser
│       │   ├── halumem.rs      # HaluMem parser
│       │   └── custom.rs       # User-defined JSON dataset
│       ├── judge/
│       │   ├── mod.rs          # Judge orchestration
│       │   ├── prompts.rs      # Type-specific judge prompt templates
│       │   ├── dual.rs         # Dual-model judging with tiebreaker
│       │   └── calibration.rs  # Calibration suite (pre-scored pairs)
│       ├── llm/
│       │   ├── mod.rs          # LLMClient trait + provider registry
│       │   ├── anthropic.rs    # Claude API via direct HTTP (reqwest)
│       │   ├── openai.rs       # OpenAI API via direct HTTP
│       │   ├── cli.rs          # Claude CLI fallback (claude --print)
│       │   └── rate_limit.rs   # Per-provider token bucket rate limiter
│       ├── systems/
│       │   ├── mod.rs          # System registry
│       │   ├── http.rs         # Generic HTTP/REST adapter (TOML-configured)
│       │   ├── subprocess.rs   # Generic CLI subprocess adapter
│       │   └── echo.rs         # Echo adapter (returns input as context — for testing)
│       ├── metrics/
│       │   ├── mod.rs          # Metric aggregation
│       │   ├── accuracy.rs     # Per-type, task-averaged, overall, abstention
│       │   ├── latency.rs      # p50/p95/p99 per operation (ingest, search, context)
│       │   └── cost.rs         # Token usage and estimated dollar cost
│       ├── report/
│       │   ├── mod.rs          # Report generation dispatch
│       │   ├── table.rs        # Terminal table (comfy-table)
│       │   ├── markdown.rs     # GitHub-ready markdown
│       │   ├── json.rs         # Machine-readable JSON summary
│       │   ├── csv.rs          # Spreadsheet export
│       │   └── failure.rs      # Missed question analysis export
│       ├── web/
│       │   ├── mod.rs          # Web UI server (axum)
│       │   ├── routes.rs       # API routes: results, questions, systems, metrics
│       │   ├── static/         # Embedded static assets (HTML/CSS/JS)
│       │   │   ├── index.html  # Single-page app shell
│       │   │   ├── app.js      # Vanilla JS — tables, charts, drill-down
│       │   │   └── style.css   # Minimal styling
│       │   └── embed.rs        # rust-embed for bundling static assets into binary
│       └── errors.rs           # Error types
├── adapters/
│   ├── recallbench-femind/   # Femind native adapter
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── recallbench-mem0/       # Mem0 HTTP adapter
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── recallbench-omega/      # OMEGA HTTP adapter
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── recallbench-zep/        # Zep HTTP adapter
│       ├── Cargo.toml
│       └── src/lib.rs
├── calibration/
│   └── longmemeval_50.json     # Pre-scored calibration pairs
├── specs/
│   └── PRD.md
├── tests/
│   ├── integration/
│   │   ├── echo_system.rs      # End-to-end with echo adapter
│   │   ├── longmemeval.rs      # Dataset parsing validation
│   │   ├── judge.rs            # Judge prompt correctness
│   │   ├── resume.rs           # Checkpoint/resume correctness
│   │   └── longevity.rs        # Longitudinal degradation test
│   └── fixtures/
│       ├── sample_longmemeval.json
│       ├── sample_locomo.json
│       ├── sample_convomem.json
│       └── sample_custom.json
└── results/                    # .gitignored output directory
```

### 6.2 Core Traits

```rust
/// The primary trait any memory system must implement.
#[async_trait]
pub trait MemorySystem: Send + Sync {
    /// Human-readable name for reports.
    fn name(&self) -> &str;

    /// Version string for reproducibility.
    fn version(&self) -> &str;

    /// Reset all state between questions for isolation.
    async fn reset(&self) -> Result<()>;

    /// Ingest a conversation session into the memory system.
    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats>;

    /// Retrieve relevant context for a question within a token budget.
    async fn retrieve_context(
        &self,
        query: &str,
        query_date: Option<&str>,
        token_budget: usize,
    ) -> Result<RetrievalResult>;
}

/// Statistics returned from ingestion.
pub struct IngestStats {
    pub memories_stored: usize,
    pub duplicates_skipped: usize,
    pub duration_ms: u64,
}

/// Result of a retrieval operation.
pub struct RetrievalResult {
    pub context: String,
    pub items_retrieved: usize,
    pub tokens_used: usize,
    pub duration_ms: u64,
}

/// LLM provider abstraction for generation and judging.
#[async_trait]
pub trait LLMClient: Send + Sync {
    fn name(&self) -> &str;
    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String>;
    async fn generate_with_seed(&self, prompt: &str, max_tokens: usize, seed: u64) -> Result<String>;
}

/// A benchmark dataset that can be loaded and iterated.
pub trait BenchmarkDataset: Send + Sync {
    fn name(&self) -> &str;
    fn variant(&self) -> &str;
    fn questions(&self) -> &[BenchmarkQuestion];
    fn question_types(&self) -> Vec<String>;
    fn description(&self) -> &str;
}
```

### 6.3 Universal Types

```rust
pub struct BenchmarkQuestion {
    pub id: String,
    pub question_type: String,
    pub question: String,
    pub ground_truth: Vec<String>,
    pub question_date: Option<String>,
    pub sessions: Vec<ConversationSession>,
    pub is_abstention: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct ConversationSession {
    pub id: String,
    pub date: Option<String>,
    pub turns: Vec<Turn>,
}

pub struct Turn {
    pub role: String,
    pub content: String,
}

pub struct EvalResult {
    pub question_id: String,
    pub system_name: String,
    pub question_type: String,
    pub is_abstention: bool,
    pub hypothesis: String,
    pub ground_truth: String,
    pub is_correct: bool,
    pub ingest_latency_ms: u64,
    pub retrieval_latency_ms: u64,
    pub generation_latency_ms: u64,
    pub judge_latency_ms: u64,
    pub tokens_used: u32,
    pub tokens_generated: u32,
}
```

---

## 7. Evaluation Pipeline

For each system under test, for each question:

```
┌─────────┐    ┌─────────┐    ┌──────────┐    ┌──────────┐    ┌─────────┐
│  Reset   │───▶│ Ingest  │───▶│ Retrieve │───▶│ Generate │───▶│  Judge  │
│  System  │    │Sessions │    │ Context  │    │  Answer  │    │ Score   │
└─────────┘    └─────────┘    └──────────┘    └──────────┘    └─────────┘
     │              │               │               │               │
     ▼              ▼               ▼               ▼               ▼
  (clean)     IngestStats    RetrievalResult   Hypothesis      EvalResult
              latency_ms     latency_ms        latency_ms      (final)
```

Steps 1-3: **system-specific** (behind the `MemorySystem` trait)
Steps 4-5: **universal** (RecallBench handles LLM generation and judging)

---

## 8. Concurrency Model

```
                    ┌──────────────────────────┐
                    │     Question Queue        │
                    │  [q001, q002, ... q500]   │
                    └────────────┬─────────────┘
                                 │
                    ┌────────────┼─────────────┐
                    ▼            ▼              ▼
              ┌──────────┐ ┌──────────┐  ┌──────────┐
              │ Worker 1 │ │ Worker 2 │  │ Worker N │
              │ reset    │ │ reset    │  │ reset    │
              │ ingest   │ │ ingest   │  │ ingest   │
              │ retrieve │ │ retrieve │  │ retrieve │
              │ generate │ │ generate │  │ generate │
              │ judge    │ │ judge    │  │ judge    │
              └────┬─────┘ └────┬─────┘  └────┬─────┘
                   │            │              │
                   ▼            ▼              ▼
              ┌──────────────────────────────────┐
              │       Results Collector          │
              │  (append JSONL, update progress) │
              └──────────────────────────────────┘
```

- Each worker processes one question at a time with full isolation (reset between questions)
- Configurable concurrency: `--concurrency N` (default: 10)
- Per-provider rate limiter with token bucket algorithm and exponential backoff
- Progress bar with completed/total count and running accuracy updated every N questions
- Resume: load existing JSONL, skip completed question IDs

---

## 9. CLI Interface

```bash
# List available datasets and their variants
recallbench datasets

# Download a dataset
recallbench download longmemeval --variant oracle
recallbench download locomo

# Run benchmark — single system
recallbench run \
  --system femind \
  --dataset longmemeval \
  --variant oracle \
  --concurrency 10 \
  --budget 16384 \
  --judge-model claude-sonnet \
  --gen-model claude-sonnet \
  --output results/femind-oracle.jsonl \
  --seed 42

# Run benchmark — filter by question type
recallbench run \
  --system femind \
  --dataset longmemeval \
  --filter temporal-reasoning,knowledge-update \
  --output results/femind-temporal.jsonl

# Run benchmark — generic HTTP system
recallbench run \
  --system-config my-system.toml \
  --dataset longmemeval \
  --output results/custom.jsonl

# Compare multiple systems
recallbench compare \
  --systems femind,omega,mem0 \
  --dataset longmemeval \
  --variant oracle \
  --output results/comparison.json

# Generate report from results
recallbench report results/femind-oracle.jsonl
recallbench report results/femind-oracle.jsonl --format markdown
recallbench report results/femind-oracle.jsonl --format csv
recallbench report results/comparison.json --format table

# Failure analysis
recallbench failures results/femind-oracle.jsonl --export failures.json

# Show dataset statistics
recallbench stats longmemeval --variant oracle

# Validate a custom dataset
recallbench validate my-dataset.json

# Calibrate judge
recallbench calibrate --judge-model claude-sonnet --dataset longmemeval

# Resume an interrupted run
recallbench run --resume results/femind-oracle.jsonl [...]

# Longitudinal degradation test — simulate months of memory accumulation
recallbench longevity \
  --system femind \
  --sessions 1000 \
  --checkpoints 10 \
  --eval-questions 50 \
  --output results/femind-longevity.json

# Launch local web UI to browse results
recallbench serve --port 8888 --results-dir results/
```

---

## 10. Configuration

```toml
# recallbench.toml — project-level defaults

[defaults]
concurrency = 10
token_budget = 16384
gen_model = "claude-sonnet"
judge_model = "claude-sonnet"
output_dir = "results"
seed = 42
quick_size = 50  # Default subset size for --quick mode

[llm.anthropic]
# API key via ANTHROPIC_API_KEY env var or op read
mode = "cli"  # "cli" (default, subscription) or "api"
rate_limit_rpm = 60
rate_limit_tpm = 100000

[llm.openai]
# API key via OPENAI_API_KEY env var
mode = "cli"
rate_limit_rpm = 60
rate_limit_tpm = 100000

# OpenAI-compatible custom endpoint (DeepInfra, Together, Ollama, etc.)
[llm.custom]
base_url = "https://api.deepinfra.com/v1/openai"
api_key_env = "DEEPINFRA_API_KEY"
model = "meta-llama/Llama-3.1-70B-Instruct"
rate_limit_rpm = 120
rate_limit_tpm = 200000

# Local inference (Ollama, LM Studio, vLLM, llama.cpp)
[llm.local]
base_url = "http://localhost:11434/v1"
api_key_env = ""  # No key needed for local
model = "llama3.1:70b"
rate_limit_rpm = 0  # 0 = no limit
rate_limit_tpm = 0

# Generic HTTP system configuration
[system]
name = "my-memory-system"
version = "1.0"

[system.endpoints]
reset = { method = "POST", url = "http://localhost:8080/reset" }
ingest = { method = "POST", url = "http://localhost:8080/ingest" }
retrieve = { method = "POST", url = "http://localhost:8080/retrieve" }
```

### Quick Mode with Stratified Sampling

The `--quick` flag (alias `--dev`) evaluates a stratified random subset instead of the full dataset. This enables rapid iteration during memory system development.

- Default subset: 50 questions (configurable via `--quick-size N` or `quick_size` in config)
- **Stratified sampling**: proportional representation from each question type (e.g., if 20% of questions are temporal reasoning, ~10 of 50 will be temporal reasoning)
- Accepts `--seed` for reproducible subsets
- Report output notes it was a quick run with sample size and sampling method
- Combined with local/cheap models, enables the fastest possible iteration loop

```bash
# Quick run with defaults (50 questions, stratified)
recallbench run --system femind --dataset longmemeval --quick

# Custom quick size
recallbench run --system femind --dataset longmemeval --quick --quick-size 100

# Reproducible quick subset
recallbench run --system femind --dataset longmemeval --quick --seed 42

# Quick compare
recallbench compare --systems femind,omega --dataset longmemeval --quick
```

### Configurable OpenAI-Compatible Endpoints

Any service using the OpenAI Chat Completions API format can be used as a generation or judge model. One implementation handles all OpenAI-compatible services:

- **Local inference**: Ollama, LM Studio, vLLM, llama.cpp (`http://localhost:11434/v1`)
- **Third-party**: DeepInfra, Together, Fireworks, Groq, Replicate
- Selectable via `--gen-model custom` or `--gen-model local` (matching TOML section names)
- Rate limiter applies per-endpoint (set to 0 for no limit on local)
- API key optional (local servers typically need none)

```bash
# Use local Ollama for free iteration
recallbench run --system femind --dataset longmemeval --quick \
  --gen-model local --judge-model local

# Use DeepInfra for cheap cloud iteration
recallbench run --system femind --dataset longmemeval \
  --gen-model custom --judge-model custom

# Mix providers: cheap generation, quality judging
recallbench run --system femind --dataset longmemeval --quick \
  --gen-model local --judge-model claude-sonnet
```

---

## 11. Output Formats

### Terminal Table
```
RecallBench v1.0.0 — LongMemEval Oracle — 2026-03-20
══════════════════════════════════════════════════════════════════════

Accuracy
────────────────────────────────────────────────────────────────────
System           Task-Avg  Overall  IE     MR     KU     TR     ABS
Femind 0.1     88.7%     88.7%    90.2%  81.0%  91.0%  94.0%  85.0%
OMEGA Memory     95.4%     95.2%    96.1%  93.5%  95.8%  94.1%  97.0%

Latency (ms) — p50 / p95 / p99
────────────────────────────────────────────────────────────────────
System           Ingest      Search      Context     Total
Femind 0.1     2.3/6/8     8.1/18/22   4.2/8/10    14.6/32/40
OMEGA Memory     1.8/4/5     6.9/15/18   3.5/6/8     12.2/25/31

Cost
────────────────────────────────────────────────────────────────────
System           Tokens In   Tokens Out  Est. Cost
Femind 0.1     1,234,567   45,678      $2.34
```

### JSONL (per-question results)
```json
{"question_id":"q001","system":"femind","question_type":"temporal-reasoning","hypothesis":"March 5th","ground_truth":"March 5th","correct":true,"ingest_ms":12,"retrieval_ms":8,"generation_ms":2100,"judge_ms":890,"tokens_used":1024,"tokens_generated":42}
```

### JSON Summary (per-run aggregate)
### Markdown (GitHub-ready)
### CSV (spreadsheet)
### Failure Analysis (missed questions with full context)

---

## 12. Judge Design

### Type-Specific Prompts (from LongMemEval methodology)

| Type | Judge Instruction |
|------|-------------------|
| Abstention | "Did the model correctly identify this question as unanswerable?" |
| Temporal Reasoning | "Allow off-by-one errors for days/weeks/months. Focus on relative correctness of temporal relationships." |
| Knowledge Update | "Accept the response if the most recently updated answer is presented as the primary answer." |
| Preference | "Does the response correctly recall and utilize the user's stated personal preference?" |
| Multi-Session | "Does the response correctly synthesize information from across multiple conversation sessions?" |
| Default | "Does the response contain the correct answer or a semantically equivalent statement?" |

### Dual-Model Judging

- Primary judge: fast model (Claude Sonnet / GPT-4o-mini)
- Tiebreaker: stronger model (Claude Opus / GPT-4o) — invoked only on disagreement
- Disagreements logged with both judgments for manual review

### Calibration Suite

- 50 pre-scored (question, ground_truth, hypothesis, expected_judgment) pairs
- `recallbench calibrate` verifies judge accuracy before full runs
- Calibration results included in report metadata for reproducibility

---

## 13. Stage-Level Checkpointing

The evaluation pipeline has 5 stages: reset, ingest, retrieve, generate, judge. Stage-level checkpointing saves state after each stage completes, allowing resumption from any point — not just at the question boundary.

### Checkpoint Format

```json
{
  "question_id": "q042",
  "stage": "retrieve",
  "timestamp": "2026-03-20T15:30:00Z",
  "data": {
    "ingest_stats": { "memories_stored": 47, "duration_ms": 234 },
    "retrieval_result": { "context": "...", "items_retrieved": 12, "tokens_used": 4096 }
  }
}
```

Checkpoints are stored in a `.checkpoints/` directory alongside results. On `--resume`, the runner detects the last completed stage per question and resumes from the next stage. This is critical for:
- **Large datasets** (Medium variant, 2.7GB) where ingestion alone takes minutes per question
- **Expensive LLM calls** — don't re-generate if you already have the hypothesis and just need to re-judge
- **Judge iteration** — re-run judging with a different model without re-running the entire pipeline

---

## 14. Longitudinal Degradation Testing

`recallbench longevity` tests what happens to a memory system's accuracy as it accumulates data over time. No other benchmark tool does this well.

### How It Works

1. Generate or load a pool of conversation sessions (e.g., 1,000 sessions spanning 10 simulated months)
2. Ingest sessions incrementally in chronological batches (e.g., 100 sessions per checkpoint)
3. At each checkpoint, evaluate a fixed set of questions against the accumulated memory
4. Report accuracy curve over time: does the system maintain quality as memory grows?

### Output

```
RecallBench Longevity — Femind v0.1
════════════════════════════════════════════════════

Sessions    Memories    Accuracy    Latency (p50)
────────────────────────────────────────────────────
100         423         92.0%       4.2ms
200         891         91.0%       5.8ms
500         2,104       88.5%       12.3ms
1,000       4,287       85.0%       28.7ms
```

### What It Measures

- **Accuracy degradation** — does retrieval quality drop as the memory store grows?
- **Latency scaling** — how does search time grow with memory count?
- **Consolidation effectiveness** — do systems with deduplication/consolidation maintain quality better?
- **Decay model correctness** — do ACT-R or custom decay models properly age out irrelevant memories?

This is a significant differentiator over memorybench and all academic benchmarks, which only test static snapshots.

---

## 15. Web UI

`recallbench serve` launches a local web server for interactive result inspection. This is a key adoption differentiator — memorybench has a web UI, and CLI-only tools lose non-terminal-native users.

### Design

- **Server:** axum (lightweight, async, Rust-native)
- **Frontend:** Vanilla JS single-page app — no framework, no build step, no node_modules
- **Assets:** Embedded in binary via `rust-embed` — `cargo install recallbench` includes the UI, no separate install
- **Data:** Reads JSONL/JSON results from the results directory

### Pages

1. **Dashboard** — overview of all runs in the results directory. Cards showing system name, dataset, accuracy, date.
2. **Run Detail** — accuracy table, latency charts, cost summary for a single run. Drill-down into per-type breakdown.
3. **Question Inspector** — individual question view: question text, ground truth, hypothesis, retrieved context, judge decision, latency breakdown. Filter/sort by type, correctness, latency.
4. **Comparison View** — side-by-side results from `compare` runs. Highlight per-question disagreements.
5. **Longevity View** — accuracy and latency curves over time from `longevity` runs.

### Non-Goals for Web UI

- No authentication or multi-user support — this is a local development tool
- No database — reads flat files from results directory
- No real-time streaming — view completed runs only
- No complex charting library — simple CSS/SVG charts or inline sparklines

---

## 16. Dependencies

```toml
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
indicatif = "0.17"
anyhow = "1"
async-trait = "0.1"
comfy-table = "7"
toml = "0.8"
csv = "1"
governor = "0.8"          # Token bucket rate limiting
hdrhistogram = "7"        # Latency percentile computation
axum = "0.8"              # Web UI HTTP server
rust-embed = "8"          # Embed static assets in binary
tower-http = { version = "0.6", features = ["fs", "cors"] }  # Static file serving
```

---

## 17. DIAL Build Phases

Each phase produces a working, testable increment. Tasks are ordered for dependency resolution — each task builds on previous tasks within and across phases.

### Phase 1: Foundation (Project Structure + Core Types)

| # | Task | Description | Test |
|---|------|-------------|------|
| 1 | Workspace setup | Convert to Cargo workspace with `recallbench` binary crate. Add workspace dependencies. Configure build/test commands. | `cargo build` succeeds |
| 2 | Core types | Implement `types.rs`: `BenchmarkQuestion`, `ConversationSession`, `Turn`, `EvalResult`, `IngestStats`, `RetrievalResult` with serde derive | Unit tests for serialization/deserialization round-trip |
| 3 | Trait definitions | Implement `traits.rs`: `MemorySystem`, `LLMClient`, `BenchmarkDataset` async traits with full documentation | Compiles with trait object usage (`Box<dyn MemorySystem>`) |
| 4 | Error types | Implement `errors.rs`: `RecallBenchError` enum covering dataset, system, llm, judge, io, config error categories | Unit tests for error display and conversion |
| 5 | Configuration | Implement `config.rs`: parse `recallbench.toml` with defaults for concurrency, budget, models, output dir, rate limits. Fall back to env vars. | Unit tests for TOML parsing, defaults, env override |
| 6 | Echo system adapter | Implement `systems/echo.rs`: a test adapter that returns input sessions as retrieval context (no actual memory). Implements `MemorySystem`. | Unit test: ingest sessions, retrieve returns them verbatim |

**Exit criteria:** `cargo test` passes. All core types compile and serialize. Echo adapter implements MemorySystem trait.

### Phase 2: Dataset Engine

| # | Task | Description | Test |
|---|------|-------------|------|
| 7 | Dataset download | Implement `datasets/download.rs`: async download from URL with progress bar (indicatif), file caching in `~/.cache/recallbench/`, resume on partial download, skip if cached | Unit test with mock URL; integration test downloads a small fixture |
| 8 | Dataset trait + registry | Implement `datasets/mod.rs`: `BenchmarkDataset` trait, dataset registry that maps string names to loaders, `list_datasets()` function | Unit test: register and retrieve datasets by name |
| 9 | LongMemEval parser | Implement `datasets/longmemeval.rs`: parse all 3 variants (oracle, S, M). Map 7 question types to strings. Handle `Answer` enum (single, number, array). Detect abstention via `_abs` in question_id. | Unit tests with fixture data covering all question types and answer formats |
| 10 | LongMemEval dataset integration | Wire LongMemEval into the dataset registry. Implement download URLs for all 3 variants from HuggingFace. Add `stats` computation (question count per type, total turns, total sessions). | Integration test: download oracle variant, parse all 500 questions, verify type distribution |
| 11 | LoCoMo parser | Implement `datasets/locomo.rs`: parse LoCoMo format, normalize into BenchmarkQuestion. Handle LoCoMo's different structure (10 conversations, QA pairs, event summaries). | Unit tests with LoCoMo fixture data |
| 12 | ConvoMem parser | Implement `datasets/convomem.rs`: parse ConvoMem format (used by memorybench). Normalize into BenchmarkQuestion. Handle ConvoMem's conversation structure and question types. | Unit tests with ConvoMem fixture data |
| 13 | Custom dataset format | Implement `datasets/custom.rs`: define a simple JSON schema for user-defined datasets. Validate schema on load. Normalize into BenchmarkQuestion. | Unit test with sample custom dataset |
| 14 | Dataset validation | Implement `validate` logic: check required fields, question type consistency, session structure, ground truth format. Return structured validation errors. | Unit tests for valid and invalid datasets |

**Exit criteria:** LongMemEval oracle downloads, parses, and reports correct statistics (500 questions, correct type distribution). LoCoMo, ConvoMem, and custom datasets parse correctly.

### Phase 3: LLM Integration

RecallBench supports multiple LLM providers for both generation and judging. Each provider supports **two modes**: CLI subscription (default, no API key needed) and direct HTTP API. Users choose via `recallbench.toml` or CLI flags.

**Supported providers:**
- **Claude** — `claude --print` (CLI subscription) or Anthropic Messages API
- **ChatGPT** — `chatgpt` CLI or OpenAI Chat Completions API
- **Gemini** — `gemini` CLI or Google Generative AI API
- **Codex** — `codex` CLI (OpenAI)

| # | Task | Description | Test |
|---|------|-------------|------|
| 15 | LLM client trait + registry | Implement `llm/mod.rs`: `LLMClient` trait with `generate` and `generate_with_seed`. Provider registry mapping names to clients. Support `cli` and `api` modes per provider. | Compiles with trait objects |
| 16 | Rate limiter | Implement `llm/rate_limit.rs`: token bucket rate limiter using `governor` crate. Configurable RPM and TPM per provider. Wraps any `LLMClient` with rate-limited version. Only applies to API mode (CLI providers handle their own rate limiting). | Unit test: verify rate limiting delays requests appropriately |
| 90 | Anthropic/Claude client | Implement `llm/anthropic.rs`: support BOTH Claude Code CLI subscription (`claude --print`, default, no API key needed) AND direct HTTP API to Claude Messages API. User chooses mode via config. CLI is the default. | Integration test: send prompt via CLI, verify response |
| 91 | OpenAI/ChatGPT client | Implement `llm/openai.rs`: support BOTH ChatGPT CLI and direct HTTP API to OpenAI Chat Completions API. User chooses via config. | Integration test with CLI or API |
| 92 | Gemini client | Implement `llm/gemini.rs`: support Gemini CLI and direct HTTP API to Google Generative AI API. User chooses via config. | Integration test with CLI or API |
| 93 | Codex CLI client | Implement `llm/codex.rs`: shell out to `codex` CLI with appropriate flags, parse response. CLI-only provider. | Unit test with mock |
| 94 | Generic CLI adapter | Implement `llm/cli.rs`: generic CLI adapter pattern that Claude/ChatGPT/Gemini/Codex all use internally. Detect installed providers via `which`. Provider selection via `--gen-model` and `--judge-model` flags (e.g., `--gen-model claude-sonnet`, `--judge-model chatgpt-4o`). | Unit test: provider detection, flag parsing |

**Exit criteria:** Can send prompts via Claude CLI subscription (no API key) and get responses. Multiple providers detected and selectable. Rate limiter works for API mode.

### Phase 4: Judge System

| # | Task | Description | Test |
|---|------|-------------|------|
| 20 | Judge prompts | Implement `judge/prompts.rs`: type-specific prompt templates for all LongMemEval question types (abstention, temporal, knowledge-update, preference, multi-session, single-session, default). Prompts take (question, ground_truth, hypothesis) and return a binary judgment prompt. | Unit tests verifying prompt generation for each type |
| 21 | LLM judge | Implement `judge/mod.rs`: `judge_answer()` function that selects the appropriate prompt template, calls the LLM, parses yes/no response. Handle edge cases (empty response, ambiguous response). | Unit test with mock LLM client returning yes/no |
| 22 | Dual-model judging | Implement `judge/dual.rs`: primary judge + tiebreaker. If primary returns uncertain/borderline, invoke secondary model. Log disagreements. Return final judgment with confidence metadata. | Unit test with mock LLM clients simulating agreement and disagreement |
| 23 | Calibration suite | Implement `judge/calibration.rs`: load 50 pre-scored pairs from `calibration/longmemeval_50.json`. Run judge against all pairs. Report accuracy. Fail if judge accuracy < 90%. | Integration test verifying calibration logic |
| 24 | Create calibration data | Curate 50 representative (question, ground_truth, hypothesis, expected_judgment) pairs from LongMemEval. Cover all question types. Include edge cases (partial matches, near-misses, semantic equivalents). | Validate all 50 pairs have expected fields |

**Exit criteria:** Judge correctly classifies calibration pairs with >90% accuracy. Dual-model judging works.

### Phase 5: Metrics & Reporting

| # | Task | Description | Test |
|---|------|-------------|------|
| 25 | Accuracy metrics | Implement `metrics/accuracy.rs`: per-type accuracy, task-averaged (mean of per-type), overall (mean of all), abstention accuracy. Handle empty categories gracefully. | Unit tests with known EvalResult sets |
| 26 | Latency metrics | Implement `metrics/latency.rs`: collect per-operation latencies, compute p50/p95/p99 using hdrhistogram. Track ingest, retrieval, generation, judge latencies separately. | Unit tests with synthetic latency data |
| 27 | Cost metrics | Implement `metrics/cost.rs`: track input/output token counts, compute estimated dollar cost per provider (configurable pricing). | Unit test with known token counts |
| 28 | Terminal table report | Implement `report/table.rs`: formatted terminal output using comfy-table. Show accuracy table, latency table, cost table. Support single-system and multi-system comparison layouts. | Unit test verifying table output matches expected format |
| 29 | Markdown report | Implement `report/markdown.rs`: GitHub-ready markdown tables with the same data as terminal output. Include metadata header (date, dataset, variant, recallbench version). | Unit test verifying markdown output |
| 30 | JSON report | Implement `report/json.rs`: machine-readable JSON summary with all metrics, per-system results, metadata. Versioned schema. | Unit test verifying JSON structure and round-trip |
| 31 | CSV report | Implement `report/csv.rs`: spreadsheet-friendly CSV with one row per question per system. Headers match EvalResult fields. | Unit test verifying CSV output |
| 32 | Failure analysis | Implement `report/failure.rs`: export all missed questions with full context (question, ground truth, hypothesis, retrieved context, question type). Group by failure pattern. | Unit test with mixed correct/incorrect results |
| 33 | Report dispatch | Implement `report/mod.rs`: `--format` flag dispatch to table/markdown/json/csv. Default to table. Support writing to stdout or file. | Unit test for format selection |

**Exit criteria:** All report formats produce correct output from a set of EvalResults. Latency percentiles compute correctly.

### Phase 6: Benchmark Runner

| # | Task | Description | Test |
|---|------|-------------|------|
| 34 | Resume logic | Implement `resume.rs`: load existing JSONL results file, extract completed question IDs, filter question queue to skip completed. Append new results incrementally. | Unit test: create partial JSONL, verify resume skips completed |
| 35 | Stage-level checkpointing | Implement `checkpoint.rs`: save/load per-question pipeline state after each stage (ingest, retrieve, generate, judge). Store checkpoints in `.checkpoints/` directory. On resume, detect last completed stage and resume from next. Support re-judging without re-generating. | Unit test: checkpoint after retrieve, resume from generate stage |
| 36 | Worker pipeline | Implement single-question evaluation pipeline: reset → ingest all sessions → retrieve context → build generation prompt → call LLM → judge answer → return EvalResult. Time each stage. Write checkpoint after each stage. | Integration test with echo adapter + mock LLM |
| 37 | Concurrency engine | Implement `concurrency.rs`: tokio task pool with configurable concurrency limit (semaphore). Question queue distribution. Results collector channel. Progress bar (indicatif) with running accuracy. | Integration test: run 10 questions concurrently with echo adapter |
| 38 | Runner orchestration | Implement `runner.rs`: ties everything together. Load dataset → filter by type (if --filter) → check resume/checkpoints → spawn workers → collect results → compute metrics → generate report. Handle errors per-question (skip and log, don't abort). | Integration test: full run with echo adapter, 20 questions, verify metrics |
| 39 | Generation prompt templates | Implement generation prompt builder: construct the LLM prompt from retrieved context + question + date. Use LongMemEval standard template. Support per-dataset prompt customization. | Unit test verifying prompt format |

**Exit criteria:** Full benchmark run completes with echo adapter. Resume works. Concurrency scales. Progress bar shows running accuracy.

### Phase 7: System Adapters

| # | Task | Description | Test |
|---|------|-------------|------|
| 40 | Generic HTTP adapter | Implement `systems/http.rs`: configurable REST endpoints via TOML. Map ingest/retrieve/reset to HTTP calls. Parse JSON responses. Handle timeouts and retries. | Unit test with mock HTTP server |
| 41 | Generic subprocess adapter | Implement `systems/subprocess.rs`: configurable CLI commands via TOML. Pipe session data as JSON stdin, parse JSON stdout. Handle process errors and timeouts. | Unit test with echo subprocess |
| 42 | System registry | Implement `systems/mod.rs`: registry mapping system names to adapter constructors. Support built-in adapters (echo, http, subprocess) + external crate adapters. | Unit test for registration and lookup |
| 43 | Femind adapter | Implement `adapters/recallbench-femind/`: native Rust adapter linking against femind crate. Map ConversationSession to Femind's MemoryRecord. Use Femind's hybrid search for retrieval. | Integration test: ingest and retrieve with real Femind engine |
| 44 | Mem0 adapter | Implement `adapters/recallbench-mem0/`: HTTP adapter calling Mem0's REST API (add memory, search). Handle Mem0's response format. | Unit test with mock Mem0 server |
| 45 | OMEGA adapter | Implement `adapters/recallbench-omega/`: HTTP adapter for OMEGA's MCP-compatible API. Handle OMEGA's graph memory format. | Unit test with mock OMEGA server |
| 46 | Zep adapter | Implement `adapters/recallbench-zep/`: HTTP adapter calling Zep's REST API (`/memory`, `/search`). Handle Zep's session-based memory model and response format. | Unit test with mock Zep server |

**Exit criteria:** Femind adapter reproduces the historical femind benchmark range. HTTP and subprocess adapters work with TOML configuration. All 4 named adapters (Femind, Mem0, OMEGA, Zep) implemented.

### Phase 8: CLI

| # | Task | Description | Test |
|---|------|-------------|------|
| 47 | CLI framework | Implement `main.rs` with clap: subcommands for `run`, `compare`, `download`, `report`, `stats`, `validate`, `calibrate`, `datasets`, `failures`, `longevity`, `serve`. Global flags for verbosity and config file path. | `recallbench --help` shows all subcommands |
| 48 | `download` subcommand | Wire dataset download to CLI. Flags: `--variant`, `--force` (re-download). Show download progress. | Manual test: download LongMemEval oracle |
| 49 | `run` subcommand | Wire benchmark runner to CLI. Flags: `--system`, `--system-config`, `--dataset`, `--variant`, `--concurrency`, `--budget`, `--judge-model`, `--gen-model`, `--output`, `--seed`, `--filter`, `--resume`. | Integration test with echo adapter |
| 50 | `compare` subcommand | Run multiple systems sequentially against the same dataset. Flags: `--systems` (comma-separated). Produce comparative report. | Integration test with multiple echo adapters |
| 51 | `report` subcommand | Load JSONL or JSON results, compute metrics, output in specified format. Flags: `--format`, `--output`. | Unit test with fixture results |
| 52 | `stats` subcommand | Show dataset statistics: question count per type, total sessions, total turns, estimated tokens. | Unit test with parsed dataset |
| 53 | `validate` subcommand | Validate custom dataset JSON against schema. Report errors. | Unit test with valid and invalid fixtures |
| 54 | `calibrate` subcommand | Run judge calibration suite. Report accuracy per question type. Fail with clear message if below threshold. | Integration test with mock LLM |
| 55 | `failures` subcommand | Load results, extract missed questions, generate failure analysis report. Flags: `--export`, `--type-filter`. | Unit test with fixture results |
| 56 | `datasets` subcommand | List all registered datasets with name, description, variants, question counts. | Unit test verifying output |

**Exit criteria:** All CLI subcommands work end-to-end. `recallbench run --system echo --dataset longmemeval --variant oracle` completes successfully.

### Phase 9: Additional Datasets

| # | Task | Description | Test |
|---|------|-------------|------|
| 57 | MemBench parser | Implement `datasets/membench.rs`: parse MemBench format (ACL 2025). Handle multi-aspect evaluation (effectiveness, efficiency, capacity). Normalize to BenchmarkQuestion. | Unit tests with fixture data |
| 58 | MemoryAgentBench parser | Implement `datasets/mab.rs`: parse MemoryAgentBench (ICLR 2026). Handle EventQA and FactConsolidation tasks. Map selective forgetting tests. | Unit tests with fixture data |
| 59 | HaluMem parser | Implement `datasets/halumem.rs`: parse HaluMem format (MemTensor). Handle memory hallucination question types. Map Medium and Long variants. | Unit tests with fixture data |
| 60 | Dataset registry update | Register all new datasets in the registry. Update `datasets` subcommand output. Add download URLs. | Integration test: all datasets list correctly |
| 61 | Cross-dataset metrics | Extend metrics to support dataset-specific question type mappings. Ensure per-type accuracy works across datasets with different type taxonomies. | Unit test with mixed dataset results |

**Exit criteria:** All 7 dataset families (LongMemEval, LoCoMo, ConvoMem, MemBench, MemoryAgentBench, HaluMem, custom) parse correctly and produce valid BenchmarkQuestion sets.

### Phase 10: Advanced Features

| # | Task | Description | Test |
|---|------|-------------|------|
| 62 | Question type filtering | Implement `--filter` in runner: accept comma-separated question types, only evaluate matching questions. Work across all datasets. | Unit test: filter reduces question set correctly |
| 63 | Deterministic seeding | Pass `--seed` through to LLM clients. Use seed for Anthropic API (if supported) and OpenAI API. Log seed in report metadata for reproducibility. | Verify same seed produces same report metadata |
| 64 | Stress test mode | Implement `--stress` flag: run the same question set N times and report accuracy variance, mean, stddev. Useful for measuring judge consistency. | Integration test with echo adapter, 3 repetitions |
| 65 | Token budget analysis | Add `--budget-sweep` mode: run the same questions at multiple token budgets (e.g., 4096, 8192, 16384, 32768) and report accuracy vs. budget curve. | Integration test with echo adapter |
| 66 | Comparative diff | When running `compare`, highlight per-question disagreements between systems. Show which questions system A got right but system B missed. | Unit test with divergent results |
| 67 | Longitudinal degradation engine | Implement `longevity.rs`: generate/load session pool, ingest incrementally in chronological batches, evaluate at configurable checkpoints (e.g., every 100 sessions). Track accuracy and latency at each checkpoint. Output accuracy-over-time curve. | Integration test with echo adapter, 3 checkpoints |
| 68 | `longevity` subcommand | Wire longevity engine to CLI. Flags: `--system`, `--sessions`, `--checkpoints`, `--eval-questions`, `--output`. Generate longevity report (table + JSON). | Integration test with echo adapter |

**Exit criteria:** All advanced features work and produce meaningful output. Longevity test shows accuracy/latency curves over simulated time.

### Phase 11: Web UI

| # | Task | Description | Test |
|---|------|-------------|------|
| 69 | Web server setup | Implement `web/mod.rs`: axum HTTP server with configurable port. Serve static assets via rust-embed. CORS headers for local development. Graceful shutdown on Ctrl+C. | Server starts, serves index.html on `/` |
| 70 | API routes | Implement `web/routes.rs`: JSON API endpoints — `GET /api/runs` (list result files), `GET /api/runs/:id` (load specific result), `GET /api/runs/:id/metrics` (compute metrics for a result), `GET /api/runs/:id/questions` (list all questions with results), `GET /api/runs/:id/failures` (missed questions only). | Unit tests for each route with fixture data |
| 71 | Dashboard page | Implement `web/static/index.html` + `app.js`: dashboard listing all runs in the results directory. Cards showing system name, dataset, accuracy, date. Click to drill into run detail. | Manual test: dashboard displays runs |
| 72 | Run detail + question inspector | Implement run detail view: accuracy table, latency breakdown, cost summary. Question list with filter/sort by type, correctness, latency. Click any question to see full context: question text, ground truth, hypothesis, retrieved context, judge decision. | Manual test: drill into individual questions |
| 73 | Comparison view | Implement comparison view for `compare` results: side-by-side accuracy tables, per-question disagreement highlighting (which questions did system A get right but B miss). | Manual test: load comparison results |
| 74 | Longevity view | Implement longevity view: accuracy and latency curves over session count from `longevity` results. Simple SVG line charts. | Manual test: load longevity results |
| 75 | `serve` subcommand | Wire web server to CLI. Flags: `--port` (default 8888), `--results-dir` (default `results/`). Open browser automatically on launch. | `recallbench serve` starts and opens browser |

**Exit criteria:** `recallbench serve` launches local web UI. All views render correctly from result files. No external dependencies (no npm, no build step).

### Phase 12: Testing & Hardening

| # | Task | Description | Test |
|---|------|-------------|------|
| 76 | Unit test suite | Ensure all modules have unit tests. Target >80% line coverage on core logic (types, metrics, resume, checkpoints, prompts, config). | `cargo test` — all pass |
| 77 | Integration test: echo end-to-end | Full pipeline test: download fixture dataset → run with echo adapter → verify metrics → verify JSONL output → verify resume → verify stage checkpoint → verify report formats. | Single integration test covering the full loop |
| 78 | Integration test: Femind end-to-end | Full pipeline test with Femind adapter: download LongMemEval oracle → run 10 questions → verify scores are in expected range. | Integration test (requires femind crate) |
| 79 | Error handling hardening | Audit all `unwrap()` and `expect()` calls. Replace with proper error propagation. Ensure per-question failures don't abort the full run. Add timeout handling for LLM calls and HTTP adapters. | Chaos test: inject failures into echo adapter, verify graceful degradation |
| 80 | Edge cases | Handle: empty datasets, single-question datasets, datasets with no abstention questions, systems that return empty context, LLM returning non-yes/no, JSONL corruption on resume, checkpoint corruption, concurrent file writes. | Unit tests for each edge case |
| 81 | Tracing & logging | Add structured tracing throughout: question processing, LLM calls, retries, errors, timing. Respect `--verbose` / `RUST_LOG` for filtering. | Verify log output at different verbosity levels |

**Exit criteria:** `cargo test` passes all unit and integration tests. No panics on malformed input. Graceful error handling throughout.

### Phase 13: Documentation & Publication

| # | Task | Description | Test |
|---|------|-------------|------|
| 82 | README | Write comprehensive README: what it is, installation, quickstart, supported datasets, supported systems, adapter guide, CLI reference, output examples, web UI screenshots. | Review for completeness |
| 83 | Adapter guide | Write "Add Your System" guide: implement MemorySystem trait, configure HTTP adapter, configure subprocess adapter. Include template code. | Review for clarity |
| 84 | CONTRIBUTING.md | Write contributor guide: how to add datasets, how to add adapters, code style, testing requirements. | Review |
| 85 | CHANGELOG.md | Write changelog for v1.0.0 covering all features. | Review |
| 86 | Cargo.toml metadata | Update all crate metadata: description, documentation, homepage, repository, readme, categories, keywords. Verify workspace member metadata. | `cargo publish --dry-run` succeeds |
| 87 | CI workflow | GitHub Actions: cargo build, cargo test, cargo clippy, cargo fmt check. Run on push and PR. | CI passes on push |
| 88 | crates.io publish | Publish recallbench v1.0.0 and adapter crates to crates.io. Verify `cargo install recallbench` works. | Installation succeeds and binary runs |
| 89 | License files | Add LICENSE-MIT and LICENSE-APACHE to workspace root and all crate directories. | Verify files exist |

**Exit criteria:** `cargo install recallbench` works. README provides clear quickstart. All crates published. CI green.

---

## 18. DIAL Configuration

```bash
# Initialize DIAL for recallbench
cd ~/projects/recallbench
dial init --phase foundation

# Configure build/test
dial config set build_cmd "cargo build --workspace"
dial config set test_cmd "cargo test --workspace"

# Index specs
dial index
```

**Total: 13 phases, 89 tasks**

Build order enforces dependencies:
- Phase 1 (Foundation) has no dependencies
- Phase 2 (Datasets) depends on Phase 1 types
- Phase 3 (LLM) depends on Phase 1 traits
- Phase 4 (Judge) depends on Phase 3 LLM
- Phase 5 (Metrics) depends on Phase 1 types
- Phase 6 (Runner) depends on Phases 2-5
- Phase 7 (Adapters) depends on Phase 1 traits
- Phase 8 (CLI) depends on Phases 2-7
- Phase 9 (Additional Datasets) depends on Phase 2
- Phase 10 (Advanced Features) depends on Phase 6
- Phase 11 (Web UI) depends on Phase 5 + Phase 6
- Phase 12 (Testing) depends on all prior phases
- Phase 13 (Documentation) depends on Phase 12

---

## 19. Success Criteria

| Criteria | Measure |
|----------|---------|
| Reproduce LongMemEval published scores within ±2% | Femind adapter matches the historical femind benchmark range |
| 500-question eval in <5 minutes (concurrent, 10 workers) | Wall-clock time with LLM API latency |
| 4+ memory systems benchmarked comparatively | Femind + Mem0 + OMEGA + Zep |
| 7 datasets supported | LongMemEval, LoCoMo, ConvoMem, MemBench, MemoryAgentBench, HaluMem, custom |
| Judge calibration >90% accuracy | On 50 pre-scored pairs |
| Stage-level resume works | Interrupt mid-pipeline, resume from last completed stage |
| Longevity test produces degradation curve | Accuracy + latency over 1,000 sessions |
| Web UI renders all views | Dashboard, run detail, question inspector, comparison, longevity |
| Zero panics on malformed input | Fuzz testing with invalid data |
| `cargo install recallbench` works | Clean install on fresh machine |

---

## 20. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| LLM judge inconsistency across providers | Dual-model judging + calibration suite + seed parameter |
| API rate limits during concurrent eval | Per-provider rate limiter with configurable RPM/TPM |
| Memory systems with incompatible APIs | Generic HTTP + subprocess adapters as fallbacks |
| Dataset licensing restrictions | Download on demand, never bundle datasets in the crate |
| Scope creep during DIAL iterations | Strict one-task-per-iteration, no feature additions beyond spec |
| LongMemEval methodology drift | Pin dataset version, document exact judge prompts |
| memorybench (competitor) gaining traction | Ship faster, differentiate on Rust performance + latency profiling + longevity testing + vendor neutrality |
| Web UI scope creep | Vanilla JS only, no frameworks, no build step — function over polish |

---

## 21. Feature Comparison vs. Competition

| Feature | RecallBench | memorybench | letta-evals | Academic tools |
|---------|-------------|-------------|-------------|----------------|
| Language | Rust | TypeScript | Python | Python |
| Pluggable systems | Trait + HTTP + subprocess | Provider adapters | Partial (LLMs only) | No |
| Named adapters | 4 (Femind, Mem0, OMEGA, Zep) | 3 (Supermemory, Mem0, Zep) | 0 | 0 |
| Datasets | 7 | 3 | Custom only | 1 each |
| Concurrent eval | Yes (configurable pool) | Unknown | No | No |
| Latency profiling (p50/p95/p99) | Yes | No | No | No |
| Token cost tracking | Yes | No | No | No |
| Dual-model judging | Yes | No | Multi-grader gates | No |
| Judge calibration | Yes (50 pre-scored pairs) | No | No | No |
| Stage-level checkpointing | Yes (5 stages) | Yes (5 stages) | No | No |
| Question-level resume | Yes | Yes | Yes | No |
| Failure analysis export | Yes | No | No | No |
| Stress test (variance) | Yes | No | No | No |
| Budget sweep | Yes | No | No | No |
| Comparative diff | Yes | No | No | No |
| Longitudinal degradation | Yes | No | No | No |
| Web UI | Yes (embedded, no deps) | Yes | No | No |
| Deterministic seeding | Yes | No | No | No |
| Quick mode (stratified sampling) | Yes | No | No | No |
| OpenAI-compatible endpoints | Yes (Ollama, vLLM, DeepInfra, etc.) | No | No | No |
| Local inference support | Yes (zero cost iteration) | No | No | No |
| Vendor-neutral | Yes | No (Supermemory) | No (Letta) | Academic |
| Custom datasets | Yes (JSON schema) | Yes | Yes (YAML) | No |
| Output formats | 4 (table, markdown, JSON, CSV) | Structured reports | JSONL | Varies |
