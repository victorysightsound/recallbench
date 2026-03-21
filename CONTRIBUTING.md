# Contributing to RecallBench

## Project Structure

```
recallbench/
├── Cargo.toml                  # Workspace root
├── recallbench/                # Main binary crate
│   └── src/
│       ├── main.rs             # CLI entry point (clap)
│       ├── traits.rs           # MemorySystem, LLMClient, BenchmarkDataset traits
│       ├── types.rs            # BenchmarkQuestion, EvalResult, Turn, etc.
│       ├── config.rs           # recallbench.toml parsing
│       ├── runner.rs           # Benchmark orchestration engine
│       ├── resume.rs           # JSONL checkpoint/resume
│       ├── sampling.rs         # Stratified random sampling (--quick mode)
│       ├── longevity.rs        # Longitudinal degradation testing
│       ├── datasets/           # Dataset parsers (LongMemEval, LoCoMo, etc.)
│       ├── judge/              # LLM judge system with type-specific prompts
│       ├── llm/                # LLM providers (Claude, ChatGPT, Gemini, Codex, compatible)
│       ├── metrics/            # Accuracy, latency, cost computation
│       ├── report/             # Output formats (table, markdown, JSON, CSV, failure)
│       ├── systems/            # Memory system adapters (echo, HTTP, subprocess)
│       └── web/                # Web UI (axum server, static assets)
├── specs/                      # PRD and design documents
└── .github/workflows/          # CI configuration
```

## Adding a Dataset

1. Create `recallbench/src/datasets/yourformat.rs`
2. Implement a struct with a `from_json(json: &str) -> Result<Self>` constructor
3. Parse the dataset's format and normalize into `BenchmarkQuestion` structs
4. Implement the `BenchmarkDataset` trait
5. Add `pub mod yourformat;` to `recallbench/src/datasets/mod.rs`
6. Register in `DatasetRegistry::register_defaults()`
7. Add unit tests with fixture JSON data

Reference: `recallbench/src/datasets/longmemeval.rs` for a complete example.

## Adding a System Adapter

### Native Rust adapter

1. Create `adapters/recallbench-yoursystem/` as a separate workspace crate
2. Implement the `MemorySystem` trait from `recallbench/src/traits.rs`
3. Reference `recallbench/src/systems/echo.rs` for the trait contract

### HTTP adapter (any language)

Create a TOML config pointing to your system's REST endpoints:

```toml
name = "my-system"
version = "1.0"

[endpoints]
reset = "http://localhost:8080/reset"
ingest = "http://localhost:8080/ingest"
retrieve = "http://localhost:8080/retrieve"
```

Your system must accept:
- `POST /reset` — clear all state, return 200
- `POST /ingest` — JSON body with `{"session": {...}}`, return `IngestStats` JSON
- `POST /retrieve` — JSON body with `{"query": "...", "query_date": null, "token_budget": 16384}`, return `RetrievalResult` JSON

### Subprocess adapter

Create a TOML config with CLI commands:

```toml
name = "my-cli-system"
version = "1.0"

[commands]
reset = ["my-system", "reset"]
ingest = ["my-system", "ingest"]      # receives session JSON on stdin
retrieve = ["my-system", "retrieve"]  # receives query JSON on stdin, outputs RetrievalResult JSON on stdout
```

## Adding an LLM Provider

1. Create `recallbench/src/llm/yourprovider.rs`
2. Implement the `LLMClient` trait
3. Add `pub mod yourprovider;` to `recallbench/src/llm/mod.rs`
4. Wire into `LLMRegistry::resolve_provider()` and `create_client()`

For OpenAI-compatible endpoints, use the existing `compatible.rs` client with a custom config section instead.

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy --workspace` — should pass (dead_code warnings are suppressed)
- Write unit tests for all new logic
- Keep dependencies minimal — prefer direct HTTP over SDK crates
- Use `anyhow` for error handling in the binary crate

## Testing

```bash
# Run all tests
cargo test --workspace

# Run clippy
cargo clippy --workspace

# Check formatting
cargo fmt --check

# Run a specific test
cargo test sampling::tests::maintains_proportions
```

## Commit Messages

- Short and descriptive, under 72 characters
- Lead with the action: "Add", "Fix", "Refactor", "Update", "Remove"
- No AI attribution in commit messages
