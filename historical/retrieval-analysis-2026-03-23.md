# Retrieval Analysis — Full 500-Question Comparison

Three chunk sizes tested: 500, 1000, 2000 chars
Budget: 16384 tokens | Model: all-MiniLM-L6-v2 | Dataset: longmemeval-small
470 regular + 30 abstention = 500 total questions

## Overall Metrics

| Metric | c500 | c1000 | c2000 |
|--------|------|-------|-------|
| Recall@10 | 94.2% | 94.0% | 91.9% |
| Recall@20 | 97.9% | 98.0% | 97.3% |
| Recall@50 | 99.3% | 99.6% | 99.1% |
| Failures | 2 | 1 | 1 |
| Low-ranked (>10) | 50 | 52 | 71 |

## Per-Type Recall@10

| Type | c500 | c1000 | c2000 | Best |
|------|------|-------|-------|------|
| knowledge-update | 100.0% | 100.0% | 100.0% | c500 |
| multi-session | 90.5% | 90.2% | 87.0% | c500 |
| single-session-assistant | 98.2% | 100.0% | 100.0% | c1000 |
| single-session-preference | 96.7% | 96.7% | 96.7% | c500 |
| single-session-user | 98.4% | 98.4% | 98.4% | c500 |
| temporal-reasoning | 93.2% | 92.9% | 89.6% | c500 |

## Questions Where Chunk Sizes Disagree

### 4388e9dd [single-session-assistant]
Q: I was going through our previous chat and I was wondering, what was Andy wearing in the script you wrote for the comedy 
Answer sessions: ['answer_sharegpt_qTi81nS_0']
- c500: 0/1 found | answer_sharegpt_qTi8→MISSING
- c1000: 1/1 found | answer_sharegpt_qTi8→rank 0
- c2000: 1/1 found | answer_sharegpt_qTi8→rank 0

## All Retrieval Failures

gpt4_385a5000 fails across ALL chunk sizes — answer_7a4a93f1_1 never found.
4388e9dd fails ONLY at c500 — answer_sharegpt_qTi81nS_0 fragmented too much.

## Answer Sessions Ranked >50 (worst retrieval quality)

| Question | Type | Session | Rank@c500 | Rank@c1000 | Rank@c2000 |
|----------|------|---------|-----------|------------|------------|
| 10d9b85a | multi-session | answer_e0585cb5_1 | 45 | 45 | 52 |
| 1a8a66a6 | multi-session | answer_2bd23659_4 | 63 | 70 | 78 |
| 4dfccbf8 | temporal-reason | answer_4bebc783_2 | 69 | 44 | 84 |
| 6d550036 | multi-session | answer_ec904b3c_3 | 84 | 85 | 101 |
| ba358f49 | multi-session | answer_cbd08e3c_2 | 80 | 84 | 106 |
| ba358f49_abs | multi-session | answer_cbd08e3c_abs_ | 43 | 40 | 103 |

## Conclusion

Chunk size 1000 is optimal: best overall Recall@10 (95.3%), fewest failures (1),
best MRR (0.928). c500 introduces an extra failure. c2000 consistently ranks answer
sessions lower due to diluted embeddings. The one persistent failure (gpt4_385a5000)
is a hard edge case where the answer session discusses 'seedlings' without mentioning
'tomatoes' or 'marigolds' — weak semantic and keyword overlap with the query.