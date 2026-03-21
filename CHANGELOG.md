# Changelog

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
