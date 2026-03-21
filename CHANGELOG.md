# Changelog

## 0.2.0 (2026-03-20)

Production-ready release. All alpha gaps resolved.

### Added
- `compare` subcommand — run multiple systems side-by-side with comparative tables
- `lib.rs` library target — external crates can now depend on recallbench to implement MemorySystem
- Custom/local OpenAI-compatible endpoint resolution via `--gen-model custom` and `--gen-model local`
- Stage-level checkpointing (`.checkpoints/` directory) for resuming from any pipeline stage
- Named adapter crates: recallbench-mindcore, recallbench-mem0, recallbench-omega, recallbench-zep
- `create_llm_client()` helper resolving provider from model string and config

### Fixed
- `compare` subcommand was a stub — now fully functional
- Custom endpoint config parsing (replaced broken `serde(flatten)` with explicit `custom`/`local` fields)
- Dead code warnings suppressed properly

## 0.1.0 (2026-03-20)

Initial release.

### Features

- Multi-system benchmarking via MemorySystem trait, HTTP adapter, and subprocess adapter
- 7 dataset parsers: LongMemEval, LoCoMo, ConvoMem, MemBench, MemoryAgentBench, HaluMem, custom JSON
- Multi-provider LLM support: Claude, ChatGPT, Gemini, Codex with CLI subscription and API modes
- OpenAI-compatible custom endpoints for local inference (Ollama, vLLM) and cloud (DeepInfra, Together)
- Quick mode with stratified sampling for fast iteration during development
- Type-specific judge prompts following LongMemEval methodology
- Dual-model judging with tiebreaker and calibration suite
- Accuracy metrics: per-type, task-averaged, overall, abstention
- Latency profiling: p50/p95/p99 per pipeline stage
- Token cost tracking with configurable pricing
- 4 report formats: terminal table, Markdown, JSON, CSV
- Failure analysis with per-type grouping
- Longitudinal degradation testing
- Question-level resume with JSONL checkpoints
- Web UI for browsing results locally
- Configurable concurrency and rate limiting
- Full CLI with 12 subcommands
