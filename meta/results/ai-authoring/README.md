# AI-Authorability Benchmark Bootstrap

## Status

**Current harness, narrow evidence only.** This directory retains replayable
agent tasks and exact result records. The first task compares weaker and stronger
available model configurations using raw text. Semantic-entity and typed-hole
variants are added only after those interfaces are Current; no missing variant
is represented by a fake score.

A tiny task cannot establish that one source format, interface, or model is
generally superior.

## Result Schema

Every `results/*.json` uses `lkjscript.ai-authorability-result/v1` and records:

- exact repository commit and task/prompt hashes;
- model/provider and agent harness;
- interface under test;
- wall duration and model token accounting available from the harness;
- tool calls, failed mutations, compiler invocations, and repair iterations;
- structural/compiler/functional correctness;
- changed and unrelated paths; and
- explicit unmeasured fields.

`validate.py` rejects unknown schema versions, missing metrics, inconsistent
pass verdicts, and unsorted changed paths. It never rewrites a result.

## Protocol

1. Start from the task's exact clean commit in an isolated worktree.
2. Supply the exact prompt bytes from the task JSON.
3. Do not give one interface hidden context unavailable to another.
4. Retain the complete agent event log outside the repository when it contains
   provider-private reasoning; retain hashes and aggregate accounting here.
5. Run every declared acceptance command.
6. Inspect the complete diff, count unrelated paths, and preserve failures.
7. Write one immutable result JSON. Do not remove outliers or failed runs.
8. Compare only like-for-like task/interface variants.

Provider token accounting is provider-specific. `inputTokens` may include
repeated/cached context and is not equated with unique prompt bytes. A null
metric means the harness did not expose it; zero means measured zero.

## Initial Task

`tasks/rename-function-v1.json` renames one function across a two-file canonical
source closure. It measures a common maintenance edit and exact VM acceptance,
not generated-program quality. The raw-text runs establish the replay baseline.
The future semantic variant must use one declaration-key rename transaction and
the same acceptance commands. The future hole variant is inapplicable to this
rename task and therefore must not receive a fabricated result.
