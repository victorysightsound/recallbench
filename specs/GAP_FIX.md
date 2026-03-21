# RecallBench Gap Fix Spec

**Date:** 2026-03-20
**Goal:** Resolve all gaps identified in alpha review. Make recallbench production-ready.

---

## Phase 1: Critical Stubs (High Severity)

These are user-facing features that print "not yet implemented" or are completely missing.

### Task 1: Implement `compare` subcommand
The `compare` command currently prints a stub message. Implement it:
- Accept `--systems` as comma-separated list of system names or config paths
- Run each system sequentially against the same dataset
- Collect results per system
- Output comparative accuracy table, latency table, cost table
- Support all report formats via `--format`
- Support `--quick` mode

### Task 2: Implement `calibrate` subcommand
The `calibrate` command prints a stub. Implement it:
- Create `calibration/longmemeval_50.json` with 50 pre-scored (question, ground_truth, hypothesis, expected_judgment) pairs
- Wire `judge/calibration.rs` (already implemented) into the CLI
- Run judge against all pairs, report per-type accuracy
- Fail with clear message if accuracy < 90%

### Task 3: Wire custom/local endpoints into CLI
`CompatibleClient` exists in `llm/compatible.rs` but isn't wired into `cmd_run()`.
- When `--gen-model` or `--judge-model` matches a config section name (e.g., "custom", "local"), resolve to `CompatibleClient` using that section's `base_url`, `api_key_env`, `model`
- Update `config.rs` to properly parse `[llm.custom]` and `[llm.local]` sections (fix the `#[serde(flatten)]` approach which may conflict with typed fields)
- Add rate limiting wrapper when `rate_limit_rpm > 0`

### Task 4: Add `lib.rs` library target
External crates need to depend on recallbench to implement `MemorySystem`.
- Create `recallbench/src/lib.rs` that re-exports `traits`, `types`, `config`, `systems::echo`
- Update `recallbench/Cargo.toml` to declare both `[[bin]]` and `[lib]` targets
- Verify `use recallbench::traits::MemorySystem` works from an external crate

---

## Phase 2: Named Adapters (High Severity)

### Task 5: Create adapter workspace crates
Create the `adapters/` directory with workspace member crates:
- `adapters/recallbench-mindcore/` — links to `mindcore` crate via path dep, maps ConversationSession to MemoryRecord
- `adapters/recallbench-mem0/` — HTTP adapter for Mem0 REST API
- `adapters/recallbench-omega/` — HTTP adapter for OMEGA MCP API
- `adapters/recallbench-zep/` — HTTP adapter for Zep REST API

Each adapter crate:
- Has its own `Cargo.toml` depending on `recallbench` (lib) + system-specific deps
- Implements `MemorySystem` trait
- Has unit tests with mock servers where appropriate
- Is registered in the CLI system resolution (`cmd_run` match arm)

---

## Phase 3: Missing Advanced Features (Medium Severity)

### Task 6: Stage-level checkpointing
Implement `checkpoint.rs`:
- Save per-question pipeline state after each stage (ingest, retrieve, generate, judge) to `.checkpoints/` directory
- On `--resume`, detect last completed stage per question and resume from next
- Support `--rejudge` flag to re-run judging without re-generating (uses saved hypothesis)
- JSON format per checkpoint file

### Task 7: Stress test mode (`--stress`)
Implement in runner:
- Accept `--stress N` flag — run the same question set N times
- Compute accuracy mean, variance, stddev across runs
- Report judge consistency metrics
- Output per-run breakdown

### Task 8: Budget sweep mode (`--budget-sweep`)
Implement in runner:
- Accept `--budget-sweep` flag with optional budget list (default: 4096,8192,16384,32768)
- Run questions at each budget level
- Report accuracy vs. token budget curve
- Output as table and JSON

### Task 9: Comparative diff
Implement in `report/`:
- After `compare` runs, identify per-question disagreements
- Show which questions system A got right but system B missed (and vice versa)
- Exportable as JSON

---

## Phase 4: Dataset Parser Validation (Medium Severity)

### Task 10: Validate dataset parsers against real data
For each non-LongMemEval parser:
- Research the actual JSON schema from the dataset's GitHub repo
- Download a sample of real data (or create accurate fixture data from the repo's format)
- Fix parser to match actual format
- Add tests with realistic fixture data

Datasets to validate:
- LoCoMo (snap-research/locomo)
- MemBench (import-myself/Membench)
- MemoryAgentBench (HUST-AI-HYZ/MemoryAgentBench)
- HaluMem (MemTensor/HaluMem)
- ConvoMem (supermemoryai/memorybench)

### Task 11: Add dataset download URLs
For datasets with public download links, add URLs to the registry so `recallbench download <name>` works automatically (not just for LongMemEval).

---

## Phase 5: Integration Testing (Medium Severity)

### Task 12: End-to-end integration test with echo adapter
Create `tests/integration/echo_e2e.rs`:
- Create a small fixture LongMemEval dataset (5 questions)
- Run full pipeline: load dataset → run with echo adapter + mock LLM → verify JSONL output → verify resume → verify report generation
- Verify metrics computation produces expected values

### Task 13: CLI integration tests
Test all subcommands with fixture data:
- `recallbench datasets` lists all 7
- `recallbench stats` with fixture data
- `recallbench validate` with valid and invalid custom datasets
- `recallbench report` with fixture JSONL
- `recallbench failures` with fixture JSONL
- `recallbench run --quick` with echo adapter

---

## Phase 6: Error Hardening (Low-Medium Severity)

### Task 14: Audit and fix unwrap/expect calls
- Grep for all `unwrap()` and `expect()` in non-test code
- Replace with proper error propagation (`?`, `context()`, `bail!()`)
- Add timeout handling for LLM CLI calls (default 60s)
- Add timeout handling for HTTP adapter calls (default 30s)
- Ensure per-question failures don't abort the entire run

### Task 15: Edge case handling
Add tests and handling for:
- Empty dataset (0 questions)
- Dataset with only 1 question
- Dataset with no abstention questions (abstention metric should be None)
- System returning empty context
- LLM returning empty string
- LLM returning non-yes/no to judge
- Corrupt JSONL on resume (skip malformed lines)
- Missing results directory on report/failures commands

---

## Phase 7: Web UI Polish (Low Severity)

### Task 16: Comparison view
Implement comparison view in web UI:
- Load comparison results (multiple systems in same file or multiple files)
- Side-by-side accuracy tables
- Per-question disagreement highlighting

### Task 17: Longevity view
Implement longevity view in web UI:
- Load longevity JSON results
- Render accuracy curve as SVG line chart
- Render latency curve as SVG line chart

---

## Phase 8: Final Validation

### Task 18: Full smoke test
Run the complete tool manually:
- `recallbench datasets`
- `recallbench download longmemeval --variant oracle`
- `recallbench run --system echo --dataset longmemeval --variant oracle --quick --quick-size 10` (with mock/echo, no real LLM needed)
- `recallbench report results/*.jsonl`
- `recallbench failures results/*.jsonl`
- `recallbench serve` (verify web UI loads)
- Verify `cargo install recallbench` works from crates.io

### Task 19: Publish v0.2.0
- Update version to 0.2.0 in workspace Cargo.toml
- Update CHANGELOG.md with gap fixes
- `cargo publish -p recallbench`
- Push all changes to GitHub
