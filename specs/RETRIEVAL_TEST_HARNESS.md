# Retrieval Test Harness — Local Iteration Without LLM Costs

## Goal

Build a local test harness in recallbench that measures retrieval quality
directly — no LLM generation, no judging, no API costs. Enables rapid
iteration on femind's search pipeline parameters (RRF weights, chunk size,
reranker settings, diversification) with instant feedback.

## Design

### Core Concept

For each question in the dataset, we know:
- The haystack sessions (40+ sessions with distractors)
- The answer session IDs (which sessions contain the answer)
- The ground truth answer text

The harness:
1. Loads all haystack sessions into femind
2. Runs the search pipeline with the question text
3. Checks if chunks from the answer sessions appear in the top-N results
4. Reports retrieval metrics without calling any LLM

### Metrics

- **Recall@K**: % of answer sessions with at least one chunk in top-K results
- **MRR (Mean Reciprocal Rank)**: average 1/rank of first answer chunk
- **Coverage**: % of answer session chunks that appear anywhere in results
- **Hit Rate**: % of questions where ALL answer sessions are represented

### Implementation

New subcommand: `recallbench retrieval-test`

```
recallbench retrieval-test \
  --system femind-api \
  --variant small \
  --filter multi-session \
  --quick --quick-size 20 \
  --budget 16384
```

Output:
```
Retrieval Quality — longmemeval small (multi-session, 20 questions)
═══════════════════════════════════════════════════════════════════
Recall@10:  45.0%   (answer sessions in top 10 results)
Recall@20:  62.0%
Recall@50:  78.0%
Recall@100: 89.0%
MRR:        0.31    (first answer chunk at avg rank 3.2)
Hit Rate:   35.0%   (all answer sessions found for 7/20 questions)

Per-question detail:
  Q c4a1ceb8: answer sessions [s1, s5, s12] → found [s1✓@3, s5✓@8, s12✗]
  Q 6d550036: answer sessions [s3, s7] → found [s3✓@1, s7✓@15]
  ...
```

### Key Features

1. **No LLM needed** — pure retrieval measurement
2. **Instant feedback** — runs in seconds, not hours
3. **Parameter sweeps** — test multiple budget/chunk/RRF configs in one run
4. **Uses embedding cache** — no re-embedding cost
5. **Exports failure details** — which answer sessions are missing and at what rank
6. **Supports all question types** — not just multi-session

### Files

- `recallbench/src/retrieval_test.rs` — core harness logic
- CLI integration in `main.rs` — new `retrieval-test` subcommand
- Uses existing embedding cache, dataset loading, and MemorySystem trait

## Success Criteria

1. `recallbench retrieval-test` runs without LLM calls
2. Reports Recall@K, MRR, Hit Rate per question type
3. Shows per-question detail for failures
4. Runs full 133 multi-session questions in under 5 minutes
5. Supports --budget flag for testing different context sizes
6. Works with embedding cache (no re-embedding)
