# Project: recallbench

## On Entry (MANDATORY)

```bash
session-context
```

---

## Project Overview

**RecallBench** is a standalone benchmark application for AI memory systems. It evaluates native Rust memory engines, HTTP-backed systems, and subprocess-driven systems against shared datasets with reproducible result artifacts and comparative reporting.

**Status:** active Rust workspace with a repo-local documentation database and a retained historical benchmark archive.

---

## Key Files

| File | Purpose |
|------|---------|
| `README.md` | Public project overview and usage |
| `CONTRIBUTING.md` | Maintainer workflow, workspace layout, and test guidance |
| `specs/PRD.md` | Main architecture and product spec |
| `.docs/recallbench_spec.db` | Authoritative repo-local architecture and planning database |
| `historical/` | Archived benchmark history and imported legacy runs |

---

## Documentation Database

Primary local source of truth:
```bash
sqlite3 .docs/recallbench_spec.db "SELECT section_id, title FROM sections ORDER BY sort_order;"
```

The active Markdown docs in this repo should stay aligned with `.docs/recallbench_spec.db`. Files under `historical/` are archival context, not the primary spec.

---

## Development Workflow

- Work from the current spec, active task list, and workspace build/test commands.
- Prefer local compile, lint, and non-network validation first.
- Do not run live benchmark paths that depend on real CLI/API LLM calls without explicit user approval.

---

## DIAL

This repo keeps a DIAL project history in `.dial/foundation.db`.

### Quick Reference

```bash
sqlite3 .dial/foundation.db "SELECT key, value FROM config ORDER BY key;"
sqlite3 .dial/foundation.db "SELECT id, description, status FROM tasks ORDER BY id LIMIT 20;"
```

### Build/Test Defaults

```bash
cargo build --workspace
cargo test --workspace
cargo check --workspace
```

---

## External-Facing Writing

- Keep README files, specs, changelogs, commit messages, PR text, and code comments in normal developer voice.
- Do not describe implementation work in terms of agent runs, autonomous loops, model names, or internal AI workflow mechanics.
- Mention AI, LLMs, datasets, judging, or orchestration only when they are part of the actual RecallBench product surface being documented.
