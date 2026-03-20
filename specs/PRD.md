# RecallBench — Universal AI Memory System Benchmark

**Version:** 1.0
**Date:** 2026-03-20
**Status:** Draft

---

## 1. Problem

There is no standard, reproducible, multi-system benchmark tool for AI memory systems. Each memory project (MindCore, OMEGA, Mem0, Hindsight, Letta) either self-reports numbers or runs its own evaluation scripts. Results are not comparable — different datasets, different judges, different prompts, different hardware.

LongMemEval (ICLR 2025) established the gold-standard dataset, but it ships as a Python notebook, not a reusable tool. Running it against a new memory system requires significant integration work. Other benchmarks (LOCOMO, MemBench, MemoryAgentBench) have the same problem.

RecallBench solves this by providing a single binary that any memory system can plug into via a standard interface, producing comparable, reproducible results across systems and datasets.

---

## 2. Vision

`cargo install recallbench` gives any developer a tool to benchmark their memory system against established datasets with a single command. Memory system authors implement one trait, and RecallBench handles dataset management, evaluation orchestration, LLM judging, and comparative reporting.

RecallBench becomes the defacto standard for measuring AI memory system quality — the way `criterion` is for Rust microbenchmarks or `MLPerf` is for ML hardware.

---

## 3. Goals

1. **Multi-system**: Benchmark any memory system through a pluggable trait interface
2. **Multi-dataset**: Support LongMemEval, LOCOMO, MemBench, and custom datasets
3. **Reproducible**: Deterministic evaluation with pinned judge prompts and versioned datasets
4. **Fast**: Concurrent evaluation with configurable parallelism — 500 questions in minutes, not hours
5. **Comparative**: Side-by-side results tables across systems on identical questions
6. **Extensible**: Custom datasets, custom judges, custom metrics
7. **Publishable**: Output formats suitable for papers, READMEs, and leaderboards

---

## 4. Non-Goals

- RecallBench does not implement a memory system — it tests them
- RecallBench does not train or fine-tune models
- RecallBench is not an LLM benchmark (it benchmarks memory/retrieval, not generation quality)
- RecallBench does not host a web leaderboard (v1 — may add later)

---

## 5. Architecture

### 5.1 Project Structure

```
recallbench/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Public API for programmatic use
│   ├── runner.rs            # Benchmark orchestration engine
│   ├── traits.rs            # MemorySystem + LLMClient traits
│   ├── datasets/
│   │   ├── mod.rs           # Dataset trait + registry
│   │   ├── longmemeval.rs   # LongMemEval loader + parser
│   │   ├── locomo.rs        # LOCOMO loader + parser
│   │   └── custom.rs        # User-defined dataset format
│   ├── judge/
│   │   ├── mod.rs           # Judge trait + registry
│   │   ├── llm_judge.rs     # LLM-based binary judging
│   │   └── prompts.rs       # Type-specific judge prompts
│   ├── llm/
│   │   ├── mod.rs           # LLMClient trait
│   │   ├── anthropic.rs     # Claude API (direct HTTP)
│   │   ├── openai.rs        # OpenAI API
│   │   └── cli.rs           # Claude CLI fallback
│   ├── systems/
│   │   ├── mod.rs           # Built-in system adapters
│   │   ├── http.rs          # Generic HTTP/REST adapter
│   │   └── subprocess.rs    # Generic subprocess adapter
│   ├── metrics/
│   │   ├── mod.rs           # Metric computation
│   │   ├── accuracy.rs      # Per-type, task-averaged, overall
│   │   └── latency.rs       # Per-operation timing
│   ├── report/
│   │   ├── mod.rs           # Report generation
│   │   ├── table.rs         # Terminal table output
│   │   ├── json.rs          # Machine-readable JSON
│   │   ├── markdown.rs      # GitHub-ready markdown
│   │   └── csv.rs           # Spreadsheet export
│   └── download.rs          # Dataset fetcher with caching
├── adapters/
│   ├── mindcore/            # MindCore native adapter (separate crate)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── mem0/                # Mem0 HTTP adapter
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── omega/               # OMEGA adapter
│       ├── Cargo.toml
│       └── src/lib.rs
├── datasets/
│   └── .gitkeep             # Downloaded datasets cached here
├── specs/
│   └── PRD.md
└── results/                  # Benchmark output directory
```

### 5.2 Core Traits

```rust
/// The primary trait any memory system must implement.
#[async_trait]
pub trait MemorySystem: Send + Sync {
    /// Human-readable name for reports.
    fn name(&self) -> &str;

    /// Version string for reproducibility.
    fn version(&self) -> &str;

    /// Reset all state (called between questions for isolation).
    async fn reset(&self) -> Result<()>;

    /// Ingest a conversation session into the memory system.
    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats>;

    /// Given a question, retrieve relevant context within a token budget.
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

/// Abstraction over LLM providers for generation and judging.
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String>;
}
```

### 5.3 Dataset Abstraction

```rust
/// A benchmark dataset that can be loaded and iterated.
pub trait BenchmarkDataset: Send + Sync {
    fn name(&self) -> &str;
    fn variant(&self) -> &str;
    fn questions(&self) -> &[BenchmarkQuestion];
    fn question_types(&self) -> Vec<String>;
}

/// Universal question format that all datasets normalize into.
pub struct BenchmarkQuestion {
    pub id: String,
    pub question_type: String,
    pub question: String,
    pub ground_truth: Vec<String>,
    pub question_date: Option<String>,
    pub sessions: Vec<ConversationSession>,
    pub is_abstention: bool,
}

pub struct ConversationSession {
    pub id: String,
    pub date: Option<String>,
    pub turns: Vec<Turn>,
}

pub struct Turn {
    pub role: String,      // "user" or "assistant"
    pub content: String,
}
```

---

## 6. Supported Datasets

### 6.1 LongMemEval (ICLR 2025) — Primary

- **500 questions** across 5 memory abilities and 7 question types
- **Variants:** Oracle (15MB), Small (277MB), Medium (2.7GB)
- **Source:** HuggingFace `xiaowu0162/longmemeval-cleaned`
- **Metrics:** Per-type accuracy, task-averaged accuracy, overall accuracy, abstention accuracy

| Ability | Types | Description |
|---------|-------|-------------|
| Information Extraction | single-session-user, single-session-assistant, single-session-preference | Recall specific facts |
| Multi-Session Reasoning | multi-session | Synthesize across sessions |
| Knowledge Updates | knowledge-update | Track changed information |
| Temporal Reasoning | temporal-reasoning | Reason about dates and time |
| Abstention | any type with `_abs` | Refuse unanswerable questions |

### 6.2 LOCOMO (Snap Research) — Phase 2

- Long-context conversation memory evaluation
- Different question distribution than LongMemEval

### 6.3 MemBench (ACL 2025) — Phase 2

- Comprehensive memory system testing

### 6.4 MemoryAgentBench (ICLR 2026) — Phase 3

- Tests selective forgetting
- Aligns with ACT-R activation models

### 6.5 Custom Datasets — Phase 2

Users can define datasets in a standard JSON format:
```json
[
  {
    "id": "q001",
    "type": "temporal",
    "question": "When did the user last mention pizza?",
    "answer": ["March 5th"],
    "sessions": [...]
  }
]
```

---

## 7. Memory System Adapters

### 7.1 Native Rust Adapters (direct crate dependency)

| System | Adapter | Integration |
|--------|---------|-------------|
| MindCore | `recallbench-mindcore` | `mindcore = { version = "0.1" }` — direct API calls |

### 7.2 HTTP Adapters (REST API)

| System | Adapter | Integration |
|--------|---------|-------------|
| Mem0 | `recallbench-mem0` | HTTP calls to Mem0 Cloud or self-hosted |
| OMEGA | `recallbench-omega` | HTTP calls to OMEGA's MCP server |
| Letta | `recallbench-letta` | HTTP calls to Letta server |

### 7.3 Generic Adapters

| Type | Description |
|------|-------------|
| `GenericHttpAdapter` | Any system with a REST API — configure endpoints via TOML |
| `SubprocessAdapter` | Any system with a CLI — configure commands via TOML |

```toml
# recallbench.toml — Generic HTTP adapter config
[system]
name = "my-memory-system"
version = "1.0"

[system.endpoints]
reset = "POST http://localhost:8080/reset"
ingest = "POST http://localhost:8080/ingest"
retrieve = "POST http://localhost:8080/retrieve"
```

---

## 8. CLI Interface

```bash
# List available datasets
recallbench datasets

# Download a dataset
recallbench download longmemeval --variant oracle

# Run a single system
recallbench run \
  --system mindcore \
  --dataset longmemeval \
  --variant oracle \
  --concurrency 10 \
  --budget 16384 \
  --judge-model claude-sonnet \
  --output results/mindcore-oracle.jsonl

# Run comparative benchmark (multiple systems)
recallbench compare \
  --systems mindcore,omega,mem0 \
  --dataset longmemeval \
  --variant oracle \
  --output results/comparison.json

# Generate report from results
recallbench report results/mindcore-oracle.jsonl --format table
recallbench report results/mindcore-oracle.jsonl --format markdown
recallbench report results/comparison.json --format table

# Show dataset statistics
recallbench stats longmemeval --variant oracle

# Validate a custom dataset
recallbench validate my-dataset.json

# Resume an interrupted run
recallbench run --resume results/mindcore-oracle.jsonl [...]
```

---

## 9. Output & Reporting

### 9.1 Terminal Table

```
RecallBench v0.1.0 — LongMemEval Oracle
════════════════════════════════════════════════════════════════════

System           Task-Avg  Overall  IE     MR     KU     TR     ABS
─────────────────────────────────────────────────────────────────────
MindCore v0.1    88.7%     88.7%    90.2%  81.0%  91.0%  94.0%  85.0%
OMEGA Memory     95.4%     95.2%    96.1%  93.5%  95.8%  94.1%  97.0%
Hindsight        91.4%     91.3%    92.0%  88.9%  91.5%  90.2%  94.5%

Latency (p50 / p99)
─────────────────────────────────────────────────────────────────────
System           Ingest     Search     Context    Total
MindCore v0.1    2.3/8.1    8.1/22.4   4.2/9.8    14.6/40.3
OMEGA Memory     1.8/5.2    6.9/18.1   3.5/7.6    12.2/30.9
```

### 9.2 Markdown (for README / papers)

Auto-generates a GitHub-ready markdown table with the same data.

### 9.3 JSONL (machine-readable)

One result per line for streaming and incremental processing:
```json
{"question_id":"q001","system":"mindcore","hypothesis":"March 5th","ground_truth":"March 5th","correct":true,"type":"temporal-reasoning","latency_ms":142,"tokens":1024}
```

### 9.4 JSON Summary

```json
{
  "benchmark": "longmemeval",
  "variant": "oracle",
  "date": "2026-03-20",
  "recallbench_version": "0.1.0",
  "systems": [
    {
      "name": "MindCore",
      "version": "0.1.0",
      "task_averaged": 0.887,
      "overall": 0.887,
      "per_type": { ... },
      "latency": { ... }
    }
  ]
}
```

---

## 10. Evaluation Pipeline

For each system under test, for each question:

```
┌─────────┐    ┌─────────┐    ┌──────────┐    ┌──────────┐    ┌─────────┐
│  Reset   │───▶│ Ingest  │───▶│ Retrieve │───▶│ Generate │───▶│  Judge  │
│  System  │    │Sessions │    │ Context  │    │  Answer  │    │ Score   │
└─────────┘    └─────────┘    └──────────┘    └──────────┘    └─────────┘
                    │               │               │               │
                    ▼               ▼               ▼               ▼
              IngestStats    RetrievalResult   Hypothesis      Correct/Wrong
              (memories,     (context, tokens,  (LLM answer)   (binary)
               duration)      duration)
```

Steps 1-3 are **system-specific** (behind the `MemorySystem` trait).
Steps 4-5 are **universal** (RecallBench handles LLM generation and judging).

---

## 11. Concurrency Model

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

- Each worker gets its own `MemorySystem` instance (via `reset()`)
- Configurable concurrency: `--concurrency N` (default: 10)
- LLM rate limiting handled per-provider
- Progress bar shows completed/total with running accuracy

---

## 12. Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
indicatif = "0.17"
anyhow = "1"
async-trait = "0.1"
comfy-table = "7"       # Terminal table formatting
tabled = "0.17"         # Alternative table formatting
```

---

## 13. Judge Design

### 13.1 Type-Specific Prompts

Each question type gets a tailored judge prompt (from LongMemEval methodology):

| Type | Judge Instruction |
|------|-------------------|
| Abstention | "Did the model correctly identify this as unanswerable?" |
| Temporal Reasoning | "Allow off-by-one for days/weeks/months. Focus on relative correctness." |
| Knowledge Update | "Accept if the updated (most recent) answer is primary." |
| Preference | "Does the response recall and utilize personal preference info?" |
| Default | "Does the response contain the correct answer or semantic equivalent?" |

### 13.2 Dual-Model Judging

For borderline cases, use two judge models and require agreement:
- Primary judge: Claude Sonnet (fast, cheap)
- Tiebreaker: Claude Opus or GPT-4o (higher accuracy)
- Disagreements logged for manual review

### 13.3 Judge Calibration

RecallBench ships a calibration set of 50 pre-scored question/answer pairs. Users can verify their judge setup produces expected scores before running full benchmarks.

---

## 14. Phases & Roadmap

### Phase 1: Core Framework (MVP)

**Goal:** Single-system LongMemEval benchmarking with the trait-based architecture.

| Task | Description |
|------|-------------|
| 1.1 | Define `MemorySystem`, `LLMClient`, `BenchmarkDataset` traits |
| 1.2 | Implement LongMemEval dataset loader (port from mindcore-bench) |
| 1.3 | Implement dataset download with caching and resume |
| 1.4 | Implement Anthropic HTTP LLM client (direct API, no SDK dep) |
| 1.5 | Implement LLM judge with type-specific prompts |
| 1.6 | Implement benchmark runner with concurrency pool |
| 1.7 | Implement metrics computation (per-type, task-averaged, overall) |
| 1.8 | Implement terminal table reporter |
| 1.9 | Implement JSONL output with resume support |
| 1.10 | CLI: `run`, `download`, `report`, `stats` subcommands |
| 1.11 | MindCore native adapter (first adapter, validates the trait design) |

**Exit criteria:** `recallbench run --system mindcore --dataset longmemeval --variant oracle` produces correct accuracy scores matching mindcore-bench results.

### Phase 2: Multi-System & Multi-Dataset

**Goal:** Compare multiple systems, support additional datasets.

| Task | Description |
|------|-------------|
| 2.1 | Generic HTTP adapter with TOML configuration |
| 2.2 | Generic subprocess adapter |
| 2.3 | Mem0 HTTP adapter |
| 2.4 | OMEGA adapter |
| 2.5 | `compare` subcommand for side-by-side evaluation |
| 2.6 | LOCOMO dataset loader |
| 2.7 | Custom dataset format + `validate` subcommand |
| 2.8 | Markdown report output |
| 2.9 | JSON summary output |
| 2.10 | CSV export |
| 2.11 | OpenAI LLM client |

**Exit criteria:** `recallbench compare --systems mindcore,mem0 --dataset longmemeval` produces a comparative results table.

### Phase 3: Advanced Features

**Goal:** Production-quality tool with calibration, latency profiling, and community features.

| Task | Description |
|------|-------------|
| 3.1 | Latency profiling (p50/p99 per operation: ingest, search, context) |
| 3.2 | Dual-model judge with tiebreaker |
| 3.3 | Judge calibration suite (50 pre-scored pairs) |
| 3.4 | MemBench dataset loader |
| 3.5 | MemoryAgentBench dataset loader |
| 3.6 | Token cost tracking and reporting |
| 3.7 | Failure analysis: export missed questions with context for debugging |
| 3.8 | Configuration file (`recallbench.toml`) for defaults and system configs |
| 3.9 | `--filter` flag to run specific question types only |
| 3.10 | Deterministic seeding for reproducible LLM outputs (where supported) |

**Exit criteria:** Full benchmark suite with calibrated judges and latency profiling across 3+ systems.

### Phase 4: Community & Publication

**Goal:** Open-source release with documentation and community tooling.

| Task | Description |
|------|-------------|
| 4.1 | Comprehensive README with quickstart |
| 4.2 | "Add your system" guide with adapter template |
| 4.3 | Published results for MindCore, Mem0, OMEGA on LongMemEval |
| 4.4 | GitHub Actions CI running benchmark regression on MindCore |
| 4.5 | crates.io publication with proper version (1.0.0) |
| 4.6 | Blog post / announcement |

---

## 15. Success Metrics

| Metric | Target |
|--------|--------|
| Reproduce LongMemEval published scores within ±2% | Phase 1 |
| 500-question evaluation completes in <5 minutes (concurrent) | Phase 1 |
| At least 3 memory systems benchmarked comparatively | Phase 2 |
| Community adoption: 2+ external systems using RecallBench | Phase 4 |

---

## 16. Risks

| Risk | Mitigation |
|------|------------|
| LLM judge inconsistency | Dual-model judging + calibration suite |
| API rate limits during concurrent eval | Per-provider rate limiter with backoff |
| Memory systems with incompatible APIs | Generic HTTP + subprocess adapters as fallback |
| Dataset licensing restrictions | Only bundle open-access datasets; download on demand |
| LongMemEval methodology changes | Pin to specific dataset version; document exact methodology |

---

## 17. Competitive Landscape (Known Benchmark Scores)

Reference scores RecallBench should reproduce:

| System | LongMemEval Score | Source |
|--------|-------------------|--------|
| OMEGA Memory | 95.4% | Official leaderboard |
| Mastra OM | 94.87% | Self-reported |
| Hindsight (Vectorize) | 91.4% | arxiv.org/abs/2512.12818 |
| MindCore | 88.7% | mindcore-bench run (2026-03-20) |
| Emergence AI | 86% | Self-reported |
