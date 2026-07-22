# Performance Roadmap

## Purpose

Define a measured path toward exceptional runtime performance without turning
aspiration into a release claim.

## Status

Interpreter, precise mark-sweep, and tail-frame reuse are **Current**. Typed IR,
exact numeric execution, adaptive GC, native JIT, and profile-guided
specialization are **Deferred** or **Experimental** until their recorded gates
pass.

## Sequence

```text
truthful semantics and safety
  -> reproducible measurement
  -> normalized typed IR
  -> exact specialized bytecode
  -> allocation and call-path optimization
  -> adaptive memory management
  -> baseline native code
  -> profile-guided specialization
```

Safety and type/runtime agreement are performance prerequisites: an optimizer
cannot preserve semantics that are contradictory or undefined.

## Current Interpreter

The VM uses dense bytecode, contiguous stacks, tagged values, precise
non-moving mark-sweep collection, and return-adjacent frame reuse. Source is
compiled on every CLI invocation. Host effects block synchronously.

A historical GC condition caused a full heap trace before nearly every
instruction after the arena exceeded 4,096 slots. Removing that condition and
reusing tail frames eliminated the observed cliff. Historical debug figures
were 0.091 seconds for 20,000 and 0.877 seconds for 200,000 Leibniz iterations;
the C comparison at 20,000 was 0.001 seconds. The machine, variance, and raw
artifacts were not preserved, so these are diagnostic history rather than a
current baseline.

## Immediate Measurement Work

Create separate measurements for:

- source loading and compilation;
- VM-only execution of an already compiled chunk;
- CLI startup plus compile plus run;
- allocations, collections, live/high-water heap, and pause latency;
- resident memory and output correctness;
- lkjedit and HTTP end-to-end regressions.

Use randomized repeated trials and report median plus dispersion. Compare an
algorithm-equivalent C implementation as well as the optimized iterative C
version.

## Candidate Layers

### Typed IR

Resolve symbols, declarations, imports, and types into one normalized form
consumed by both bytecode and future native lowering. This removes duplicated
operator/type tables and prevents typechecker/code-generator disagreement.

### Exact Bytecode

Use operations whose names encode actual semantics, such as integer and float
arithmetic, checked conversion, and typed resource access. Validate chunks at
public boundaries.

### Allocation And Calls

Measure rooted immutable constants, closure-prototype access without cloning,
tail-call argument movement without temporary allocation, bulk byte IO, and
content-keyed compiler caches independently and in combinations.

### Adaptive Memory

Compare multiple fixed thresholds, live-heap adaptive thresholds, compaction,
and nursery/old-generation designs. Retain rejected policies with workload and
heap-shape conditions in the experiment registry.

### Native Execution

The current JIT seam is an explicit placeholder. Native work starts only after
there is a typed IR and an execution handoff with defined failure and
fallback/deoptimization behavior. Measure warmup and steady state separately.

### Profile-Guided Specialization

Long-running applications may specialize hot calls, representations, and host
paths. Specialization must remain observable, bounded, invalidatable, and
correct under the same conformance suite as the interpreter.

## Adoption Rules

See [experiments.md](experiments.md). No isolated win is adopted solely from a
single microbenchmark. Correctness, whole-suite geometric mean, worst
regression, memory, and operational complexity are considered together.

## Deferred Product Work

Browser, GUI, package, server, supervisor, and framework milestones receive
their own workload matrices. They do not serve as justification for prematurely
shipping an unsafe or semantically inconsistent optimization layer.
