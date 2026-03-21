# RecallBench

A universal benchmark harness for AI memory systems. Evaluate any memory system against established academic datasets with a single command.

```bash
cargo install recallbench
recallbench run --system echo --dataset longmemeval --variant oracle
```

## Features

- **Multi-system**: Benchmark any memory system via trait implementation, HTTP adapter, or subprocess adapter
- **Multi-dataset**: LongMemEval, LoCoMo, ConvoMem, MemBench, MemoryAgentBench, HaluMem, custom JSON
- **Multi-provider LLM**: Claude, ChatGPT, Gemini, Codex — CLI subscription (no API key) or direct API
- **OpenAI-compatible endpoints**: Ollama, vLLM, LM Studio, DeepInfra, Together, Groq — zero-cost local iteration
- **Quick mode**: Stratified sampling for fast directional signal during development (`--quick`)
- **Latency profiling**: p50/p95/p99 per pipeline stage (ingest, retrieval, generation, judge)
- **Dual-model judging**: Primary + tiebreaker judge with calibration suite
- **Longevity testing**: Measure accuracy degradation over time as memory accumulates
- **Web UI**: Local browser interface for exploring results (`recallbench serve`)
- **Resume**: Interrupt and resume benchmark runs without losing progress
- **Reports**: Terminal table, Markdown, JSON, CSV output formats

## Quick Start

```bash
# Install
cargo install recallbench

# Download a dataset
recallbench download longmemeval --variant oracle

# Run benchmark with echo adapter (test mode)
recallbench run --system echo --dataset longmemeval --variant oracle

# Quick mode for fast iteration (50 stratified questions)
recallbench run --system echo --dataset longmemeval --quick

# Generate report from results
recallbench report results/echo-longmemeval-oracle.jsonl --format markdown

# Browse results in web UI
recallbench serve
```

## Supported Datasets

| Dataset | Source | Description |
|---------|--------|-------------|
| LongMemEval | ICLR 2025 | 500 questions, 5 memory abilities, 7 types |
| LoCoMo | Snap Research | Long-context conversation memory |
| ConvoMem | memorybench | Conversational memory evaluation |
| MemBench | ACL 2025 | Multi-aspect (effectiveness, efficiency, capacity) |
| MemoryAgentBench | ICLR 2026 | Selective forgetting, fact consolidation |
| HaluMem | MemTensor | Memory hallucination detection |
| Custom | User-defined | Any JSON dataset matching the schema |

## Supported LLM Providers

All providers support CLI subscription mode (default, no API key) and/or direct API mode.

| Provider | CLI Command | API | Config Key |
|----------|-------------|-----|------------|
| Claude | `claude --print` | Anthropic Messages API | `llm.anthropic` |
| ChatGPT | `chatgpt` | OpenAI Chat Completions | `llm.openai` |
| Gemini | `gemini` | Google Generative AI | `llm.gemini` |
| Codex | `codex` | — | — |
| Custom | — | Any OpenAI-compatible endpoint | `llm.custom`, `llm.local` |

### Local Inference (Zero Cost)

```toml
# recallbench.toml
[llm.local]
base_url = "http://localhost:11434/v1"
api_key_env = ""
model = "llama3.1:70b"
rate_limit_rpm = 0
```

```bash
recallbench run --system echo --dataset longmemeval --quick \
  --gen-model local --judge-model local
```

## Adding Your Memory System

### Option 1: Implement the Rust trait

Add recallbench as a workspace member and implement the `MemorySystem` trait:

```rust
#[async_trait]
impl MemorySystem for MySystem {
    fn name(&self) -> &str { "my-system" }
    fn version(&self) -> &str { "1.0.0" }
    async fn reset(&self) -> Result<()> { /* clear state */ }
    async fn ingest_session(&self, session: &ConversationSession) -> Result<IngestStats> { /* store */ }
    async fn retrieve_context(&self, query: &str, date: Option<&str>, budget: usize) -> Result<RetrievalResult> { /* search */ }
}
```

See `recallbench/src/systems/echo.rs` for a complete reference implementation.

### Option 2: HTTP adapter (any language)

```toml
# my-system.toml
name = "my-system"
version = "1.0"

[endpoints]
reset = "http://localhost:8080/reset"
ingest = "http://localhost:8080/ingest"
retrieve = "http://localhost:8080/retrieve"
```

```bash
recallbench run --system-config my-system.toml --dataset longmemeval
```

### Option 3: Subprocess adapter

```toml
# my-cli-system.toml
name = "my-cli-system"
version = "1.0"

[commands]
reset = ["my-system", "reset"]
ingest = ["my-system", "ingest"]
retrieve = ["my-system", "retrieve"]
```

## CLI Reference

```
recallbench datasets          List available datasets
recallbench download          Download a dataset
recallbench run               Run benchmark
recallbench compare           Compare multiple systems
recallbench report            Generate report from results
recallbench stats             Show dataset statistics
recallbench validate          Validate custom dataset
recallbench calibrate         Run judge calibration
recallbench failures          Export failure analysis
recallbench longevity         Run longitudinal degradation test
recallbench serve             Launch web UI
```

## Custom Datasets

Create a JSON file matching this schema:

```json
[
  {
    "id": "q001",
    "question_type": "factual",
    "question": "What is the user's favorite color?",
    "ground_truth": ["blue"],
    "sessions": [
      {
        "id": "session_1",
        "date": "2024-01-15",
        "turns": [
          {"role": "user", "content": "My favorite color is blue"},
          {"role": "assistant", "content": "Got it!"}
        ]
      }
    ]
  }
]
```

Validate before running:

```bash
recallbench validate my-dataset.json
```

## Configuration

Create `recallbench.toml` in your project directory:

```toml
[defaults]
concurrency = 10
token_budget = 16384
gen_model = "claude-sonnet"
judge_model = "claude-sonnet"
output_dir = "results"
quick_size = 50

[llm.anthropic]
mode = "cli"           # "cli" (default) or "api"
rate_limit_rpm = 60

[llm.openai]
mode = "cli"
rate_limit_rpm = 60

# Add custom OpenAI-compatible endpoints
[llm.custom]
base_url = "https://api.deepinfra.com/v1/openai"
api_key_env = "DEEPINFRA_API_KEY"
model = "meta-llama/Llama-3.1-70B-Instruct"
rate_limit_rpm = 120

[llm.local]
base_url = "http://localhost:11434/v1"
api_key_env = ""
model = "llama3.1:70b"
rate_limit_rpm = 0
```

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `ANTHROPIC_API_KEY` | Claude API key (only needed in API mode) |
| `OPENAI_API_KEY` | OpenAI API key (only needed in API mode) |
| `GEMINI_API_KEY` / `GOOGLE_API_KEY` | Gemini API key (only needed in API mode) |
| `RECALLBENCH_CONCURRENCY` | Override default concurrency |
| `RECALLBENCH_TOKEN_BUDGET` | Override default token budget |
| `RECALLBENCH_GEN_MODEL` | Override default generation model |
| `RECALLBENCH_JUDGE_MODEL` | Override default judge model |
| `RUST_LOG` | Control log verbosity (e.g., `RUST_LOG=debug`) |

## License

MIT OR Apache-2.0
