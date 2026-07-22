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

## Claim Policy

Claims are category- and tier-specific: for example, “lowest median cold
startup in this published set” or “baseline JIT breaks even after N calls under
this cache budget.” A geometric mean may summarize a declared suite but never
replaces per-workload results. Regressions, failed compilation, VM fallbacks,
rejected candidates, and unfavorable categories remain visible.
