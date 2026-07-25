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
| C0 | duplicated untyped AST interpretation | bytecode VM | superseded historical baseline |
| C1 | resolved typed HIR | bytecode VM | adopted current baseline; isolate correctness and compile-time cost |
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

## N1 Linux x86-64 Native Backend: Adopted

- Status: **Adopted** for a repository-owned emitter; the Cranelift 0.134.2
  production candidate is **Rejected** under this experiment's rule.
- Question: which one future production Linux x86-64 baseline-JIT backend
  should lower the repository-owned typed SSA: an owned byte encoder or a
  mature Rust-native Cranelift-class backend?
- Baseline/candidate tree: clean tracked tree
  `e4c1d0e9f3be2d81df92b629517448859ebb6cd2` from `main` for both. The
  candidate spikes were ignored, uncommitted standalone crates, so there is no
  candidate product commit. Owned source SHA-256 was
  `9d89104d5af6c950bc0b563daf53b74809bc67d1e9427dc47b4e1817030aa058`;
  Cranelift source was
  `ec0ae7692b23bf31c743364b033dcb4b678fad3f9342fc64001d25157e60b7a1`;
  the randomized harness was
  `2f2b2134c9aea9b14ca671695651892a68dcaa17c80139ac680f78afc03c5c28`.
  These hashes identify deleted evidence generators, not production source.
- Worktree/environment: `/tmp/pi-agent-f78a7774-22ba-404-fb7ceb62`, Linux
  `7.0.0-27-generic` x86-64, System V AMD64, AMD Ryzen 9 9955HX, 20 logical
  CPUs available, 32 GiB RAM, Rust/Cargo 1.96.0
  (`ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96` /
  `30a34c6821b57de0aaec83a901aca39f88f6778c`), LLVM 22.1.2, Python 3.12.3,
  Ubuntu 24.04. The owned code used baseline scalar x86-64/SSE2 encodings; the
  Cranelift module used native host detection, `opt_level=speed`, verifier on,
  `is_pic=false`, and non-colocated libcalls.
- Scope: two real candidate JIT images with the same SysV ABI
  `fn(*mut Context, I64, U64, F64) -> I64`. Both generated and called a direct
  helper and an external `runtime_adjust` call. The loop exercised checked
  add/subtract/multiply paths, integer comparisons and conditional branches,
  F64 multiply/add/subtract/comparison, and a loop backedge. The owned spike
  encoded 394 machine-code bytes directly. Cranelift used `JITBuilder`,
  `JITModule`, CLIF construction, `define_function`, relocations,
  `finalize_definitions`, and `get_finalized_function`; it did not use assembly
  text or copied fixture bytes.
- Safety boundary: both spikes intentionally used temporary raw pointers, FFI,
  executable mappings, and native calls. They are not product-safe APIs. No
  product dependency or backend was added.

### Predeclared Gates And Method

The protocol SHA-256 was
`279a152534a6599bdb59650f83359cb59325f84f95cdbcf6f69d7e336c36a92a`.
Before timing, correctness/call/W^X failures, unavailable locking, unacceptable
licenses, or a known unmitigated advisory were disqualifying. A generated-
execution median difference of at least 10% and more than twice the larger MAD
selected the faster candidate if secondary costs were acceptable. Otherwise
Cranelift would be selected if its median two-function compile/finalize was at
most 5 ms, stripped binary at most 20 MiB, clean locked build at most 180
seconds and 2 GiB peak RSS, Linux normal dependency closure at most 50, and its
stack-map/replacement fit credible.

Each process compiled and finalized fresh helper and kernel code. The internal
compile timer included declarations, IR/encoding, relocation, allocation, and
RX finalization; it excluded process startup and, for Cranelift, separately
reported ISA/JIT-module setup. Five oracle cases, including one
20,000-iteration case, called generated code before timing in every process. The execution timer then
made 2,000 generated calls of 20,000 iterations each (40,000,000 loop
iterations) with no timer subtraction. Eight randomized warmup process pairs
preceded 31 retained randomized process pairs. Order seed was
`0x4c4b4a5342454e43`; no sample was removed. Fresh processes reduce hidden
cross-candidate state, while the pre-timing oracle calls warm each generated
code object.

Exact oracle tuples are `(return, external_calls, helper_calls,
external_accum, f64_bits)`: seed/iterations/input `0/0/0.0` produced
`(46,1,1,7,0)`; `5/1/1.25` produced
`(136,1,1,25,4608308320729028560)`; `-17/17/-2.5` produced
`(-314,1,1,-65,13836183955113334096)`; `12345/777/3.999` produced
`(179536,1,1,35905,4616188725301163405)`; and `5/20000/1.25` produced
`(-149879,1,1,-29978,4608348810325596881)`. Every timed batch produced
checksum `-299758000`, 2,000 direct helper calls, 2,000 external calls,
external accumulator `-59956000`, and F64 bits `4608348810325596881`.
The independent Rust reference used checked integer operations and ordinary
IEEE F64 operations. Exact generated results and contexts matched it for every
oracle and retained sample.

### Commands

Commands were run from the worktree above. `measure_command.py` used Python
`monotonic_ns` plus Linux `getrusage(RUSAGE_CHILDREN).ru_maxrss` and checked the
child status.

```sh
cargo generate-lockfile --manifest-path target/backend-spike-owned/Cargo.toml
cargo generate-lockfile --manifest-path target/backend-spike-cranelift/Cargo.toml
cargo fetch --locked --manifest-path target/backend-spike-cranelift/Cargo.toml
cargo fmt --manifest-path target/backend-spike-owned/Cargo.toml
cargo fmt --manifest-path target/backend-spike-cranelift/Cargo.toml
cargo clean --manifest-path target/backend-spike-owned/Cargo.toml
cargo clean --manifest-path target/backend-spike-cranelift/Cargo.toml
python3 target/backend-spike-harness/measure_command.py \
  target/backend-spike-harness/build-owned.json \
  target/backend-spike-harness/build-owned.log \
  cargo build --release --locked --manifest-path target/backend-spike-owned/Cargo.toml
python3 target/backend-spike-harness/measure_command.py \
  target/backend-spike-harness/build-cranelift.json \
  target/backend-spike-harness/build-cranelift.log \
  cargo build --release --locked --manifest-path target/backend-spike-cranelift/Cargo.toml
python3 target/backend-spike-harness/run.py
python3 target/backend-spike-harness/measure_command.py \
  target/backend-spike-harness/rss-owned.json target/backend-spike-harness/rss-owned.log \
  target/backend-spike-owned/target/release/backend-spike-owned sample rss
python3 target/backend-spike-harness/measure_command.py \
  target/backend-spike-harness/rss-cranelift.json target/backend-spike-harness/rss-cranelift.log \
  target/backend-spike-cranelift/target/release/backend-spike-cranelift sample rss
target/backend-spike-owned/target/release/backend-spike-owned wx
target/backend-spike-cranelift/target/release/backend-spike-cranelift wx
target/backend-spike-owned/target/release/backend-spike-owned inspect \
  target/backend-spike-harness/owned.bin
target/backend-spike-cranelift/target/release/backend-spike-cranelift inspect \
  target/backend-spike-harness/cranelift
objdump -D -b binary -m i386:x86-64 -M intel \
  target/backend-spike-harness/owned.bin
objdump -D -b binary -m i386:x86-64 -M intel \
  target/backend-spike-harness/cranelift-kernel.bin
python3 target/backend-spike-harness/dependency_report.py
target/backend-spike-tools/bin/cargo-audit audit \
  --file target/backend-spike-cranelift/Cargo.lock --json
```

### Results

Times are milliseconds. Median absolute deviation is MAD; p95 is nearest-rank.
Generated execution is primary.

| Candidate | Metric | Median | MAD | p95 | Min | Max |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| owned | generated execution | 48.406374 | 0.540016 | 58.985271 | 47.657486 | 60.117770 |
| Cranelift 0.134.2 | generated execution | 119.422902 | 0.566505 | 136.792513 | 118.847981 | 142.511744 |
| owned | compile + finalize | 0.007164 | 0.000591 | 0.015850 | 0.005741 | 0.017463 |
| Cranelift 0.134.2 | compile + finalize | 0.625476 | 0.026470 | 0.706368 | 0.549693 | 1.029094 |
| Cranelift 0.134.2 | excluded ISA/module setup | 0.021520 | 0.002925 | 0.026299 | 0.012473 | 0.034845 |

Cranelift execution took 2.467 times the owned median; equivalently, the owned
candidate reduced elapsed generated-execution time 59.47%. That materially
exceeded the predeclared threshold and dispersion. Inspection showed Cranelift
spilled and reloaded the loop-carried F64 value on each iteration, while the
owned encoder kept it in a register. This explains this generated code, not all
Cranelift workloads. Owned compilation was 87.31 times faster, but execution,
not compilation, decided the selection.

| Cost | Owned | Cranelift 0.134.2 |
| --- | ---: | ---: |
| generated code | 394 B total | 472 B total (72 B helper, 400 B kernel) |
| retained metadata visible to spike | 0 B | at least 80 B; 2 relocations, 0 traps, 0 stack maps |
| stripped release binary | 347,768 B | 4,516,424 B |
| clean locked release build wall | 1.935595 s | 65.213507 s |
| build child peak RSS | 160,792 KiB | 1,241,904 KiB |
| one extra sample child peak RSS | 9,844 KiB | 9,836 KiB |
| Linux runtime-normal dependencies | 0 | 38 |
| Linux build-only dependencies | 0 | 5 |
| all-target locked third-party packages | 0 | 61 |

The metadata figure is a lower bound from public relocation, trap, and outer
stack-map records; it excludes module allocator/table overhead. Owned labels
were fixed before installation and retained no metadata in this scalar spike,
but the production code-object contract still requires relocations,
safepoints, stack maps, and source/outcome maps. Peak build RSS is the maximum
waited child reported by Linux `ru_maxrss`, not aggregate process-tree RSS. The
one runtime RSS observation is secondary and too coarse to establish equality.

Both W^X probes inspected `/proc/self/maps`. The owned mapping was `rw-p` during
emission and `r-xp` after `mprotect`; no RWX protection was requested. A wrapper
around Cranelift's real `SystemMemoryProvider` observed both executable
allocations as `rw-p` from allocation and `r-xp` after
`finalize_definitions`; no RWX mapping was observed. All 31 ordinary finalized
samples also reported `r-xp`. These probes inspect temporary unsafe spikes and
do not establish the future safe `lkjscript-sys` contract.

### Dependency, License, Security, And Maintenance Review

The owned crate had no third-party dependency and its temporary source had 19
lines containing the token `unsafe`; production pure encoding need not be
unsafe, while mapping/call unsafety must move behind `lkjscript-sys`. The
Cranelift manifest pinned 0.134.2 for `cranelift-codegen`, `frontend`, `jit`,
`module`, and `native`; codegen used only `std`, `unwind`, and `host-arch`.
Its lock SHA-256 was
`243c224bd4e07a49d4a5c8614d6121506a142e23d6b38d90243d739f0900fc04`.
Because code generation would run inside the JIT process, the 38 normal
packages are runtime product dependencies; the five build-only packages do not
ship as linked runtime code.

Current-target license declarations were all permissive: 19
`Apache-2.0 WITH LLVM-exception`, 16 `MIT OR Apache-2.0`, three
`Apache-2.0 OR MIT`, two `MIT`, and one each `Apache-2.0 / MIT`,
`MIT/Apache-2.0`, and `Zlib`. `cargo-audit 0.22.2` reported zero
vulnerabilities and no warnings against RustSec database commit
`1abf7a8c1822223a38e99f652bc232071c44a86d` (1,169 advisories, updated
2026-07-23). The source-token proxy found 2,836 `unsafe` occurrences on 2,820
lines in 204 `src/**/*.rs` files across 27 runtime packages, plus one occurrence
in one build-only package. This deliberately overcounts comments and nested
source tests and is not a semantic unsafe audit; it makes the dependency safety
surface visible rather than claiming audited blocks.

Cranelift is maintained in the Bytecode Alliance Wasmtime repository, has a
published 0.134.2 API and documentation, an Apache-2.0-with-LLVM-exception
license, and requires Rust 1.94.0, within this experiment's Rust 1.96.0. That
maturity reduces instruction-selection/register-allocation work but couples the
runtime to a large, rapidly versioned release train. The owned path avoids that
coupling while accepting all Linux x86-64 encoder, allocator, relocation, ABI,
and metadata maintenance in this repository. The direct product dependency
classification would require a separate upgrade/security process if Cranelift
were reconsidered.

Exact all-target locked third-party versions were:

`allocator-api2 0.2.21; anyhow 1.0.104; arbitrary 1.4.2; bitflags 1.3.2; bumpalo 3.20.3; cfg-if 1.0.4; cranelift-assembler-x64 0.134.2; cranelift-assembler-x64-meta 0.134.2; cranelift-bforest 0.134.2; cranelift-bitset 0.134.2; cranelift-codegen 0.134.2; cranelift-codegen-meta 0.134.2; cranelift-codegen-shared 0.134.2; cranelift-control 0.134.2; cranelift-entity 0.134.2; cranelift-frontend 0.134.2; cranelift-isle 0.134.2; cranelift-jit 0.134.2; cranelift-module 0.134.2; cranelift-native 0.134.2; cranelift-srcgen 0.134.2; equivalent 1.0.2; fnv 1.0.7; foldhash 0.2.0; gimli 0.33.0; hashbrown 0.16.1; hashbrown 0.17.1; heck 0.5.0; indexmap 2.14.0; libc 0.2.189; libm 0.2.16; log 0.4.33; mach2 0.4.3; memmap2 0.9.11; proc-macro2 1.0.107; quote 1.0.47; regalloc2 0.15.2; region 3.0.2; rustc-hash 2.1.3; serde 1.0.229; serde_core 1.0.229; serde_derive 1.0.229; smallvec 1.15.2; stable_deref_trait 1.2.1; syn 3.0.3; target-lexicon 0.13.5; unicode-ident 1.0.24; wasmtime-internal-core 47.0.2; wasmtime-internal-jit-icache-coherence 47.0.2; windows-link 0.2.1; windows-sys 0.52.0; windows-sys 0.61.2; windows-targets 0.52.6; windows_aarch64_gnullvm 0.52.6; windows_aarch64_msvc 0.52.6; windows_i686_gnu 0.52.6; windows_i686_gnullvm 0.52.6; windows_i686_msvc 0.52.6; windows_x86_64_gnu 0.52.6; windows_x86_64_gnullvm 0.52.6; windows_x86_64_msvc 0.52.6`.

### Safepoints, Integration, And Decision

The scalar kernel had no GC references, so zero stack maps were correct.
Cranelift 0.134.2 exposes user stack maps at non-tail calls and frontend marking
that spills producer-identified live values, but its own documentation says the
CLIF producer remains responsible for identifying GC-managed values; a loop
backedge needs an explicit call/poll or additional support. The owned emitter
must expose exact register/spill/frame layout and generate repository-owned maps
itself. Allocation-free code may arrive first, and allocation-capable code
remains rejected until exact-map tests pass. Thus Cranelift has the easier fit,
but neither spike implements production GC integration.

The selected owned backend will consume verified typed SSA and return an
uninstalled byte image plus typed relocations and exact code-object metadata.
Executable memory remains a separate `lkjscript-sys` responsibility. This
boundary lets a later measured replacement remove the encoder without changing
SSA semantics, native/runtime ABI identities, tier state, or code-object
ownership. Cranelift is rejected for this production baseline selection but
conditionally retained if broad completed-SSA evidence reverses the runtime
result or the owned implementation fails correctness/maintenance gates.

The owned emitter is adopted by
[Linux x86-64 Native Backend](../decisions/linux-x86-64-native-backend.md).
No production backend or JIT is implemented by this experiment/decision commit.
A current-VM comparison was not practical: the repository has no typed SSA or
native ABI yet, and a separately authored source program would not isolate this
backend decision.

### Every Retained Sample

Values are exact nanoseconds in ordinal order. O/C means owned then Cranelift;
C/O means the reverse. Warmups are correctness/order records, not retained
timing samples.

- owned compile: `7695, 7855, 17463, 7734, 7494, 15850, 7384, 6221, 6953, 7193, 6382, 6863, 6692, 7735, 7164, 7915, 6161, 6432, 6572, 7474, 6983, 7154, 6573, 6211, 6873, 7454, 5741, 10530, 14948, 6503, 8666`
- owned execution: `48089328, 58985271, 48476085, 48461037, 48154932, 48517934, 51366099, 48434998, 49007835, 47841002, 48406374, 48960426, 48028894, 48946390, 51698964, 60117770, 47684418, 47657486, 47869666, 48190459, 47816817, 48074000, 48121699, 49054714, 47851061, 47785297, 48490032, 48388461, 50507184, 47703113, 48508787`
- Cranelift compile: `685418, 706368, 681331, 629995, 658067, 688314, 615658, 681311, 1029094, 617060, 658958, 568178, 647908, 583978, 569560, 700857, 615708, 618042, 626598, 610888, 602573, 613333, 592884, 585421, 610668, 625476, 634593, 549693, 620647, 626097, 651946`
- Cranelift execution: `119585618, 121801292, 122019793, 119126464, 118847981, 118965883, 120353151, 142511744, 118852309, 118987834, 123994506, 119137916, 122609542, 119076110, 120690314, 127713116, 118856397, 120257812, 119130071, 119002422, 120184784, 119355935, 119079937, 118959922, 118940745, 119422902, 119862789, 136792513, 120465702, 119203208, 120386172`
- Cranelift excluded setup: `22713, 22862, 24136, 12473, 22873, 24126, 15569, 26299, 34845, 18435, 25127, 16361, 20859, 17984, 13204, 25568, 22022, 18595, 21520, 20228, 19417, 23884, 14177, 17964, 17713, 19737, 26089, 12934, 23344, 24015, 22713`
- retained pair order: `O/C, O/C, O/C, C/O, C/O, O/C, C/O, O/C, C/O, C/O, O/C, O/C, O/C, O/C, C/O, C/O, O/C, O/C, O/C, O/C, O/C, O/C, C/O, C/O, C/O, C/O, O/C, C/O, C/O, C/O, C/O`
- warmup pair order: `O/C, C/O, C/O, O/C, O/C, C/O, C/O, O/C`

### Limitations And Cleanup

This was one scalar/check-heavy kernel on one CPU, OS, and Cranelift version. It
did not test allocation, GC, exception/unwind behavior, structured language
outcomes, VM/native transitions, recursion, code-cache concurrency, cold-cache
control, multiple target CPUs, or a full typed SSA/ABI. Cranelift's register
spill may not generalize; the owned code was manually register-assigned and
therefore understates a general allocator's implementation cost. Execution
samples include loop/check/call work but exclude process startup and native
compilation. The build RSS method is not aggregate tree RSS. No benchmark
sample was discarded or tuned after observation.

After the compact record and hashes were prepared, the reproducible
`target/backend-spike-owned`, `target/backend-spike-cranelift`,
`target/backend-spike-harness`, and `target/backend-spike-tools` trees were
deleted. Only this documentation evidence remains. No temporary dependency,
lockfile, generated code, result JSON, or binary is retained.

## C4 Callable Scalar Baseline JIT: Adopted

- Status: **Adopted** for retained instrumentation, the scalar baseline tier,
  ordinary `auto`, and a 64-entry default. This does not adopt OSR, native
  references/allocation, recursion, host IO, or an optimizing tier.
- Question: does source-derived generated execution materially beat the
  same-commit VM after making compile/startup/memory costs visible, and which
  conservative function-entry threshold should ordinary runs use?
- Baseline/candidate: same-commit VM, forced, and auto used clean implementation
  commit `025cbb2feadbb18fbae51e68e38b9c849798d068`; the instrumentation/default
  landed in `56535c589998eeefa045fca622720662a2f78662` and time-to-first-entry in
  `025cbb2`. The diagnostic historical VM baseline was
  `c4c96094260072323f9399fe7f0f7b4a14d1eef6`.
- Predeclared completion target: exact oracle and stream silence, forced nonzero
  native entry with zero fallback/failure, at least four warmups and 31
  randomized samples, every sample retained, and aspirational generated
  execution at least 5x same-commit VM. Compilation, process wall, RSS, code,
  metadata, fallback, and break-even remain visible even if unfavorable.
- Workload: canonical `src/examples/jit-scalar`, 100,000 F64 loop iterations and
  direct calls, main SHA-256
  `aa8acecbad8add81f7a3a79b19a69e8f503d36c8af6e1f503b572bfadd14157e`.
  Independent Python IEEE evaluation required returned F64 bits
  `0x401af3ef5a48f5f0`; runtime output was not hard-coded.
- Environment: isolated `/tmp/pi-agent-278f3a26-771b-4bc-ddc2e7ec`, Linux
  7.0.0-27-generic x86-64/glibc 2.39, AMD Ryzen 9 9955HX, 20 logical CPUs
  available, 32 GiB RAM, Rust/Cargo 1.96.0, Python 3.12.3. Release binary
  SHA-256 was
  `94dec3b623f07333ed57659c67d8461c8ac30e7c13684f147700b72cefd9a638`,
  size 1,448,584 bytes. The harness SHA-256 was
  `c8005324d9389194b441988ad09582f50e54ea6b573c808881ed143d22b99c49`.
- Protocol: fixed seed `0x4c4b4a534d455452` (decimal
  5,497,569,491,774,952,530), one verified pre-timing process per variant, four
  randomized warmups per variant, then 31 randomized measured samples per
  variant. Python `monotonic_ns` surrounded process creation through wait;
  `/proc/<pid>/status` `VmRSS` was polled about every 0.5 ms. P95 is nearest
  rank. No sample was removed.

### Primary Result

Times are milliseconds; each cell is median / MAD / p95 / min / max. Process
wall includes process startup, source compilation, engine work, and shutdown.
Internal timers use Rust monotonic `Instant` directly.

| Variant | Process wall | Engine execution | Peak RSS KiB |
| --- | ---: | ---: | ---: |
| VM | 354.533038 / 4.711766 / 362.572659 / 347.360647 / 369.390164 | 352.918824 / 4.019214 / 361.367864 / 345.805002 / 367.389793 | 2,736 / 48 / 2,880 / 2,664 / 2,900 |
| forced baseline | 9.372036 / 0.467328 / 10.364211 / 8.711153 / 10.472645 | 7.752251 / 0.065453 / 9.158604 / 7.680937 / 9.239046 | 2,724 / 52 / 2,812 / 2,572 / 2,860 |
| auto, threshold 64 | 214.482019 / 3.352331 / 226.691819 / 206.949992 / 228.798658 | 212.908863 / 3.337173 / 225.098072 / 205.642925 / 227.326952 | 2,808 / 48 / 2,900 / 2,720 / 2,916 |

Forced native execution after installation was 7.647935 / 0.068429 / 9.067563 /
7.570590 / 9.137916 ms versus VM execution 352.918413 / 4.019445 /
361.367684 / 345.804802 / 367.389623 ms. The median generated-execution
speedup is **46.1456x**, passing 5x. Forced process wall is 37.8288x faster than
VM; auto process wall is 1.6530x faster, a 39.50% reduction. These claims are
specific to this supported scalar workload.

Median source compiler total was 0.368733 ms VM, 0.370507 ms forced, and
0.371899 ms auto. The VM sample's phase medians were source loading 0.104205,
parse 0.022483, HIR analysis 0.046507, effect analysis 0.003045, SSA construction
0.019016, SSA verification 0.019266, normalization 0.076243, bytecode lowering
0.026790, and bytecode validation 0.035898 ms. Full distributions for every
phase and variant remain in JSON.

Forced native lowering/encoding median was 0.040096 ms and relocation/W^X
installation 0.036558 ms, 0.076654 ms combined. Time from JIT-session creation
to first native entry was 0.080141 ms. Its first call was the complete workload
and took 7.647935 ms. Against the 345.270478 ms median execution saved per
whole-workload invocation, compile/install breaks even in one repeated
invocation. Forced counts were one object, 100,001 native entries, 100,000
unboxed direct calls, 300,002 PollV1 calls, zero VM fallback, and zero compile
failure. The object retained 1,926 code bytes, 2,618 metadata bytes, 4,096
accounted mapping bytes, eight relocations, and eight scalar safepoints.

Auto compiled only `scalar-step`; `main` stayed VM, so this is not OSR. Median
time to first native entry was 0.297720 ms and first scalar native-call duration
0.000691 ms. Exactly 64 initial VM entries were recorded, followed by 99,936
native entries and PollV1 calls. There were zero compilation failures and no
unexpected fallback. Its one object retained 751 code bytes, 1,074 metadata
bytes, and a 4,096-byte accounted mapping.

### Threshold Decision

Threshold alternatives used the same clean commit, oracle, binary hash,
warmup/sample count, random seed, and complete three-variant protocol. Auto
results are:

| Threshold | Process wall median / MAD / p95 ms | Time to native entry median | Initial VM entries | Native entries |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 211.286082 / 1.961277 / 220.026771 | 0.081463 ms | 1 | 99,999 |
| **64** | 214.482019 / 3.352331 / 226.691819 | 0.297720 ms | 64 | 99,936 |
| 1,024 | 211.901028 / 1.875576 / 218.506574 | 3.556024 ms | 1,024 | 98,976 |

The process distributions overlap; no threshold is called fastest. Threshold 1
pays compilation on the first observed scalar call. Threshold 1,024 delays
native entry more than tenfold versus 64 without a retained process benefit.
The existing middle value 64 is therefore adopted conservatively: tiny calls
may finish in VM, while this repeated-call workload tiers promptly. One long
main invocation remains VM until OSR exists.

### Pre-JIT VM Diagnostic

The retained `meta/benchmarks/jit/pre-jit-workload` source is accepted by both
commits and checks the exact F64 bits in-program, exiting 1 on mismatch. This
works around the older CLI's lack of returned-value metrics without adding
benchmark output. Both binaries ran with empty stdout/stderr and status 0. Seed
`0xc4c9609426007232`, four randomized warmups, 31 randomized samples per
variant, and the same process/RSS method were used; no sample was removed.

| VM | Process wall median / MAD / p95 / min / max ms | RSS median / MAD / p95 / min / max KiB |
| --- | ---: | ---: |
| pre-JIT `c4c9609` | 364.419240 / 3.637667 / 370.862130 / 359.489554 / 371.563278 | 2,272 / 8 / 2,336 / 2,192 / 2,344 |
| current `025cbb2` | 357.510855 / 3.938523 / 376.733275 / 349.049752 / 401.131845 | 2,756 / 60 / 2,864 / 2,664 / 2,916 |

The current/pre-JIT median ratio was 0.981. The 6.908 ms difference is below
twice the larger MAD (7.877 ms), so this diagnostic supports neither a VM
regression nor an improvement claim. Current RSS median was 484 KiB higher.
The old binary was 1,129,440 bytes, SHA-256
`b5e8be566cc72456e90534a4483d86bca6a1a3e11b58357cca10e90c4a0dafd5`;
the current binary was 319,144 bytes/28.26% larger. The binary and RSS costs are
retained negative evidence.

### Commands, Samples, Limitations, And Cleanup

Correctness preceded timing:

```sh
cargo run --locked -q -p lkjscript-xtask -- quiet verify
cargo build --workspace --release --locked
python3 meta/benchmarks/jit/benchmark.py
python3 meta/benchmarks/jit/benchmark.py --auto-threshold 1 \
  --output /tmp/jit-threshold-1.json
python3 meta/benchmarks/jit/benchmark.py --auto-threshold 1024 \
  --output /tmp/jit-threshold-1024.json
```

Every primary/threshold/pre-JIT sample is committed respectively in:

- `meta/benchmarks/jit/results/callable-baseline-jit-linux-x86_64.json`, SHA-256
  `ffdc3671857e44dfba2cbd2d1c287514a12c5a4c58a07124bfd99645c3bb86a7`;
- `meta/benchmarks/jit/results/auto-threshold-1.json`, SHA-256
  `51d5eccd1705248e9e3a761f9de7e413ef1853e6f7e9befc8f7a006d2db207b0`;
- `meta/benchmarks/jit/results/auto-threshold-1024.json`, SHA-256
  `f99d70bbf3af16d6fb83c1febd453a98620ae1feaa47078f33d14dbed50cdf25`;
- `meta/benchmarks/jit/results/pre-jit-c4-vm-comparison.json`, SHA-256
  `771d5206229f6452f96f5186d82353b80111b455f9e09c894c7c531206246884`.

For the old comparison, one Cargo artifact tree was used sequentially with a
clean between incompatible same-version worktrees; the old binary was copied
only for measurement. The detached `c4c9609` worktree, copied binary, temporary
source copy, and temporary JSON were removed. The current release artifact tree
was rebuilt afterward. The retained source hashes match the removed copy.

This result does not cover unsupported native Str/Symbol/list/buffer/handle,
product/Option/Result/reference/allocation/host paths, recursion, indirect
calls, OSR, background compilation, optimizing tiers, non-Linux targets, or
server behavior. The 5x target passed, so no post-failure profiling or
code-layout manipulation was performed. Docker and the 20-minute Brainfuck
Mandelbrot were not run. Full Brainfuck Mandelbrot remains a later OSR workload.

## C6 Forced Proof-Optimizing Performance Gate: Adopted After Rejection

- Status: **Adopted** for forced first-tier performance at `cc967ff`, after the
  retained `063668e` run was **Rejected** by the scalar native sentinel. The
  forced proof-optimizing correctness slice remains Current; automatic
  promotion remains disabled and unmeasured.
- Question: does the first proof pipeline beat same-commit forced baseline by
  at least 1.20x beyond noise without hiding a greater-than-5% scalar sentinel
  regression?
- Baseline/candidate: the adopted rerun used implementation commit
  `cc967ff7e6f57a3225ae974d64ced6039ed8e9ae` and one clean locked release
  binary. The prior rejected run used
  `063668e08b92a97a2feae8397ff0d634887bd0b6`. Both compare their same-commit
  forced tiers and scalar sentinel with retained callable baseline commit
  `025cbb2feadbb18fbae51e68e38b9c849798d068`.
- Environment: both runs record Linux 7.0.0-27-generic x86-64/glibc 2.39, AMD
  Ryzen 9 9955HX, 20 logical CPUs available, 32 GiB RAM, Rust/Cargo 1.96.0,
  Git 2.43.0, and Python 3.12.3. The adopted 2,131,792-byte binary SHA-256 was
  `7116365712455ef43c180ed84d69f2e521b0da3783074c7324bf6cd7273955b7`.
- Build and command: the standard-library harness ran `cargo build --locked
  --workspace --release` itself, then `python3
  meta/benchmarks/jit/benchmark.py`. Normal VM/baseline/optimizing streams were
  empty. Timed runs opted into one `LKJSCRIPT_METRICS` JSON line.
- Oracle: a separate reference-VM process returned exact I64 `3333` for
  `src/examples/jit-optimizing`; Python independently computed scalar F64 bits
  `0x401af3ef5a48f5f0`; the allocation graph returned exact I64 `1`.
- Protocol: fixed seed `0x4c4b4a534f505449`, four warmups and 31 measured
  samples for optimizing-workload baseline, optimizing-workload optimizing,
  and scalar baseline in one deterministic randomized interleaving. Python
  monotonic nanoseconds covered process creation through collection; Linux
  `VmRSS` was polled about every 0.5 ms. P95 is nearest-rank, MAD is median
  absolute deviation, and no sample was removed.
- Predeclared adoption: at least 1.20x optimizing native speedup; median
  improvement greater than twice combined MAD, where combined is the sum of
  tier MADs; exact outcomes; nonzero optimizing entries and checked proof;
  zero optimizing-sample baseline entries/fallback; baseline tier integrity;
  W^X; allocation correctness; and both scalar native and process medians no
  more than 5% over the retained callable baseline. Every criterion was
  mandatory.

### Prior Rejected Run At `063668e`

Times below are median / MAD / nearest-rank p95 / minimum / maximum for the
first run.

| Case | Native execution ms | Process wall ms | Peak RSS KiB |
| --- | ---: | ---: | ---: |
| optimizing workload, baseline | 1.997375 / 0.016721 / 2.364345 / 1.972247 / 2.467920 | 3.584969 / 0.049302 / 4.415831 / 3.508204 / 4.879183 | 4,128 / 44 / 4,228 / 3,988 / 4,240 |
| optimizing workload, optimizing | 0.681521 / 0.003567 / 0.843666 / 0.674839 / 0.978520 | 2.457400 / 0.053521 / 3.143670 / 2.391696 / 3.199314 | 4,048 / 28 / 4,136 / 3,908 / 4,148 |
| scalar workload, baseline | 8.182742 / 0.044654 / 8.915169 / 8.073577 / 9.839367 | 9.340049 / 0.168027 / 11.450636 / 9.127458 / 12.328947 | 3,712 / 36 / 3,780 / 3,552 / 3,796 |

The optimizer's native median was 2.930761x faster. Its 1.315854 ms improvement
was greater than twice combined MAD, 0.040576 ms. Optimization,
lowering/encoding, and install medians were 0.254879, 0.037590, and 0.047309
ms; baseline lowering and install medians were 0.083547 and 0.061566 ms. Exact
facts were invariant across every warmup and sample: baseline emitted 13,956
code and 16,489 metadata bytes with 10,001 baseline entries; optimizing emitted
2,724 code and 3,817 metadata bytes with 10,001 optimizing entries, 72
checked-I64 GVN records, 2,816 estimated certificate bytes, 35 executed
optimizing passes, zero baseline entries/fallback, and verified W^X.

The scalar native median was 8.182742 ms versus retained 7.647935 ms, ratio
1.069928: **failed** the 1.05 ceiling. Scalar process wall was 9.340049 versus
9.372036 ms, ratio 0.996587: passed. The source SHA-256 remained
`aa8acecbad8add81f7a3a79b19a69e8f503d36c8af6e1f503b572bfadd14157e`,
but compiler, metrics, native ABI, stack checks, binary, and surrounding
emitted code evolved across commits. This sentinel establishes a visible
regression against the retained measurement; it does not attribute that
regression to optimizing passes or compare algorithm-equivalent candidate
engines.

The one untimed allocation check reported 3 optimizing entries, 7 allocations,
259 estimated allocation bytes, 6 collections, 225 peak estimated live bytes,
maximum 3 roots, 14 attempted/14 successful heap calls, 6 barriers, zero
baseline entries/fallback, and verified W^X. All of its correctness/accounting
gates passed.

The first run's all-gates verdict was therefore **Rejected** despite the strong
local optimizer result. Its repository record also truthfully lists dirty
benchmark README, harness, and Python-cache paths. Every warmup, sample, order,
environment/tool record,
source/binary hash, full metric, and distribution remains retained under schema
`lkjscript.optimizing-jit-benchmark.v1` in
[`../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64-rejected-scalar-regression.json`](../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64-rejected-scalar-regression.json),
SHA-256
`3e4341ffab5c0cbd976b3dc228d24dfdd8ff135247b91caafb74f0a571e71cec`.

### Adopted Recovery Run At `cc967ff`

The identical predeclared protocol then retained a clean run after generated
function entry polling was folded into ABI-2 frame registration. This removed
a separate runtime transition without reducing mandatory polls or changing the
optimizer proof pipeline.

| Case | Native execution median / MAD ns | Process wall median ns |
| --- | ---: | ---: |
| optimizing workload, baseline | 1,999,889 / 10,469 | 3,565,363 |
| optimizing workload, optimizing | 670,029 / 2,174 | 2,411,023 |
| scalar workload, baseline | 7,982,586 / 54,212 | 9,207,038 |

The optimizing native speedup was 2.984780x; the exact 1,329,860 ns improvement
exceeded the 25,286 ns twice-combined-MAD threshold. Process wall improved
1.478776x. All exact outcomes and silent-stream checks, tier entry/fallback,
proof, W^X, allocation, and baseline integrity gates passed. Scalar native was
7,982,586 versus retained 7,647,935 ns, ratio 1.043757; scalar process wall was
9,207,038 versus 9,372,036 ns, ratio 0.982395. Both passed the 1.05 ceiling, so
every exact criterion passed and forced first proof-optimizing performance is
**Adopted**.

The complete adopted record is
[`../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64.json`](../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64.json),
SHA-256
`e71d1caf35b57ea50c094806372f6e8c991bf86418f44885c3d6fd0dcd4b082e`.
The recovery does not erase or reinterpret the rejected result, and the scalar
sentinel does not attribute either change to optimizing passes. No automatic
promotion threshold was selected or measured. Automatic promotion remains
disabled; OSR, deoptimization, and speculation remain absent and unclaimed.
Only compact retained JSON is committed; reproducible build and Python cache
artifacts are not evidence and are removed or remain in the shared ignored
target tree.

## C7 Automatic Baseline-To-Proof Promotion: Predeclared, Not Run

- Status: **Accepted Implementation Selection**, not Current and not measured.
  The selected automatic-optimizing policy is CLI-opt-in and disabled by
  default. This predeclaration does not alter the adopted forced `cc967ff` result or the
  retained rejected `063668e` result.
- Question: can synchronous proof promotion repay its compilation cost and
  improve end-to-end auto process time without tail, scalar, correctness,
  ownership, retry, or fallback regressions?
- Baseline/candidates: one clean locked release binary compares auto baseline-
  only (existing VM-entry threshold 64) against optimizing opt-in at exact
  thresholds 64, 256, 1,024, and 4,096 baseline-native entries of the promotion
  root. Unchanged forced baseline/optimizing cases are tier sentinels; a scalar
  forced-baseline case is the historical performance sentinel; allocation and
  reference-group cases are untimed correctness sentinels.
- Protocol: fixed recorded seed, deterministic randomized interleaving, at
  least four warmups and 31 measured samples for every timed case, monotonic
  process-wall measurement, nearest-rank p95, median absolute deviation, no
  removed samples, exact repository cleanliness/commit/environment/tool/source/
  binary identities, one shared artifact tree, and compact retained output.
- Oracle and streams: a separate reference VM supplies the exact result;
  expected stdout/stderr are checked byte-for-byte before a sample is accepted.
  Metrics remain on their opt-in file/stderr channel and cannot contaminate
  program streams.
- Transition oracle: the Nth exact baseline entry of the root must synchronously
  prove, lower, and W^X-install while invoking the captured baseline object.
  The optimizing object is pending until a later root entry. Exact opaque tokens
  identify function/object/tier; helper/direct-callee/VM/install events do not
  count. Main remains VM and reference-signature helpers may call/allocate only
  inside the native group, never through auto VM/native entry.
- Ownership/failure oracle: one process-local session owns one current and at
  most one pending selection plus bounded stale mappings until drop. Stale code
  is never selectable. Each epoch permits one attempt under a bounded total;
  same-epoch attempts are suppressed. Structured failure leaves baseline
  current. A newer explicit epoch invalidates pending/current optimizing code
  to baseline before permitting one bounded retry.
- Required metrics: exact enables/thresholds, epoch, attempts/failures/
  suppressions, all state transitions, baseline entries before first optimized,
  trigger-to-first-optimized and session-to-first-optimized time, exact tier
  entries/object IDs/tokens/code bytes, proof/checker/certificate and W^X facts,
  stale invalidations, current/pending facts, fallback, and retained mapping/
  attempt/optimizer-work/certificate/metadata bounds.

Predeclared adoption is mechanical. Every candidate must have exact correctness,
streams, state, proof, W^X, allocation/reference, and limit results; no repeated
attempt or fallback; at least 1.10x baseline-only median process speedup; a
median improvement greater than twice the sum of candidate and baseline MAD;
nearest-rank p95 no more than 5% worse than baseline-only; and a recorded
break-even entry plus cumulative saving showing optimization/lowering/install
cost repaid before workload completion. The forced scalar sentinel's native and
process medians must each be no more than 5% above retained 7,647,935 ns and
9,372,036 ns.

If candidates pass, select the largest threshold whose process median is
statistically indistinguishable from the fastest passing candidate, defined as
an absolute median difference no greater than twice the sum of those candidates'
MADs. If no threshold passes, automatic optimizing remains disabled and the
complete clean rejection is retained. No C7 command has run and no outcome is
claimed in this documentation-only selection.

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
