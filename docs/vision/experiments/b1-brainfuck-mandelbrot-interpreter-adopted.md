# Experiment Registry: B1 Brainfuck Mandelbrot Interpreter: Adopted

[Authority](../experiments.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

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
  [`../../meta/benchmarks/brainfuck/reference.c`](../../../meta/benchmarks/brainfuck/reference.c),
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
- Harness: [`../../meta/benchmarks/brainfuck/benchmark.py`](../../../meta/benchmarks/brainfuck/benchmark.py)
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
