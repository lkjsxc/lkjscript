# Performance Scorecard

## Purpose

Define reproducible performance categories and tiered-runtime evidence without
claiming one universal meaning of “fastest language.”

## Status

**Accepted Target.** Current historical measurements are diagnostic unless
their individual records say otherwise. No world-leading or JIT claim exists
until the applicable category has reproducible correctness and performance
evidence.

## Categories

### Native Compute

Integer and floating loops, SIMD, branches, direct calls, recursion, hashing,
parsing, and algorithm-equivalent kernels.

### Memory

Allocation throughput, bytes allocated/copied, peak RSS, cache misses,
GC/region pause distribution, long-running fragmentation, executable-code and
metadata bytes, and code-cache peak.

### Startup And Tiering

Cold CLI start, source compile plus VM run, VM instantiation, JIT trigger time,
JIT compilation, time to first native execution, end-to-end runtime, warm
restart, steady-state runtime, and time to break even against VM-only execution.
The minimal AOT test emitter may be timed as a backend diagnostic but is not the
adaptive product strategy.

### Server

Throughput, p50/p95/p99/p99.9 latency, concurrent connections, memory per
connection, first-request and post-warmup behavior, tier transition latency,
tail latency under collection/compilation, overload behavior, and bounded-queue
rejection.

### Compiler

Parsing, resolution, HIR and SSA construction, verification, each optimization
pass, incremental rebuild, native code-object generation, minimal file
emission, output/code/metadata size, and peak compiler memory.

### Operational Predictability

Worst observed latency, variance, resource-limit response, cancellation
latency, deterministic-mode overhead, JIT fallback behavior, code-cache limits,
and correctness under long-running load.

## Required Metadata

Every retained result records:

- repository commit and clean/dirty state;
- CPU, RAM, OS/kernel, target CPU/ABI, and tool versions;
- source/interpreter/runtime mode and optimization tier;
- workload identity/hash and correctness oracle;
- cold/warm state, warmups, repetitions, ordering, median, dispersion, tails,
  and unremoved samples;
- peak RSS and artifact cleanup;
- JIT thresholds and time until each trigger;
- compilation time and time to first native execution;
- end-to-end and steady-state time and time to break even against the VM;
- baseline/optimizing compilation counts, OSR transitions, guard failures,
  deoptimizations, VM fallbacks, native code bytes, metadata bytes, and
  code-cache peak where applicable.

A field that does not apply is recorded as not applicable; a metric that was not
collected is recorded as not measured. Offline PGO profile identity is not
required because offline PGO is rejected by
[Runtime JIT Instead of Offline PGO](../decisions/runtime-jit-instead-of-offline-pgo.md).

Comparisons use algorithm-equivalent implementations and also disclose
idiomatic optimized competitors separately. Portable and native-target binaries
are never presented as equivalent configurations. Direct lkjscript Mandelbrot
and Brainfuck Mandelbrot interpreted by lkjscript are not algorithm-equivalent.

## Tiered Runtime Comparisons

Applicable JIT studies retain variants among:

```text
reference VM
baseline JIT including compilation
baseline JIT steady state
optimizing JIT including compilation
optimizing JIT steady state
```

Compilation and warmup costs are part of the result. A steady-state native win
is not reported as an end-to-end win when total execution remains slower. A
forced JIT variant must prove required native code executed; a silent VM
fallback invalidates that forced-mode sample.

## Runtime Observation Policy

Adaptive counters are bounded, saturating, local to the current process, used
only to optimize that process, and discarded on exit. They are never uploaded
and are not telemetry. Persistent profiles and cross-run native-code caches are
not planned. Ordinary bytecode instructions do not each receive a profiling
counter; initial events are function entries and loop backedges.

## Resource Modes

Resource interruption has two separately measured modes:

- normal safety mode checks epochs/deadlines at safepoints such as loop
  backedges, calls, allocations, host calls, yields, and VM/native transitions;
- deterministic metering counts blocks or instructions for reproducible
  tenancy/accounting and reports its overhead separately.

JIT compilation work, executable bytes, metadata, object count, and code-cache
retention have explicit budgets in both modes.

## Required Workload Classes

Retain cold hello, direct lkjscript Mandelbrot, numeric loops, call-heavy
recursion, allocation-heavy and branch-heavy cases, byte/buffer processing, a
long-running OSR loop, one-shot HTTP, repeated post-warmup server requests, and
Brainfuck Mandelbrot interpreted by lkjscript. Reports state when a workload
cannot benefit before loop OSR exists.

## Retained Callable Scalar Baseline Result

Implementation commit `025cbb2feadbb18fbae51e68e38b9c849798d068`
meets the aspirational 5x generated-execution target on the supported
100,000-iteration F64/direct-call workload: forced native execution median was
7.647935 ms versus same-commit VM execution median 352.918413 ms, a 46.146x
speedup excluding compilation. Median native lowering/encoding plus relocation/
W^X installation was 0.076654 ms, so the measured repeated whole-workload
break-even is one invocation. Process medians, which include source compilation
and startup, were 354.533038 ms VM, 9.372036 ms forced baseline, and 214.482019
ms auto at threshold 64: 37.829x forced and 1.653x auto end-to-end speedups.

This is one allocation-free scalar workload, not a general language score.
Forced mode installed 1,926 code bytes and 2,618 metadata bytes in one 4,096-byte
accounted mapping. Auto installed only `scalar-step`: 751 code bytes, 1,074
metadata bytes, and one 4,096-byte mapping. Median polled peak RSS was 2,736 KiB
VM, 2,724 KiB forced, and 2,808 KiB auto. All four warmups and 31 randomized
samples per variant, exact outcome `F64 bits 0x401af3ef5a48f5f0`, compiler/JIT
phase distributions, threshold alternatives, environment, hashes, and every
unremoved sample are retained under
[`../../meta/benchmarks/jit/results/`](../../meta/benchmarks/jit/results/).

The pre-JIT `c4c96094260072323f9399fe7f0f7b4a14d1eef6` diagnostic used a
compatible source with an in-program exact-bit oracle. Current explicit VM
median was 357.510855 ms versus 364.419240 ms pre-JIT (ratio 0.981); the 6.908 ms
difference was less than twice the larger 3.939 ms MAD, so no VM regression or
improvement is claimed. Current binary size was 1,448,584 bytes versus 1,129,440
bytes (1.283x), and median RSS was 2,756 versus 2,272 KiB. The larger binary and
RSS remain visible baseline-JIT costs.

## Retained Forced Optimizing Results

The clean `cc967ff7e6f57a3225ae974d64ced6039ed8e9ae` locked release protocol
retained four warmups and 31 measured samples for same-commit forced baseline
and forced optimizing execution of `src/examples/jit-optimizing`, plus the same
count for the forced-baseline scalar sentinel, in one deterministic randomized
interleaving. A separate reference VM required exact returned I64 `3333`;
normal streams were silent. Every sample and order, phase metric, peak RSS poll,
tier/object/proof/code fact, and median/MAD/p95/min/max distribution is retained
in the adopted
[`../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64.json`](../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64.json).

| Forced optimizing workload metric | Baseline | Optimizing |
| --- | ---: | ---: |
| native execution median / MAD / p95 | 1.999889 / 0.010469 / 2.092424 ms | 0.670029 / 0.002174 / 0.686310 ms |
| process wall median / MAD / p95 | 3.565363 / 0.035817 / 4.145873 ms | 2.411023 / 0.014387 / 2.609285 ms |
| polled peak RSS median / MAD / p95 | 4,148 / 40 / 4,224 KiB | 4,048 / 40 / 4,144 KiB |
| generated code / retained metadata | 13,656 / 15,953 B | 2,424 / 3,281 B |
| native entries | 10,001 baseline | 10,001 optimizing |

Optimizing native execution was 2.984780x faster. Its exact 1,329,860 ns median
improvement exceeded twice the combined native MAD, 25,286 ns. Process wall
improved from 3,565,363 to 2,411,023 ns, a 1.478776x speedup. The optimizing
case retained 72 checked-I64 GVN records, 2,816 estimated certificate bytes, 35
actually executed optimizing passes, zero baseline entries or VM fallback, and
verified W^X. Median optimization, lowering/encoding, and installation were
0.248297, 0.034676, and 0.045806 ms. The corresponding baseline lowering and
installation medians were 0.081343 and 0.057638 ms.

The mechanically complete verdict is **Adopted** for forced first-tier
performance: every exact, stream, tier, proof, W^X, allocation, speed, noise,
and scalar criterion passed. Same-commit scalar native median was 7,982,586 ns
versus retained 7,647,935 ns, ratio 1.043757; process wall was 9,207,038 versus
9,372,036 ns, ratio 0.982395. The recovery followed folding the mandatory entry
poll into ABI-2 frame registration, removing a separate runtime transition
without weakening polling or proof checks.

The earlier `063668e08b92a97a2feae8397ff0d634887bd0b6` result remains visible in
[`../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64-rejected-scalar-regression.json`](../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64-rejected-scalar-regression.json).
Its optimizer-local 2.930761x result passed, but scalar native was 8,182,742
versus 7,647,935 ns, ratio 1.069928, so the complete verdict was **Rejected**;
scalar process wall passed at 9,340,049 versus 9,372,036 ns, ratio 0.996587.
The sentinel includes compiler/runtime evolution and does not attribute either
regression or recovery to optimizing passes. The adopted run preserves rather
than reinterprets this negative evidence.

One untimed allocation-graph metrics execution returned exact I64 `1`, entered
optimizing code three times, allocated seven objects, collected six times,
reported 14 attempted/14 successful heap calls and maximum three roots, and had
zero baseline entry/fallback with verified W^X. This is only a forced first-tier
boundary on one CPU, not a general language score. Automatic optimizing
promotion remains disabled and unmeasured; no OSR, deoptimization, or
speculation capability or measurement is claimed.

## Claim Policy

Claims are category- and tier-specific: for example, “lowest median cold
startup in this published set” or “baseline JIT breaks even after N calls under
this cache budget.” A geometric mean may summarize a declared suite but never
replaces per-workload results. Regressions, failed compilation, VM fallbacks,
rejected candidates, and unfavorable categories remain visible.
