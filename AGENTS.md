# Project: recallbench

## On Entry (MANDATORY)

```bash
session-context
```

---

## DIAL — Autonomous Development Loop

This project uses **DIAL** (Deterministic Iterative Agent Loop) for autonomous development.

### Get Full Instructions

```bash
sqlite3 ~/projects/dial/dial_guide.db "SELECT content FROM sections WHERE section_id LIKE '2.%' ORDER BY sort_order;"
```

### Quick Reference

```bash
dial status           # Current state
dial task list        # Show pending tasks
dial task next        # Show next task
dial iterate          # Start next task, get context
dial validate         # Run tests, commit on success
dial learn "text" -c category  # Record a learning
dial stats            # Statistics dashboard
```

### The DIAL Loop

1. `dial iterate` → Get task + context
2. Implement (one task only, no placeholders, search before creating)
3. `dial validate` → Test and commit
4. On success: next task. On failure: retry (max 3).

### Configuration

```bash
dial config set build_cmd "your build command"
dial config set test_cmd "your test command"
```
