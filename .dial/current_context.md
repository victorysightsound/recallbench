# Task: GAP P1.1: Implement compare subcommand — run multiple systems sequentially against same dataset, collect per-system results, output comparative tables, support --format and --quick

## ⚠️ SIGNS (Critical Rules)


- **ONE TASK ONLY: Complete exactly this task. No scope creep.**

- **SEARCH BEFORE CREATE: Always search for existing files/functions before creating new ones.**

- **NO PLACEHOLDERS: Every implementation must be complete. No TODO, FIXME, or stub code.**

- **VALIDATE BEFORE DONE: Run `dial validate` after implementing. Don't mark complete without testing.**

- **RECORD LEARNINGS: After success, capture what you learned with `dial learn "..." -c category`.**

- **FAIL FAST: If blocked or confused, stop and ask rather than guessing.**



## Project Learnings (apply these patterns)


- [other] LLM providers must support BOTH CLI subscription mode (default, no API key) and direct HTTP API mode. Claude Code CLI (claude --print) is the default provider. User selects mode via recallbench.toml config. Supported CLIs: claude, chatgpt, codex, gemini.

- [other] NEVER auto-publish to crates.io. Task 95 (publish) requires manual user action. Notify user and stop.

- [other] MindCore crate is at ~/projects/mindcore and builds successfully. Link via path dependency: mindcore = { path = "../mindcore" }

- [other] The mindcore-bench harness (~/projects/mindcore/mindcore-bench/) is the reference implementation. Port patterns from there but generalize for multi-system support.

- [other] Gap fix round: 19 tasks (102-120) to resolve all alpha gaps. Phase 1=critical stubs, Phase 2=named adapters, Phase 3=missing features, Phase 4=dataset validation, Phase 5=integration tests, Phase 6=error hardening, Phase 7=web UI polish, Phase 8=smoke test and publish.