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

## Compiler Pipeline Matrix: Proposed

| ID | Semantic IR | Backend | Purpose |
| --- | --- | --- | --- |
| C0 | duplicated untyped AST interpretation | bytecode VM | current baseline |
| C1 | resolved typed HIR | bytecode VM | isolate correctness and compile-time cost |
| C2 | typed HIR + SSA | reference evaluator/VM | differential SSA semantics |
| C3 | typed SSA | owned baseline x86-64 AOT | native ceiling and ABI baseline |
| C4 | typed SSA | measured mature build-time backend | optimization reference candidate |
| C5 | typed SSA + profile | best AOT candidates | PGO interaction |

C1 is adopted only when accepted/rejected corpus behavior and runtime outputs
remain identical while duplicate resolution/lowering logic is deleted. C2-C5
require differential trap/outcome tests before performance measurement.

## C1 Resolved Typed HIR: Adopted

- Baseline: `5815cf574600cd0d4f90ff19f0fade011749ee6f`
- Candidate: `b7f77d9` (`refactor: make resolved typed HIR authoritative`)
- Environment: Linux 7.0.0-27-generic x86-64, AMD Ryzen 9 9955HX
  16-core/32-thread, Rust/Cargo 1.96.0
- Build: locked release `lkjscript-app`, same target directory, baseline and
  candidate binaries copied only for the measurement and then deleted
- Correctness: identical hello, Mandelbrot, Leibniz, and disassembly outputs;
  60 tests, exact source closure, strict Clippy, release smokes, and Docker
  verification passed
- Timing method: four warmups per binary/workload, 31 samples per variant in
  randomized order, process wall time from monotonic high-resolution clock;
  medians, median absolute deviation, and p95 recorded

| Workload | Baseline median | HIR median | Candidate / baseline | Baseline MAD | HIR MAD |
| --- | ---: | ---: | ---: | ---: | ---: |
| hello compile + run | 1.555 ms | 1.540 ms | 0.990 | 0.145 ms | 0.108 ms |
| Mandelbrot compile + run | 5.591 ms | 5.389 ms | 0.964 | 0.171 ms | 0.158 ms |
| Leibniz compile + run | 78.401 ms | 77.130 ms | 0.984 | 2.504 ms | 3.724 ms |
| Mandelbrot compile + disassemble | 0.876 ms | 0.787 ms | 0.899 | 0.026 ms | 0.029 ms |

Release binary size increased from 608,368 to 658,288 bytes (1.082x). The HIR
was adopted for semantic authority, deletion of duplicate resolution/lowering,
and absence of a measured median runtime regression in this diagnostic sample.
The size increase is retained as an explicit optimization target. These
process-level figures are not a general performance claim or a substitute for
the full scorecard.

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
