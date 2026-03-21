# Changelog

## 0.4.0 (2026-03-21)

All remaining enhancements implemented. No known gaps.

### Added
- `calibrate` subcommand fully functional with 50 pre-scored calibration pairs (`calibration/longmemeval_50.json`)
- `--stress N` flag: run benchmark N times, report mean/variance/stddev/min/max accuracy
- `--budget-sweep` flag: run at budgets 4096/8192/16384/32768, report accuracy vs. budget curve
- Web UI: Compare view — side-by-side system comparison with per-type accuracy diff
- Web UI: Longevity view — SVG accuracy and latency charts from longevity JSON results
- Web UI: Compare and Longevity navigation buttons in navbar

## 0.3.0 (2026-03-21)

All dataset parsers rewritten against real schemas and validated with actual downloaded data.

### Added
- Download support for LoCoMo, ConvoMem (Salesforce), MemBench, HaluMem — all auto-download via `recallbench download`
- Category aliases: `--dataset recall` (LongMemEval), `--dataset longturn` (LoCoMo), `--dataset conversation` (ConvoMem), `--dataset multiaspect` (MemBench), `--dataset hallucination` (HaluMem)
- MemBench variants: simple, aggregative, comparative, conditional, knowledge_update, highlevel
- ConvoMem variants: user_evidence, assistant_facts, changing, abstention, preference, implicit_connection
- HaluMem variants: medium (33MB), long (107MB)

### Fixed
- LoCoMo parser rewritten for real schema (session_N keys, speaker names, integer answers, missing answer fields)
- ConvoMem parser rewritten for Salesforce evidenceItems format with 6 evidence categories
- MemBench parser rewritten for roles/message_list/QA multiple-choice format
- HaluMem parser rewritten for JSONL format with uuid, persona_info, sessions, memory_points, questions
- MemoryAgentBench documented as Parquet-only with export instructions

### Validated
- LoCoMo: 1,986 QA pairs, 5 categories (single-hop, temporal, multi-hop, open-domain, unanswerable)
- ConvoMem: 413+ questions per category, 6 categories
- MemBench: 500 questions per category, multiple-choice with ground_truth resolution
- HaluMem Medium: 3,467 QA pairs, 6 question types across 20 users

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
