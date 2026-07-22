# Experiment Registry

## Purpose

Preserve hypotheses, combinations, evidence, and adoption decisions so useful
ideas are not lost merely because one isolated trial failed.

## Status

**Accepted Target** for the engineering process. Individual experiments carry
their own status.

## Required Record

Every performance or architectural experiment records:

1. identifier and status;
2. question and mechanism;
3. baseline commit and candidate commit;
4. exact environment and commands;
5. correctness oracle;
6. isolated variants and multiple combinations;
7. warmup, repetition count, ordering, and noise statistics;
8. runtime, memory, allocation, and relevant latency metrics;
9. adoption and rejection thresholds chosen before measurement;
10. result, interpretation, and retained future conditions;
11. artifact cleanup performed after compact results are committed.

Allowed statuses are `Proposed`, `Running`, `Adopted`, `Rejected`, and
`Conditionally Retained`. A rejected experiment stays searchable. It may be
conditionally retained when interactions, different workloads, or a later
runtime layer could change the result.

## Foundation Baseline

The pre-cutover baseline is recorded in
[../current-state.md](../current-state.md). Its functional gates are usable;
its single-shot C comparison is diagnostic only and is not a regression gate.

## Runtime Matrix: Proposed

| ID | GC | Constants | Call path | Purpose |
| --- | --- | --- | --- | --- |
| R0 | fixed 1,024 | allocate on load | current closure clone and tail temporary | baseline |
| R1 | fixed 512 | current | current | collection-frequency boundary |
| R2 | fixed 4,096 | current | current | throughput/RSS boundary |
| R3 | adaptive to live heap | current | current | long-lived heap behavior |
| R4 | fixed 1,024 | rooted immutable objects | current | isolate constant allocation |
| R5 | fixed 1,024 | current | no closure clone or tail temporary | isolate call overhead |
| R6 | best measured GC | rooted constants | optimized calls | interaction candidate |

Each retained candidate is tested on numeric loops, allocation churn, literal
loads, tail recursion at multiple arities, list operations, bulk file IO,
compiler startup, lkjedit, and HTTP acceptance.

Default adoption thresholds are correct output, more than 10% target-workload
improvement and twice observed noise, at least 5% geometric-mean improvement,
no workload regression above 5%, and no memory growth above 10%. A GC candidate
also must avoid superlinear cliffs and keep p99 pause and RSS within their
predeclared budgets.

## Validation Matrix: Proposed

| ID | Strategy | Static parse events |
| --- | --- | ---: |
| V0 | current standalone checks plus independent root loads | about 391 |
| V1 | content-keyed shared parse cache | about 117 |
| V2 | V1 plus a declared executable/source manifest | about 117 |
| V3 | V2 plus semantic validation for otherwise unreachable modules | at least 117 |

Caching is adopted only if diagnostics and accepted/rejected programs remain
identical, semantic coverage is complete, wall time improves at least 20%, and
peak memory stays within 25% of baseline. Lower parse counts alone are not a
reason to retain complexity.

## Deferred Matrices

After process-safe VM outcomes exist, scheduler experiments will compare OS
processes, native threads, cooperative instruction quanta, and epoll plus
quanta using identical mixed workloads. Native JIT candidates require a typed
IR, executable code-object boundary, deoptimization contract, and separate
warmup/steady-state evidence before implementation claims begin.

## Disk Policy

Use one Cargo target directory, run variants sequentially, retain compact text
or structured summaries rather than build trees, keep at most two candidate
executables, run Docker only for final acceptance, and recheck free space after
each experiment batch.
