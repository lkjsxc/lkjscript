# Experiment Registry: Required Record

[Authority](../experiments.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

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
[../current-state.md](../../current-state.md). Its functional gates are usable;
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
loads, tail recursion at multiple arities, branch-heavy work, list operations,
bulk byte/buffer IO, compiler startup, lkjedit, one-shot and repeated-warm HTTP,
direct lkjscript Mandelbrot, and Brainfuck Mandelbrot interpreted by lkjscript.

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
| C0 | duplicated untyped AST interpretation | bytecode VM | superseded historical baseline |
| C1 | resolved typed HIR | bytecode VM | adopted current baseline; isolate correctness and compile-time cost |
| C2 | typed HIR + SSA | verifier + reference evaluator/VM | differential SSA semantics |
| C3 | typed SSA | shared x86-64 code-object backend + minimal AOT test emitter | native ABI, relocation, and codegen boundary | <!-- LKJ-EXACT-DATA -->
| C4 | typed SSA | synchronous function-triggered baseline JIT | callable native execution and break-even |
| C5 | typed SSA | loop-triggered baseline JIT + OSR | long-running invocation transfer |
| C6 | typed SSA | proof-based optimizing JIT | static optimization and tier promotion |
| C7 | typed SSA + guarded observations | specialization + deoptimization | later measured dynamic assumptions |

C1 is adopted only when accepted/rejected corpus behavior and runtime outputs
remain identical while duplicate resolution/lowering logic is deleted. C2-C7
require differential values, output, traps, outcomes, GC, and resource-limit
tests before performance measurement. C4-C7 forced modes must prove native code
actually executed rather than silently falling back to the VM.
## P0 Offline PGO: Historical Rejection, Reclassified Optional Target

- Status: **Historical Rejection by Product Decision**, not by measurement.
  [Measured Execution Portfolio](../../decisions/execution/execution-portfolio.md) now permits
  a later optional explicit local PGO experiment after common SSA/AOT/artifact
  identity; no implementation or measurement is Current.
- Removed mechanism: instrumented training builds, profile generation/merging,
  profile-use rebuilds, persistent profile artifacts, and PGO-specific release
  decisions.
- Replacement: bounded saturating observations used only by the current process
  for its own runtime JIT tiers and discarded at exit.
- Evidence boundary: no offline PGO implementation or benchmark was run, so no
  performance conclusion about PGO is claimed.
- Reconsideration condition: the superseding execution-portfolio decision is
  satisfied only by a new predeclared local-PGO slice with exact profile
  identity, privacy, build-cost, no-PGO/JIT comparison, and semantic validation.
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
## S1 Dedicated Unit And Exact If: Adopted

- Baseline: `9c985e6ae4618bb7bd1f9ae5364719b43da49c82`
- Candidate: `ec4c55513a4d21f509e282f699271cb8feb6723d`
- Environment: Linux 7.0.0-27-generic x86-64, AMD Ryzen 9 9955HX
  16-core/32-thread, Rust/Cargo 1.96.0
- Build: locked release workspace in one shared target tree; each binary ran the
  algorithm-equivalent source checked out at its own commit because Unit is a
  breaking source change
- Correctness: hello, Mandelbrot, and Leibniz output bytes were identical;
  candidate disassembly used Unit and no Nil; 62 tests, exact source closure,
  strict Clippy, release smokes, and Docker verification passed
- Timing: four warmups per binary/workload, 31 samples per variant in
  deterministic randomized order, monotonic process wall time, stdout discarded
  during samples; medians, median absolute deviation, and p95 recorded

| Workload | Baseline median / MAD / p95 | Unit median / MAD / p95 | Candidate / baseline |
| --- | ---: | ---: | ---: |
| hello compile + run | 0.413 / 0.010 / 0.510 ms | 0.410 / 0.011 / 0.701 ms | 0.993 |
| Mandelbrot compile + run | 4.929 / 0.161 / 6.074 ms | 4.855 / 0.117 / 5.729 ms | 0.985 |
| Leibniz 200,000 compile + run | 70.385 / 0.882 / 75.330 ms | 70.651 / 1.093 / 74.717 ms | 1.004 |
| Mandelbrot compile + disassemble | 0.686 / 0.032 / 1.327 ms | 0.689 / 0.026 / 0.851 ms | 1.005 |

Release binary size decreased from 658,288 to 646,368 bytes (0.982x), recovering
part of the C1 size increase. No performance threshold was recorded before this
required semantic migration, so these figures are diagnostic rather than a
performance adoption claim. They show no measured median regression above 1%
in this sample. The migration was adopted for exact semantics and simpler HIR.
Temporary worktree and binary copies were removed; the candidate release tree
was rebuilt after measurement.
