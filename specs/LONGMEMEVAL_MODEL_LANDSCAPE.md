# LongMemEval Model Landscape

Last updated: 2026-03-27

This note tracks what public sources actually disclose about the models used by systems that report LongMemEval results. It separates direct-source disclosures from second-hand leaderboard references so RecallBench tuning is grounded in evidence, not vendor paraphrase.

## Scope

- Focus: systems with published LongMemEval or LongMemEval_S results.
- Goal: identify the answer-generation model, judge model, and any disclosed retrieval or embedding details.
- Standard benchmark context: many teams treat `gpt-4o` as the canonical LongMemEval judge because that is the official LLM-as-judge setup used in the benchmark paper and later reproductions.

## Direct-Source Disclosures

### Mastra Observational Memory

Source:
- https://mastra.ai/research/observational-memory
- https://mastra.ai/blog/observational-memory

Disclosed models:
- `gpt-4o`
- `gemini-3-pro-preview`
- `gemini-3-flash-preview`
- `gpt-5-mini`
- observer/reflector lane: `gemini-2.5-flash`

What Mastra says:
- `gpt-4o` is their official benchmark-comparison model.
- `gpt-5-mini` is their best published LongMemEval score at `94.87%`.
- `gemini-3-pro-preview` reaches `93.27%`.
- their research page explicitly lists:
  - actor: `gemini-3-pro-preview`
  - observer / reflector: `gemini-2.5-flash`

Useful implication:
- Mastra is the clearest current public example that the LongMemEval score can move dramatically with the answering model even when the memory architecture stays fixed.

### Supermemory

Source:
- https://supermemory.ai/research/
- https://supermemory.ai/docs/memorybench/supported-models

Disclosed models:
- answer-generation comparisons:
  - `gpt-4o`
  - `gpt-5`
  - `gemini-3-pro`
- judge:
  - `gpt-4o`

Published scores:
- `81.6%` with `gpt-4o`
- `84.6%` with `gpt-5`
- `85.2%` with `gemini-3-pro`

What Supermemory says:
- they used `gpt-4o` for answer evaluation with the question-specific prompts from the LongMemEval paper.
- their docs expose the supported evaluation models used by MemoryBench.

Useful implication:
- Supermemory is a strong direct-source example that answer-model choice matters materially, while the judge remains stable on `gpt-4o`.

### Zep / Graphiti

Source:
- https://blog.getzep.com/content/files/2025/01/ZEP__USING_KNOWLEDGE_GRAPHS_TO_POWER_LLM_AGENT_MEMORY_2025011700.pdf

Disclosed models:
- graph construction: `gpt-4o-mini-2024-07-18`
- response generation:
  - `gpt-4o-mini-2024-07-18`
  - `gpt-4o-2024-11-20`
- LongMemEval judge:
  - `gpt-4o`

Published LongMemEval scores:
- `63.8%` with `gpt-4o-mini`
- `71.2%` with `gpt-4o`

Useful implication:
- Zep is one of the cleanest direct-source references because it discloses graph-construction model, answer model, and judge model separately.

### EmergenceMem

Source:
- https://www.emergence.ai/blog/sota-on-longmemeval-with-rag

Disclosed models:
- default benchmark model, unless otherwise stated:
  - `gpt-4o-2024-08-06`
- one reported baseline:
  - full-context `GPT o3`

Published scores:
- `82.4%` for `EmergenceMem Simple`
- `79.0%` for `EmergenceMem Simple Fast`
- `86.0%` for `EmergenceMem Internal`

What Emergence says:
- unless noted otherwise, the model is `gpt-4o-2024-08-06`
- their simple approach uses `gpt-4o-2024-08-06` to produce a small chain of thought before answering

Useful implication:
- EmergenceMem is still a strong benchmark reference because it keeps the answer model explicit and close to the official `gpt-4o` lane.

### OMEGA

Sources:
- https://omegamax.co/benchmarks
- https://omegamax.co/docs/benchmark-report

Disclosed models and components:
- generation: `GPT-4.1`
- grading / judge: `GPT-4.1` on the `95.4%` benchmark page
- benchmark report also says published scores use `GPT-4o or GPT-4.1` as judge
- embeddings: `bge-small-en-v1.5` ONNX, CPU-only

Published scores:
- `95.4%` on the public benchmarks page
- `76.8%` in the benchmark report

Important conflict:
- OMEGA currently has two different public LongMemEval claims:
  - `95.4%` on the benchmarks page
  - `76.8%` in the benchmark report
- both are recent enough that they cannot be treated as the same run

Useful implication:
- OMEGA is informative for model and embedding disclosure, but its public benchmark reporting is internally inconsistent and should be treated carefully until reconciled.

### Hindsight

Primary source:
- https://arxiv.org/abs/2512.12818

Secondary source:
- https://mastra.ai/research/observational-memory

What the paper directly discloses:
- an open-source `20B` backbone reaches `83.6%`
- a larger scaled backbone reaches `91.4%`

What the paper does not clearly disclose in the abstract view:
- the exact larger model name

What Mastra’s leaderboard claims:
- `Hindsight GPT-OSS 20B` at `83.6%`
- `Hindsight GPT-OSS 120B` at `89.0%`
- `Hindsight gemini-3-pro-preview` at `91.4%`

Useful implication:
- Hindsight is clearly competitive, but the exact model lineage is only partly direct-source. The `20B` result is directly supported; the named larger backbones are currently second-hand unless confirmed from Hindsight’s full paper or repo.

### MemOS

Source:
- https://github.com/MemTensor/MemOS

Disclosed models:
- processing and judging LLM:
  - `gpt-4o-mini`
- embeddings:
  - `bge-m3`

Published score:
- `77.80` on LongMemEval

Useful implication:
- MemOS is worth tracking because it publishes a lower-cost lane built around `gpt-4o-mini` and `bge-m3`, which is relevant to the “capable but affordable” benchmark strategy.

## Systems With Public Memory Claims But No Clear LongMemEval Source In This Pass

### Mem0

Sources checked:
- https://mem0.ai/
- https://mem0.ai/research

What I found:
- strong public results on LoCoMo / related memory evaluation
- no clear primary-source LongMemEval score in this pass

Note:
- OMEGA’s comparison pages explicitly say Mem0 has not published a LongMemEval score.
- that is still a second-hand statement, but it matches what I found in Mem0’s own public materials.

## Practical Reading Of The Landscape

### Stable judge lane

Systems that explicitly use `gpt-4o` as judge or benchmark comparator:
- Supermemory
- Zep
- EmergenceMem
- Mastra uses `gpt-4o` as the benchmark-comparison lane

This is the strongest reason to keep a stable `gpt-4o`-like judging lane in RecallBench when comparing systems.

### Answer model trends

The public leaderboard has moved in three directions:

- classic official lane:
  - `gpt-4o`
- stronger closed or frontier lane:
  - `gpt-5-mini`
  - `gemini-3-pro-preview`
  - `GPT-4.1`
- lower-cost or open-weight lane:
  - `gpt-4o-mini`
  - `GPT-OSS 20B`
  - `GPT-OSS 120B`

### What matters for RecallBench tuning

For competitive LongMemEval_S tuning, the strongest published examples suggest:
- keep the judge stable
- compare answer models separately
- do not treat every vendor-reported top score as equally trustworthy
- record whether a score is:
  - directly reproducible and fully sourced
  - partially sourced
  - or vendor-reported without a matching public run recipe

## Recommended RecallBench Baselines

Based on the current public landscape, the most defensible comparison lanes are:

1. Official-style lane
- answer: `gpt-4o`-class model
- judge: `gpt-4o`

2. Strong frontier lane
- answer: strongest available stable model with direct benchmark-friendly output behavior
- judge: `gpt-4o`

3. Lower-cost lane
- answer: `gpt-4o-mini` or equivalent
- judge: `gpt-4o`

4. Open / open-weight lane
- answer: `GPT-OSS 20B`, `GPT-OSS 120B`, or other open-weight equivalent
- judge: `gpt-4o`

## Open Questions

- What exact larger backbone did Hindsight use for `91.4%`?
- Which OMEGA score is the current canonical one: `76.8%` or `95.4%`?
- Are Mastra’s `gpt-5-mini` runs fully reproducible from open code and prompts, or only partly reproducible?
- Which public systems publish both retrieval metrics and judged LongMemEval outputs in a way that RecallBench can ingest directly?
