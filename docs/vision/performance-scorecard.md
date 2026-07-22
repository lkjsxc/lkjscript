# Performance Scorecard

## Purpose

Define reproducible performance categories without claiming one universal
meaning of “fastest language.”

## Status

**Accepted Target.** Current historical measurements are diagnostic only. No
world-leading claim exists until a category has reproducible comparative data.

## Categories

### Native Compute

Integer and floating loops, SIMD, branches, direct calls, recursion, hashing,
parsing, and algorithm-equivalent kernels.

### Memory

Allocation throughput, bytes allocated, bytes copied, peak RSS, cache misses,
GC/region pause distribution, long-running fragmentation, and retained native
code.

### Startup

Cold CLI start, source compile plus run, VM instantiation, AOT process start,
first request, and warm restart.

### Server

Throughput, p50/p95/p99/p99.9 latency, concurrent connections, memory per
connection, tail latency under collection, overload behavior, and bounded-queue
rejection.

### Compiler

Parsing, resolution, type/HIR construction, SSA construction, optimization,
incremental rebuild, AOT generation, output size, and peak compiler memory.

### Operational Predictability

Worst observed latency, variance, resource-limit response, cancellation
latency, deterministic mode overhead, and correctness under long-running load.

## Required Metadata

Every retained result records CPU, RAM, OS/kernel, compiler and source commit,
target CPU/ABI, VM/AOT/JIT mode, cold/warm state, optimization level, PGO
profile identity, workload hash, correctness oracle, repetitions, randomized
ordering, median, dispersion, peak RSS, and artifact cleanup.

Comparisons use algorithm-equivalent implementations and also disclose
idiomatic optimized competitors separately. Portable and native-target binaries
are never presented as equivalent configurations.

## Modes

Resource interruption has two measured modes:

- normal safety mode checks epochs/deadlines at safepoints such as loop
  backedges, calls, allocations, host calls, and yields;
- deterministic metering counts basic blocks or instructions for reproducible
  tenancy/accounting and reports its overhead separately.

Local profiling and PGO never imply telemetry. Profiles remain local and bind
to source hash, IR version, compiler version, target CPU, and workload identity.

## Claim Policy

Claims are category-specific: for example, “lowest median cold startup in this
published set” or “highest throughput at this p99 and RSS budget.” A geometric
mean may summarize a declared suite but never replaces per-workload results.
Regressions, rejected candidates, and unfavorable categories remain visible.
