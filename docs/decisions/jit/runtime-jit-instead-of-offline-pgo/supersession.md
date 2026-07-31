# Runtime JIT Instead of Offline PGO: Supersession

[Authority](../runtime-jit-instead-of-offline-pgo.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Supersession

[Measured Execution Portfolio](../../execution/execution-portfolio.md) supersedes this record's
permanent rejection of production AOT, optional explicit local PGO, and
content-addressed cross-run native caches. Runtime JIT remains the primary
adaptive path, and every Current VM/JIT contract and retained result in this
record remains authoritative evidence. No newly accepted portfolio mode is
Current merely because the long-term policy changed.
## Decision

The long-term execution tiers are:

```text
reference bytecode VM
  -> baseline native JIT
  -> proof-based optimizing JIT
  -> guarded runtime specialization when justified
```

Runtime JIT is the primary path for adaptive performance. This supersedes the
former active ordering `native AOT -> PGO AOT -> baseline JIT -> optimizing
JIT` and its gate requiring optimizing JIT to beat PGO AOT. That ordering governed the Current JIT
implementation. The later execution-
portfolio decision now permits optional explicit local PGO only after common
SSA/AOT/artifact identity and a new measured implementation slice; it does not
change Current behavior.

The compiler architecture is:

```text
canonical source
  -> parsed AST
  -> resolved typed HIR
  -> typed SSA
  -> verified optimization pipeline
      +-> reference bytecode
      +-> native code object backend
              +-> test/AOT emission harness
              +-> baseline JIT code objects
              +-> optimizing JIT code objects
      +-> direct Wasm later
```

Typed SSA is the semantic and optimization authority. The native backend is
shared by callable JIT code objects and the minimum file-emission surface needed
for ABI, relocation, disassembly, and differential tests. AOT and JIT may
differ in placement, relocation, and linking, but not in type semantics, traps,
effects, or optimization ownership.

A separate bytecode-to-machine-code compiler that independently reinterprets
language semantics is rejected.
## Current Callable-Baseline Cycle

The completed first native point is a real callable synchronous baseline JIT
on Linux x86-64. Generated native code must be lowered from verified typed SSA,
installed through bounded W^X memory, entered by the runtime, perform meaningful
language computation, and return the same value or structured outcome as the
VM. Forced mode proves a nonzero native entry count and cannot silently fall
back. Emission, object bytes, assembly text, disassembly, a Rust simulation,
hotness counters, or the observation hook are insufficient.

Only whole-function function-entry tiering is Current. Host-independent native
allocation/collection and proof-certified optimizing execution are Current in
forced mode; ownership/host-capability adapters and broader passes remain later. The exact
synchronous automatic optimizing promotion slice is selected but unimplemented;
loop OSR remains later. Background compilation, guards, deoptimization, persistent
profiles/caches, and non-Linux/non-x86-64 platforms remain later or rejected as
classified below. The detailed prerequisite,
backend-selection, ABI, safety, coverage, and evidence contract is
[Callable Linux x86-64 Baseline JIT Cycle](../callable-baseline-jit.md).
## Current Boundary

The following are **Current**:

- canonical source -> typed HIR -> verified normalized SSA -> bytecode;
- dense synchronous reference-VM execution and precise non-moving VM GC;
- a closed scalar target-lowering plan, x86-64 encoder, opaque image, owned
  bounded W^X system lease, and typed invocation;
- a verified-SSA adapter for Unit/Bool/I64/F64 plus host-independent unique
  bytes/byte-vectors and Str, Product, List, Option, Result operations, direct recursive
  SCCs, checked numerics, branches/loops, and outcomes;
- bounded baseline/optimizing code objects and explicit per-function tier states;
- bounded complete stable-ID certificates, opaque optimized-program authority,
  private candidate reconstruction, and verified algebraic/GVN cleanup output;
- forced generated baseline or optimizing main execution with no fallback, and automatic synchronous
  function-entry compilation used by later calls;
- enum-identified EnterFunctionV1/PollV1 calls with native entry, fuel,
  deadline, and structured-status accounting.

The old observation hook is removed. The native backend foundation still does
not consume source/HIR/SSA itself; only the narrow adapter consumes
`VerifiedProgram`. Unsupported ownership/Handle/host/indirect code remains an
engine error in forced mode. Auto conservatively retains reference-typed and
unsupported functions in VM.

Broader ownership/traits, host-capability allocation, broader Tier 2A passes,
and later OSR remain **Accepted Targets** unless explicitly labeled Deferred or
Rejected. Automatic optimizing promotion has the narrower **Accepted
Implementation Selection** in the proof-JIT decision and is not Current. Their
exact contracts are [Ownership And
Borrowing](../../semantics/ownership-and-borrowing.md), [Coherent Traits And Static
Dispatch](../../semantics/traits-and-static-dispatch.md), [Native References, Frames, And Exact
GC Stack Maps](../native-references-and-gc-stack-maps.md), [Allocation-Capable
Baseline JIT](../allocation-capable-baseline-jit.md), and [Proof-Based Optimizing
JIT](../proof-based-optimizing-jit.md).
## Historical Rejection And Current Absence Of Offline PGO

The Current implementation does not include:

- instrumented training builds followed by release recompilation;
- profile-generation or profile-use compiler modes;
- representative training suites used to produce release binaries;
- profile merging;
- persistent PGO profile artifacts or profile formats;
- PGO-specific inlining or layout decisions;
- user telemetry used to optimize later releases;
- a `PGO AOT` roadmap tier;
- a requirement that optimizing JIT beat a PGO-trained AOT binary.

This removal is a product decision, not evidence that an implemented PGO system
lost a benchmark. Historical measurements remain historical; no PGO
implementation or measurement is claimed.
## Runtime Observation Policy

Current-process hotness data is not offline PGO. JIT observations must be:

- collected only while the current process runs;
- consumed only to optimize that same process;
- process-local by default and discarded on exit;
- bounded by runtime resource limits;
- represented by bounded saturating counters whose overflow cannot change
  program semantics;
- absent from ordinary program stdout;
- never uploaded and never described as telemetry.

Initial observations are function-entry and loop-backedge counts. Optional
basic-block or branch counts follow only after baseline native execution works.
The VM does not increment a profiling counter for every ordinary bytecode
instruction.

Persistent cross-run JIT profiles and persistent native-code caches are not
Current. The superseding portfolio accepts a content-addressed native cache only
after complete identity, invalidation, privacy, portability, artifact-limit,
and reproducibility foundations; persistent profiles remain absent unless an
explicit optional local-PGO slice justifies them.
## Tier 0: Reference VM

All normal code begins in the reference VM. Tiny and cold commands may finish
without paying JIT compilation cost. The VM remains:

- the correctness oracle;
- the cold execution tier;
- the fallback for unsupported functions and exhausted JIT resources;
- the deterministic metering and debugging path;
- the execution path on unsupported platforms.

Tier 0 eventually collects only selected low-overhead hotness events at
function entries and loop backedges. Deterministic metering remains distinct
from adaptive hotness counters.
