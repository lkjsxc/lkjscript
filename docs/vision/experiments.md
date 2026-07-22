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
| C0 | duplicated untyped AST interpretation | bytecode VM | current baseline |
| C1 | resolved typed HIR | bytecode VM | isolate correctness and compile-time cost |
| C2 | typed HIR + SSA | verifier + reference evaluator/VM | differential SSA semantics |
| C3 | typed SSA | shared x86-64 code-object backend + minimal AOT test emitter | native ABI, relocation, and codegen boundary |
| C4 | typed SSA | synchronous function-triggered baseline JIT | callable native execution and break-even |
| C5 | typed SSA | loop-triggered baseline JIT + OSR | long-running invocation transfer |
| C6 | typed SSA | proof-based optimizing JIT | static optimization and tier promotion |
| C7 | typed SSA + guarded observations | specialization + deoptimization | later measured dynamic assumptions |

C1 is adopted only when accepted/rejected corpus behavior and runtime outputs
remain identical while duplicate resolution/lowering logic is deleted. C2-C7
require differential values, output, traps, outcomes, GC, and resource-limit
tests before performance measurement. C4-C7 forced modes must prove native code
actually executed rather than silently falling back to the VM.

## P0 Offline PGO: Rejected

- Status: **Rejected by Product Decision**, not by measurement.
- Removed mechanism: instrumented training builds, profile generation/merging,
  profile-use rebuilds, persistent profile artifacts, and PGO-specific release
  decisions.
- Replacement: bounded saturating observations used only by the current process
  for its own runtime JIT tiers and discarded at exit.
- Evidence boundary: no offline PGO implementation or benchmark was run, so no
  performance conclusion about PGO is claimed.
- Reconsideration condition: a later explicit product decision must supersede
  [Runtime JIT Instead of Offline PGO](../decisions/runtime-jit-instead-of-offline-pgo.md).

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

## S2 Exact Typed Empty Lists: Adopted

- Baseline: `8d221ac` (dedicated Unit and exact `if`)
- Candidate: `45e2d085e13638b92dd1c08e12b2095781f0d248`
- Environment/build: the S1 Linux x86-64 host and locked release procedure;
  each binary ran algorithm-equivalent source from its own commit
- Correctness: hello, Mandelbrot, and Leibniz output bytes were identical;
  64 tests, exact source closure, strict Clippy, release smokes, typed-list
  positive/negative CLI boundaries, lkjedit, and HTTP passed
- Timing: four warmups and 31 deterministic randomized samples per variant;
  entries are median / median absolute deviation / p95 process milliseconds

| Workload | Baseline | Typed empty list | Candidate / baseline |
| --- | ---: | ---: | ---: |
| hello compile + run | 0.447 / 0.022 / 0.497 | 0.448 / 0.019 / 0.507 | 1.002 |
| Mandelbrot compile + run | 5.171 / 0.118 / 5.990 | 5.061 / 0.206 / 5.790 | 0.979 |
| Leibniz 200,000 compile + run | 74.766 / 1.680 / 78.994 | 70.040 / 0.755 / 74.601 | 0.937 |
| Mandelbrot compile + disassemble | 0.672 / 0.026 / 0.967 | 0.674 / 0.013 / 1.169 | 1.003 |

Release binary size increased from 646,368 to 652,080 bytes (1.009x). No
performance threshold was recorded before this required semantic slice. The
runtime medians are diagnostic—especially the unexplained Leibniz movement,
which is not attributed to typed lists. The slice was adopted for exact type
semantics and removal of nil/list ambiguity. Temporary artifacts were removed
and the candidate release tree was rebuilt.

## S3 Explicit Option And No Nil: Adopted

- Baseline: `45e2d085e13638b92dd1c08e12b2095781f0d248`
- Candidate: `0f17d9749966101473746530b26415127371d5a1`
- Environment/build: the S1 Linux x86-64 host and locked release procedure;
  each binary ran algorithm-equivalent source from its own commit
- Correctness: hello, Mandelbrot, and Leibniz output bytes were identical;
  70 tests, exact source closure, strict Clippy, release smokes, Option/argument
  CLI boundaries, removed Nil diagnostics/opcodes, malformed-VM boundaries,
  lkjedit, HTTP, and Docker verification passed
- Timing: four warmups and 31 deterministic randomized samples per variant;
  entries are median / median absolute deviation / p95 process milliseconds

| Workload | Baseline | Option/no-nil | Candidate / baseline |
| --- | ---: | ---: | ---: |
| hello compile + run | 0.408 / 0.014 / 1.141 | 0.405 / 0.015 / 0.516 | 0.994 |
| Mandelbrot compile + run | 5.052 / 0.224 / 5.905 | 5.188 / 0.258 / 5.948 | 1.027 |
| Leibniz 200,000 compile + run | 70.093 / 1.057 / 80.102 | 71.174 / 0.824 / 77.158 | 1.015 |
| Mandelbrot compile + disassemble | 0.668 / 0.021 / 0.996 | 0.678 / 0.015 / 0.803 | 1.014 |

Release binary size increased from 652,080 to 660,112 bytes (1.012x). No
performance threshold was recorded before this required safety/semantic slice;
the figures are diagnostic and show no median regression above 3% in this
sample. The slice was adopted to eliminate type-confused absence and semantic
fallback values. Temporary artifacts were removed and the candidate release
tree was rebuilt.

## S4 Explicit Equality Families: Adopted

- Baseline: `13fbd1bd9a44d4d65864fad3f6a571df9901de9d`
- Candidate: `ba1d2219fcff34f53f8a7f316c2ece39ba6357bd`
- Semantic result: universal `eq`/`ne` and opcode 21 were removed; exact
  `equal-value`, `same-object`, bounded `list-equal`, and `f64-bits-equal`
  operations now resolve through HIR and execute in the VM. Str and Symbol use
  distinct constant/runtime categories.
- Correctness: 80 tests, exact 127-source/11-root closure, strict Clippy, hello,
  direct Mandelbrot, Brainfuck smoke, lkjedit, and HTTP passed. Runtime outputs
  were byte-identical across variants for hello, direct Mandelbrot,
  Leibniz-200000, and Brainfuck hello. Disassembly intentionally changed to the
  canonical equality vocabulary. A separate clean candidate full Brainfuck
  Mandelbrot correctness run completed in 1,265.803147 seconds and matched the
  independent 6,240-byte oracle with SHA-256
  `83a0aac65090b3b5e85c22337afac39d8ac17bfd88675f044b33bd55ca0c351b`.
- Docker: the first verification attempt exposed pre-existing broken links
  because the image omitted `meta/benchmarks`; commit
  `41d3d31346dc498cf441cdaa736187ff5c001c81` copied those committed link targets,
  and the exact Docker verification command then passed with `result=ok`.
- Environment: Linux 7.0.0-27-generic x86-64, AMD Ryzen 9 9955HX
  16-core/32-thread, 32 GiB RAM, Rust/Cargo 1.96.0, Python 3.12.3.
- Build: locked release workspace in one target tree; baseline and candidate
  binaries and exact source snapshots were retained only under `target/` for
  measurement.
- Timing: four warmups per variant/workload followed by 31 samples per variant
  and workload in deterministic randomized order (`0xE0A117`); monotonic
  end-to-end process wall time with stdout discarded. Entries are median / MAD
  / p95 milliseconds.

| Workload | Universal equality | Explicit equality | Candidate / baseline |
| --- | ---: | ---: | ---: |
| hello compile + run | 0.510 / 0.036 / 1.736 | 0.525 / 0.041 / 1.406 | 1.029 |
| Mandelbrot compile + run | 5.393 / 0.079 / 6.081 | 5.408 / 0.111 / 6.549 | 1.003 |
| Leibniz-200000 compile + run | 74.942 / 1.189 / 78.240 | 73.314 / 1.121 / 78.982 | 0.978 |
| Mandelbrot compile + disassemble | 0.824 / 0.055 / 2.065 | 0.801 / 0.032 / 1.287 | 0.972 |
| Brainfuck hello compile + run | 1.842 / 0.046 / 2.869 | 1.839 / 0.057 / 3.116 | 0.998 |

Release binary size increased from 660,112 to 663,888 bytes (1.006x). No
performance threshold was chosen before this required semantic migration. The
sample is diagnostic and shows no candidate median movement above 3%; it does
not support a general performance claim. The slice was adopted for exact static
semantics, removal of closure identity, bounded structural comparison, and
backend-ready operation identities. No samples were removed. Temporary source
snapshots, copied binaries, and detailed JSON were deleted after this compact
record was committed.

## S5 Immutable Nominal Products: Adopted

- Baseline: `e75dae80410c07df7f3ab42b237101e4eb65876f`
- Candidate: `6452104a098f8aa127a13de44fce3c34615e3f78`
- Semantic result: zero-to-15-field nominal product declarations, exact ordered
  construction, typed named access, immutable field replacement, deterministic
  ProductIds/field descriptors, bytecode/disassembly metadata, precise tracing,
  and malformed-VM checks are Current. Product declarations add no runtime
  global or initializer, and all equality families reject Product values.
- Correctness: 88 tests, exact 127-source/11-root closure, strict Clippy, product
  source-to-VM execution/disassembly, hello, direct Mandelbrot, Brainfuck smoke,
  lkjedit, HTTP, release build, and Docker verification passed. Baseline and
  candidate output bytes were identical for hello, direct Mandelbrot,
  Leibniz-200000, and folded Brainfuck hello. Candidate Mandelbrot remained
  1,176 bytes with SHA-256
  `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907`.
  A separate full folded Brainfuck Mandelbrot correctness run completed in
  1,381.302003 seconds and was byte-equal to the independent 6,240-byte oracle
  with SHA-256
  `83a0aac65090b3b5e85c22337afac39d8ac17bfd88675f044b33bd55ca0c351b`;
  its bounded 10-second diagnostic timed out as expected. This one correctness
  run is not a replacement performance sample. Mandelbrot disassembly
  intentionally gained empty product metadata tables.
- Environment: Linux 7.0.0-27-generic x86-64, AMD Ryzen 9 9955HX
  16-core/32-thread CPU with 20 logical CPUs available to the Python process,
  32 GiB RAM, Rust/Cargo 1.96.0, Python 3.12.3.
- Build: locked release builds from one Cargo target tree, run sequentially.
  Baseline binary SHA-256 was
  `2e34fa70e7ff579b4f4750762f24b457e2a646c08ce0eaa8165e97d9afc66987`;
  candidate SHA-256 was
  `d2c0a10b3d9febe276efcf5643f366b4d45761857db21df4d74ad957ac33c043`.
- Timing: four warmups per variant/workload followed by 31 samples per variant
  and workload in deterministic randomized order (seed
  `0x50524F44554354`, decimal `22608498539184980`); monotonic end-to-end process
  wall time with stdout
  discarded. Entries are median / MAD / p95 milliseconds.

| Workload | Pre-product baseline | Nominal products | Candidate / baseline |
| --- | ---: | ---: | ---: |
| hello compile + run | 0.496 / 0.064 / 1.344 | 0.501 / 0.050 / 1.386 | 1.011 |
| Mandelbrot compile + run | 5.547 / 0.162 / 6.932 | 5.252 / 0.086 / 6.460 | 0.947 |
| Leibniz-200000 compile + run | 72.659 / 1.424 / 86.283 | 72.201 / 1.405 / 75.228 | 0.994 |
| Mandelbrot compile + disassemble | 0.818 / 0.093 / 2.272 | 0.840 / 0.102 / 1.801 | 1.028 |
| Brainfuck hello compile + run | 1.799 / 0.160 / 3.056 | 1.763 / 0.079 / 2.705 | 0.980 |

Release binary size increased from 663,888 to 702,160 bytes (1.058x). No
performance threshold was selected before this semantic prerequisite. The
runtime sample is diagnostic: sub-millisecond process-startup noise is large,
Mandelbrot moved faster rather than slower, and no runtime median regression
exceeded 3%. The 5.8% binary-size increase is retained visibly for later native
backend/resource scorecards; it does not override the required product-state
semantics. No samples were discarded. Temporary worktree, copied binaries,
harness, and detailed JSON are deleted after this compact record is committed.

## B1 Brainfuck Mandelbrot Interpreter: Adopted

- Question: can a straightforward Brainfuck interpreter authored in canonical
  lkjscript execute and reproducibly validate Erik Bosman's Mandelbrot program?
- Workload identity: **Brainfuck Mandelbrot interpreted by lkjscript**. It is
  not algorithm-equivalent to the native `src/examples/mandel/` workload, and
  their timings must not be presented as a language comparison.
- Input: `pablojorge/brainfuck` commit
  `153924714ae5e569ec39dcf0c0a5b5ae33600cc6`, path
  `programs/mandelbrot.bf`, SHA-256
  `f0f048e90855450fb06f2bea21f914f0d24e6b6c15fd050c68176ff794c6229e`;
  downloaded only below `target/` under the upstream MIT license.
- Oracle: the repository-authored
  [`../../meta/benchmarks/brainfuck/reference.c`](../../meta/benchmarks/brainfuck/reference.c),
  SHA-256 `af6250f93ef18b35e35788958e6c1feed1a20155011e7208546940661dbedf1d`,
  compiled locally with strict C11 warnings and `-O3`; its required output is
  6,240 bytes with SHA-256
  `83a0aac65090b3b5e85c22337afac39d8ac17bfd88675f044b33bd55ca0c351b`.
- Acceptance chosen before measurement: every authored smoke boundary passes;
  Mandelbrot output is byte-equal to the independent oracle; a full direct run
  completes within 1,800 seconds or its timeout is retained before enabling
  only consecutive-identical `+`, `-`, `>`, and `<` run folding; a completed
  expensive variant receives one warmup and at least three measured runs.
- Primary metric: end-to-end release-process wall time, including lkjscript
  startup and compilation, VM initialization, Brainfuck load/preparation and
  execution, output writes, and shutdown. This is not interpreter-loop-only
  time. Peak RSS and allocation were not measured in this initial experiment.
- Harness: [`../../meta/benchmarks/brainfuck/benchmark.py`](../../meta/benchmarks/brainfuck/benchmark.py)
  uses Python standard-library facilities, verifies every output, and retains
  compact JSON under `target/brainfuck-bench/results/`.

The first direct implementation was measured from dirty base commit
`4426d0ec319ff3ab7461110375a4118d09cff2b6`, interpreter source SHA-256
`51ce62c2f328186d44810a3257ecabf8522178c1bb3b769726c70d2756b8c98a`.
Its 10-second diagnostic and 1,800-second full run both timed out, so no direct
completion time or stable direct benchmark is claimed. The oracle completed,
but byte equality could not be established for that timed-out run. The exact
harness command was `python3 meta/benchmarks/brainfuck/benchmark.py --mode
correctness --no-build --diagnostic-timeout 10 --timeout 1800`; the executed
release binary command was `target/release/lkjscript run
src/examples/brainfuck/main.lkjscript --
target/brainfuck-bench/inputs/mandelbrot.bf` (absolute checkout paths were
recorded by the harness).

One permitted run-folding implementation was then tried from the same dirty
base, source SHA-256
`ee43bca92a56b7b00d6961106df64ba90990526bffd8005dece1244c2a0d75e4`.
It remained direct dispatch over a prepared instruction buffer and folded only
identical arithmetic/pointer runs. A full output completed in
1,504.994145 seconds and had the expected 6,240-byte output hash; its 10-second
diagnostic timed out. That preliminary run used the pinned upstream C
interpreter as its comparison process. Audit then rejected that program as the
final oracle because its tape is uninitialized and its signed-`char` arithmetic
is not portable, even though it produced the expected bytes on this machine.
The repository-authored exact reference above replaced it. The superseded
folded interpreter was also simplified to remove avoidable lkjscript helper
calls from the hot path. Neither change erases the retained preliminary result.
The exact preliminary folded harness command was `python3
meta/benchmarks/brainfuck/benchmark.py --mode correctness --fold-runs
--no-build --diagnostic-timeout 10 --timeout 1800`; the interpreter command
added `--fold-runs` after the input path.

The final clean-tree run used implementation commit
`4589fee6c8bccbaad541b1b7f1132eb89a11af32`, interpreter source SHA-256
`bda35e1a38b29b40817924c7263df77439d3982f81fa238fa7fc7365c9acc916`,
and release binary SHA-256
`3371bc1a2d50340387bb44f6dfc143db6d6ced736f2151cb1ce9357518beae46`.
The exact command was `python3 meta/benchmarks/brainfuck/benchmark.py
--fold-runs --warmups 1 --runs 3 --diagnostic-timeout 10 --timeout 1800`.
The 10-second diagnostic timed out as bounded; the full correctness run then
completed in 1,303.539639 seconds and was byte-equal to the independent oracle.
One verified warmup took 1,264.299752 seconds. The three measured end-to-end
process wall times were 1,274.412351, 1,281.143690, and 1,294.269931 seconds:
minimum 1,274.412351, median 1,281.143690, maximum 1,294.269931, and median
absolute deviation 6.731340 seconds. No samples were discarded.

All B1 attempts used Linux 7.0.0-27-generic x86-64, an AMD Ryzen 9 9955HX
16-core/32-thread CPU, 32 GiB RAM, Rust/Cargo 1.96.0, and the locked release
workspace. The final reference compiler was `cc` 13.3.0 with `-O3 -std=c11
-Wall -Wextra -Werror`. Peak RSS and allocation were not measured, so this is a
reproducible workload result rather than a complete scorecard result. The
run-folded variant is adopted to make the workload practical; the original
direct timeout remains the baseline result and no direct completion time is
claimed. Full output files were removed after their hashes were recorded;
downloaded inputs, temporary reference binaries, and compact JSON remain only
in the ignored `target/brainfuck-bench/` tree.

## Deferred Matrices

After process-safe VM outcomes exist, scheduler experiments will compare OS
processes, native threads, cooperative instruction quanta, and epoll plus
quanta using identical mixed workloads. Baseline JIT candidates require typed
SSA, callable bounded code objects, exact outcomes, precise native stack maps,
and separate total/steady-state evidence. Loop OSR requires exact VM/SSA/native
state mapping. Proof-based optimizing JIT does not require general
deoptimization; guarded specialization does and remains a later separate gate.

## Disk Policy

Use one Cargo target directory, run variants sequentially, retain compact text
or structured summaries rather than build trees, keep at most two candidate
executables, run Docker only for final acceptance, and recheck free space after
each experiment batch.
