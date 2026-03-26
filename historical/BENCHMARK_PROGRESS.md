# LongMemEval Benchmark Progress

> Historical archive. This file captures the original femind benchmark investigation that was later imported into RecallBench.

Tracking Femind's performance on the LongMemEval Oracle dataset (500 questions).

## Competitive Landscape

No official LongMemEval leaderboard exists. All scores are self-reported. The LongMemEval GitHub repo (xiaowu0162/LongMemEval) and project page (xiaowu0162.github.io/long-mem-eval/) describe the benchmark but maintain no external results board.

### Comparison Table (Task-Averaged Accuracy)

> **Last verified: 2026-03-20.** Competitor scores are scraped from their published pages. Re-verify before citing — these projects update frequently and scores may change.

Industry convention is to report **task-averaged** accuracy (mean of per-category scores). Raw overall (correct/total) is also shown for transparency.

| System | Task-Averaged | Raw Overall | Gen Model | Judge | Dataset | Measured | Source |
|--------|---------------|-------------|-----------|-------|---------|----------|--------|
| **Femind v3** | **95.5%** | **95.6% (478/500)** | **Sonnet** | **Sonnet** | **Oracle** | **2026-03-20** | This repo |
| OMEGA | 95.4% | 93.2% (466/500) | GPT-4.1 | GPT-4o | S | ~2026-02 | dev.to/singularityjason |
| Mastra OM | 94.87% | 93.6% (468/500) | gpt-5-mini | GPT-4o | S | 2026-02-09 | mastra.ai/research/observational-memory |
| Mastra OM | 93.27% | ~92.8% | Gemini 3 Pro | GPT-4o | S | 2026-02-09 | mastra.ai/research/observational-memory |
| Hindsight | ~91.1% | 91.4% (457/500) | Gemini 3 Pro | GPT-4o | S | ~2026-02 | hindsight-benchmarks.vercel.app |
| Mastra OM | 84.23% | 84.8% (424/500) | GPT-4o | GPT-4o | S | 2026-02-09 | mastra.ai/research/observational-memory |
| Emergence | — | 86.0% | GPT-4o | GPT-4o | S | 2025-06-18 | emergence.ai/blog |
| Supermemory | — | 85.2% | Gemini 3 Pro | — | S | ~2026-01 | Reported by Mastra |
| Oracle GPT-4o | — | 82.4% | GPT-4o | GPT-4o | S | 2024-10 | LongMemEval paper (ICLR 2025) |
| OMEGA (repo) | — | 76.8% (384/500) | GPT-4.1 | GPT-4o | S | older ver. | omega-memory/core docs/benchmark-report.md |
| Zep/Graphiti | — | 71.2% | GPT-4o | GPT-4o | S | ~2025 | Reported by multiple sources |
| Full-context GPT-4o | — | 60.2-63.8% | GPT-4o | GPT-4o | S | 2024-10 | LongMemEval paper / Emergence |

### Key Caveats for Fair Comparison

1. **Dataset variant matters.** Femind ran on **Oracle** (only evidence sessions, no distractors). All competitors ran on **LongMemEval_S** (~40 sessions including distractors, ~115K tokens). Oracle is easier — all provided context is relevant, no retrieval noise. A direct comparison requires running Femind on LongMemEval_S.

2. **Generation model matters enormously.** Mastra's scores jump from 84.23% (gpt-4o) to 94.87% (gpt-5-mini) — an 11-point swing from the generation model alone, same memory system. Femind uses Sonnet. Scores are not purely a measure of the memory system.

3. **Judge model matters.** Femind uses Sonnet as judge; most competitors use GPT-4o (the LongMemEval standard). Different judges may score the same response differently.

4. **Iteration count.** OMEGA reports their 95.4% as "best run" after ~8 iterations targeting failure modes. Femind ran 3 iterations (v1/v2/v3). Mastra does not report iteration count.

5. **OMEGA has two scores.** Their marketing blog (dev.to) reports 95.4% task-averaged. Their own GitHub repo `docs/benchmark-report.md` still shows 76.8% from an older system version. The 76.8% has not been retracted — the repo docs simply haven't been updated to reflect the newer score.

### Per-Category Comparison (Femind v3 vs OMEGA 95.4% claim, as of 2026-03-20)

| Category | OMEGA (95.4% run) | Femind v3 |
|----------|-------------------|-------------|
| Single-Session (combined) | 99.2% (125/126) | 98.4% (124/126) |
| Multi-Session | 83.5% (111/133) | 91.0% (121/133) |
| Temporal Reasoning | 94.0% (125/133) | 97.7% (130/133) |
| Knowledge Update | 96.2% (75/78) | 97.4% (76/78) |
| Preference | 100% (30/30) | 90.0% (27/30) |

OMEGA's strength is preference (100%) — Femind's weakness. Femind leads on multi-session (+7.5%), temporal (+3.7%), and knowledge-update (+1.2%).

### Next Step: LongMemEval_S Evaluation

To make a direct apples-to-apples comparison with competitors, Femind needs to be evaluated on **LongMemEval_S** (the variant all competitors use). This involves:
- ~40 sessions per question (vs Oracle's minimal evidence sessions)
- ~115K tokens of context (requires actual retrieval, not full-context inclusion)
- Distractor sessions that are irrelevant to the question

This will be done via the recallbench project (`~/projects/recallbench`), which ports the prompt engineering and evaluation methodology from the original in-repo femind harness.

### Sources (Accessed 2026-03-20)

| Source | URL | What It Provides |
|--------|-----|-----------------|
| LongMemEval paper | arxiv.org/abs/2410.10813 | Benchmark definition, paper baselines |
| LongMemEval project page | xiaowu0162.github.io/long-mem-eval/ | No leaderboard, methodology only |
| LongMemEval GitHub | github.com/xiaowu0162/LongMemEval | Dataset, evaluation scripts |
| OMEGA marketing blog | dev.to/singularityjason | 95.4% claim, methodology details, per-category breakdown |
| OMEGA benchmarks page | omegamax.co/benchmarks | Same 95.4% claim, leaderboard visual |
| OMEGA GitHub repo | github.com/omega-memory/core | 76.8% in docs/benchmark-report.md (older version) |
| OMEGA MemoryStress blog | omegamax.co/blog/why-we-built-memorystress | Acknowledges multiple scores exist |
| Mastra research page | mastra.ai/research/observational-memory | 94.87% (gpt-5-mini), 84.23% (gpt-4o), full leaderboard |
| Hindsight benchmarks | hindsight-benchmarks.vercel.app/longmemeval | 91.4%, per-question results |
| Emergence AI blog | emergence.ai/blog/sota-on-longmemeval-with-rag | 86%, detailed methodology |

---

## Score Summary

| Metric | v1 (2026-03-20) | v2 (2026-03-20) | v3 (2026-03-21) |
|--------|-----------------|-----------------|-----------------|
| Overall Accuracy | 87.0% (435/500) | 94.8% (474/500) | 95.6% (478/500) |
| Task-Averaged Accuracy | 81.9% | 93.8% | 95.5% |
| Abstention Accuracy | 90.0% (27/30) | 90.0% (27/30) | 96.7% (29/30) |
| Failures | 65 | 26 | 22 |

## Per-Type Breakdown

| Category | Count | v1 | v2 | v3 |
|----------|-------|-----|-----|-----|
| Temporal Reasoning | 133 | 121 (91.0%) | 129 (97.0%) | 130 (97.7%) |
| Knowledge Update | 78 | 71 (91.0%) | 74 (94.9%) | 76 (97.4%) |
| Multi-Session | 133 | 114 (85.7%) | 121 (91.0%) | 121 (91.0%) |
| Single-Session (User) | 70 | 66 (94.3%) | 70 (100%) | 69 (98.6%) |
| Single-Session (Assistant) | 56 | 52 (92.9%) | 56 (100%) | 55 (98.2%) |
| Single-Session (Preference) | 30 | 11 (36.7%) | 24 (80.0%) | 27 (90.0%) |

---

## v1 Run (2026-03-20)

**Configuration:**
- Generation: Claude Sonnet via `claude --print`
- Judge: Claude Haiku via `claude --print`
- Context budget: 16,384 tokens
- Prompts: Single generic prompt for all question types
- Results file: `results/longmemeval_full.jsonl`

**Root cause analysis of 65 failures:**

| Root Cause | Count |
|------------|-------|
| Prompt format mismatch (preference) | 19 |
| Reasoning/computation errors | 30 |
| Judge too strict (haiku) | 13 |
| Context truncation | 1 |
| Abstention misses | 3 |

### v1 Failures by Type

**Temporal Reasoning (12 wrong):**
- a3838d2b, gpt4_483dd43c, gpt4_a1b77f9c, gpt4_7abb270c, 370a8ff4, gpt4_d6585ce8, gpt4_21adecb5, gpt4_7bc6cf22, 71017277, b46e15ee, gpt4_d6585ce9, gpt4_fa19884d

**Knowledge Update (7 wrong):**
- 6a1eabeb, 852ce960, f9e8c073, b6019101, dad224aa, 031748ae_abs, 07741c45

**Multi-Session (19 wrong):**
- 0a995998, gpt4_59c863d7, 46a3abf7, 28dc39ac, gpt4_2f8be40d, 88432d0a, gpt4_7fce9456, 7024f17c, gpt4_31ff4165, gpt4_194be4b3, gpt4_ab202e7f, e3038f8c, 7405e8b1, 9ee3ecd6, ba358f49, 09ba9854, 37f165cf, a96c20ee_abs, 09ba9854_abs

**Single-Session User (4 wrong):**
- 51a45a95, b86304ba, ec81a493, 8a137a7f

**Single-Session Assistant (4 wrong):**
- 8752c811, 3249768e, eaca4986, 778164c6

**Single-Session Preference (19 wrong):**
- 8a2466db, 75832dbd, 0edc2aef, 35a27287, afdc33df, caf03d32, 54026fce, 09d032c9, 57f827a0, 505af2f5, 75f70248, d6233ab6, 1da05512, b6025781, a89d7624, b0479f84, 1d4e3b97, 07b6f563, 1c0ddc50

---

## v2 Run (2026-03-20)

**Configuration:**
- Generation: Claude Sonnet via `claude --print`
- Judge: Claude Sonnet via `claude --print` (upgraded from haiku)
- Context budget: Unlimited (0)
- Prompts: Type-specific generation prompts
- Results file: `results/longmemeval_v2.jsonl`

**Changes from v1:**
1. Type-specific generation prompts (preference format, temporal enumeration, knowledge-update recency, multi-session enumeration)
2. Judge upgraded from haiku to sonnet with extraction-aware prompts
3. Context budget set to unlimited for oracle dataset
4. Better abstention detection in generation prompt

**Question-level diff (v1 → v2):**
- Fixed (were wrong, now correct): 49
- Regressed (were correct, now wrong): 10
- Net improvement: +39

### v2 Regressions (10 questions correct in v1, wrong in v2)

| Question ID | Type | Ground Truth |
|------------|------|-------------|
| 07741c44 | knowledge-update | under my bed |
| 6b7dfb22 | single-session-preference | build upon existing sources of inspiration (Instagram art accounts, online tutorials) |
| 6d550036 | multi-session | 2 |
| a2f3aa27 | knowledge-update | 1300 |
| af082822 | temporal-reasoning | 2 |
| c4a1ceb8 | multi-session | 3 |
| f685340e_abs | knowledge-update | (abstention) You mentioned playing tennis but not table tennis |
| gpt4_731e37d7 | multi-session | $720 |
| gpt4_93159ced | temporal-reasoning | 4 years and 9 months |
| gpt4_93159ced_abs | temporal-reasoning | (abstention) You haven't started working at Google yet |

### v2 Remaining Failures (26 questions)

**Multi-Session (12 wrong):**

| Question ID | Ground Truth | Notes |
|------------|-------------|-------|
| 0a995998 | 3 | Counting error (store pickups/returns) |
| 6d550036 | 2 | Counting error (teams led) — regression |
| c4a1ceb8 | 3 | Counting error (citrus fruits) — regression |
| 46a3abf7 | 3 | Counting error (fish tanks) |
| gpt4_2f8be40d | 3 weddings (Rachel/Mike, Emily/Sarah, Jen/Tom) | Enumeration error |
| 7024f17c | 0.5 hours | Wrong time computation |
| gpt4_731e37d7 | $720 | Cost computation — regression |
| e3038f8c | 99 | Sum of rare items |
| 9ee3ecd6 | 100 | Points computation |
| 37f165cf | 856 | Page count sum |
| 09ba9854 | $50 | Cost difference |
| 09ba9854_abs | (abstention) bus cost not mentioned | Model answered instead of abstaining |

**Single-Session Preference (6 wrong):**

| Question ID | Core Preference Missed |
|------------|----------------------|
| afdc33df | Kitchen organization (utensil holder, countertop tips) |
| caf03d32 | Slow cooker tips (beef stew success, yogurt interest) |
| 6b7dfb22 | Art inspiration sources (Instagram, online tutorials) — regression |
| 09d032c9 | Power bank optimization tips |
| 1da05512 | NAS device for home network storage issues |
| 1c0ddc50 | Podcasts/audiobooks beyond true crime, especially history |

**Knowledge Update (4 wrong):**

| Question ID | Ground Truth | Notes |
|------------|-------------|-------|
| 07741c44 | under my bed | Model picked later session (closet) — regression |
| 07741c45 | in a shoe rack in my closet | Model picked earlier session (bed) |
| a2f3aa27 | 1300 | User said "close to 1300", GT expects exact 1300 — regression |
| f685340e_abs | (abstention) tennis not table tennis | Model abstained correctly but judge said no — regression |

**Temporal Reasoning (4 wrong):**

| Question ID | Ground Truth | Notes |
|------------|-------------|-------|
| gpt4_93159ced | 4 years and 9 months | Model quotes "4 years and 3 months" from one session, misses update — regression |
| gpt4_93159ced_abs | (abstention) haven't started at Google | Model abstained correctly but judge said no — regression |
| af082822 | 2 (weeks) | Model computed 13 days, didn't convert to weeks — regression |
| 370a8ff4 | 15 | Date counting error between Jan 19 and Apr 10 |

---

## v3 Changes

**Three targeted fixes:**

### 1. Self-Verification Pass (new: `verify.rs`)
Second LLM call after generation for multi-session and knowledge-update questions. Asks model to re-check counting, arithmetic, and version selection using the original chat history as reference. Skips temporal reasoning (causes regressions), single-session, and preference types. Skips abstention questions.

### 2. Preference Few-Shot Examples (updated: `retrieval.rs`)
Two concrete examples showing content-focused preference descriptions vs. format-focused descriptions. Explicitly tells model that "BAD answers describe formatting preferences."

### 3. Lenient Abstention Judging (updated: `judge.rs`)
Judge now accepts abstention responses that explain WHY they can't answer and what IS in the chat history, as long as the primary conclusion is that they cannot answer the specific question.

---

## v3 Run (2026-03-20) — Two Attempts

### v3-draft (with temporal verification) — FAILED

**Configuration:**
- Generation: Claude Sonnet | Judge: Claude Sonnet
- Context budget: Unlimited
- Verification: multi-session, temporal, knowledge-update
- Results file: `results/longmemeval_v3-draft.jsonl`

**Result: 92.4% (462/500)** — regression from v2's 94.8%.

The verifier without context (initial attempt) hallucinated that session references were "fabricated" and rejected correct knowledge-update answers (69.2% KU, down from 94.9%).

After fixing to include context, temporal verification still caused 20 regressions — the verifier re-did date calculations from scratch and arrived at different (wrong) answers, overwriting correct ones. Temporal dropped from 97.0% to 83.5%.

**Lesson: Self-verification hurts categories where the initial answer is already strong. Only apply verification where it demonstrably helps.**

### v3 (without temporal verification) — CURRENT BEST

**Configuration:**
- Generation: Claude Sonnet | Judge: Claude Sonnet
- Context budget: Unlimited
- Verification: multi-session and knowledge-update only (temporal excluded)
- Preference: few-shot examples for content vs format preferences
- Abstention: lenient judging that accepts explanatory context
- Results file: `results/longmemeval_v3.jsonl`

**Result: 95.6% (478/500), Task-Averaged: 95.5%**

**v2 → v3 diff:**
- Fixed (were wrong, now correct): improvements in KU (+2), preference (+3), abstention (+2)
- Regressed: single-session user (-1), single-session assistant (-1) — nondeterministic model variation
- Multi-session: held at 91.0% (verification didn't help or hurt net)
- Net improvement: +4 questions correct

### v3 Remaining Failures (22 questions)

**Multi-Session (12 wrong):**
Mostly counting/enumeration errors where the model has the right items but computes the wrong total. Self-verification was expected to help here but had negligible net effect — it fixed some and regressed others.

**Single-Session Preference (3 wrong):**
Model still describes format preferences in some cases despite few-shot examples. The remaining failures involve nuanced topic inference.

**Temporal Reasoning (3 wrong):**
Edge cases: unit conversion (days→weeks), ambiguous date references, complex multi-step date arithmetic.

**Knowledge Update (2 wrong):**
Ambiguous "most recent" determination and rounding ("close to 1300" vs "1300").

**Single-Session User (1 wrong), Assistant (1 wrong):**
Nondeterministic variation — these were correct in v2.

---

## Gap to 97% (Target: 485/500)

Currently at 478/500. Need to recover 7 more. Remaining levers:

1. **Multi-session counting** (12 wrong) — biggest pool. Could try structured output format forcing the model to output a numbered list, then programmatically count items.
2. **Preference specificity** (3 wrong) — more targeted few-shot examples or chain-of-thought that first lists specific topics from the conversation.
3. **Run variance** — single-session user/assistant regressions are nondeterministic. A retry or best-of-N approach could recover 2.
4. **Deterministic answer extraction** — instead of judging the full step-by-step response, extract just the final answer and judge that.

---

## Future: Core Femind Improvements

The benchmark revealed three areas where Femind's core engine needs improvement for real-world usage (documented, not yet implemented):

1. **Preference memory tagging** — Detect and tag user preferences during ingestion as a distinct memory type with higher retrieval priority. (`src/ingest/`)

2. **Temporal-aware retrieval** — Date-biased scoring in context assembly. Boost memories closer to query date for recency queries. (`src/search/`, `src/context/`)

3. **Vector embeddings for semantic search** — Wire Candle embeddings feature into hybrid search (FTS5 + vector similarity + RRF fusion) for retrieval when keyword matching fails. (`src/search/`)

Future benchmark work will be done via the recallbench project (`~/projects/recallbench`), which ports the prompt engineering and evaluation methodology from the original in-repo femind harness.

---

## Prompt Engineering Reference (for recallbench port)

The key prompt improvements that drove v1 (87%) → v3 (95.6%) are documented here for porting to recallbench.

### Generation Prompts (retrieval.rs)

**Base preamble (all types):**
```
I will give you several history chats between a user and an AI assistant.
Based on the chat history, answer the question at the end.

History Chats:
{context}

Current Date: {question_date}
Question: {question}
```

**Type-specific instructions appended after the preamble:**

- **SingleSessionPreference:** Instructs model to describe CONTENT preferences ("The user would prefer responses that..."), not formatting preferences. Includes two few-shot examples (Sony camera accessories, quinoa recipes) and explicitly labels format-focused answers as BAD.

- **TemporalReasoning:** "List EVERY relevant event with its exact date. Then count them explicitly (1, 2, 3...) or compute the date arithmetic step by step. Do not estimate or shortcut."

- **KnowledgeUpdate:** "List ALL versions of the relevant information chronologically with their session dates. Then clearly state the latest/most recent value as your final answer."

- **MultiSession:** "Before giving your final answer, enumerate ALL relevant items/facts from EVERY session. Number each one explicitly. Do not skip any session."

- **Abstention:** "If the chat history does not contain information that DIRECTLY answers this question, you MUST respond with 'I don't know'. Do NOT attempt to infer, extrapolate, or guess."

- **Default (SingleSessionUser, SingleSessionAssistant):** "Extract the relevant information, then provide a concise answer."

### Judge Prompts (judge.rs)

Key improvements over the LongMemEval standard judge:

- **Extraction-aware:** "The model's response may be long and contain step-by-step reasoning. Search the ENTIRE response for the required answer."
- **Equivalent representations:** "Accept equivalent representations (e.g., '3 weeks' vs '21 days', 'two' vs '2')."
- **Multi-item awareness:** "If the required answer is a list of items, verify ALL items are mentioned somewhere in the response."
- **Lenient abstention:** Accepts responses that explain WHY they can't answer, as long as the primary conclusion is abstention.
- **Preference-specific:** "Answer 'yes' if the response captures the essential preference, even if phrased differently. Word-for-word match is NOT required."

### Self-Verification (verify.rs)

Applied only to multi-session and knowledge-update (NOT temporal — causes regressions).

Verification prompt includes the full original chat history and asks the model to:
- Multi-session: "re-enumerate every relevant item numbered 1, 2, 3... and recount"
- Knowledge-update: "re-list all versions chronologically and confirm the final one is correct"

If the answer is correct, output it unchanged. If wrong, output only the corrected answer.

---

## Files

| File | Description |
|------|-------------|
| `results/longmemeval_full.jsonl` | v1 raw results (500 questions, deduplicate by question_id) |
| `results/longmemeval_v2.jsonl` | v2 raw results (500 questions) |
| `results/longmemeval_v3-draft.jsonl` | v3 draft attempt (with temporal verification — regression, kept for research) |
| `results/longmemeval_v3.jsonl` | v3 final results (current best — 95.6%) |
| `results/bench.log` | v1 runtime log |
| `results/bench_v2.log` | v2 runtime log |
| `results/bench_v3.log` | v3 runtime log |
| `results/bench_v3b.log` | v3b runtime log |
