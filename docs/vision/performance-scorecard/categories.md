# Performance Scorecard: Categories

[Authority](../performance-scorecard.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

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
- baseline/optimizing enables and thresholds, exact epoch, and time until each
  trigger;
- attempts, structured failures, same-epoch suppressions, and state transitions;
- compilation time, exact baseline entries before first optimized entry, and
  time to first native and first optimized execution;
- end-to-end and steady-state time and time to break even against the VM or
  baseline-only auto control;
- exact tier entries, opaque object IDs/entry tokens, proof/W^X facts, stale
  invalidations, baseline/optimizing compilation counts, OSR transitions, guard
  failures, deoptimizations, VM fallbacks, native code bytes, metadata bytes,
  retained mapping/object/attempt/work/certificate limits, and code-cache peak
  where applicable.

A field that does not apply is recorded as not applicable; a metric that was not
collected is recorded as not measured. No Current mode has an offline profile.
If optional explicit local PGO is later implemented, its workload/profile
identity, training/build cost, privacy boundary, and no-PGO comparison become
mandatory under [Measured Execution
Portfolio](../../decisions/execution/execution-portfolio.md).

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
[`../../meta/benchmarks/jit/results/`](../../../meta/benchmarks/jit/results).

The pre-JIT `c4c96094260072323f9399fe7f0f7b4a14d1eef6` diagnostic used a
compatible source with an in-program exact-bit oracle. Current explicit VM
median was 357.510855 ms versus 364.419240 ms pre-JIT (ratio 0.981); the 6.908 ms
difference was less than twice the larger 3.939 ms MAD, so no VM regression or
improvement is claimed. Current binary size was 1,448,584 bytes versus 1,129,440
bytes (1.283x), and median RSS was 2,756 versus 2,272 KiB. The larger binary and
RSS remain visible baseline-JIT costs.
