# Runtime JIT Instead of Offline PGO

## Purpose

Replace offline profile-guided optimization with a runtime-JIT-first strategy
for adaptive native performance while preserving one typed semantic pipeline
and the reference VM as the cold tier and correctness oracle.

## Status

**Current** for the reference VM, synchronous scalar and host-independent
allocation/recursion Linux x86-64 baseline tier, and the forced first
proof-based optimizing pipeline. The synchronous automatic baseline-to-proof-
optimizing slice is an **Accepted Implementation Selection**, not yet Current.
Handle/host-capability allocation, lexical ownership adapters, native/VM
reference transitions, broader proof passes, and later OSR remain **Accepted
Targets**.
Offline PGO was **Rejected by Product Decision**, not by measurement; optional
explicit local PGO is now a **Deferred Optional Target** under the superseding
execution portfolio. Canonical source/verified-SSA linkage, bounded code objects,
VM/native transfer, `vm`/`auto`/`baseline-jit`/`optimizing-jit`, PollV1, and actual generated
calls are implemented. Closed machine plans also have ABI-2 exact typed
reference frames/maps and a collecting Buf-reference slot. Source-level host-independent native references/allocation and recursive SCCs
are implemented. Handle/host calls, lexical ownership adapters, native/VM
reference transitions, automatic optimizing promotion, broader proof passes,
OSR, speculative tiers, background work, and deoptimization are absent.

## Supersession

[Measured Execution Portfolio](execution-portfolio.md) supersedes this record's
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
JIT` and its gate requiring optimizing JIT to beat PGO AOT. That ordering governed the Current JIT implementation. The later execution-
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
[Callable Linux x86-64 Baseline JIT Cycle](callable-baseline-jit.md).

## Current Boundary

The following are **Current**:

- canonical source -> typed HIR -> verified normalized SSA -> bytecode;
- dense synchronous reference-VM execution and precise non-moving VM GC;
- a closed scalar target-lowering plan, x86-64 encoder, opaque image, owned
  bounded W^X system lease, and typed invocation;
- a verified-SSA adapter for Unit/Bool/I64/F64 plus host-independent Str, legacy
  Buf, Product, List, Option, Result allocation/operations, direct recursive
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
Borrowing](ownership-and-borrowing.md), [Coherent Traits And Static
Dispatch](traits-and-static-dispatch.md), [Native References, Frames, And Exact
GC Stack Maps](native-references-and-gc-stack-maps.md), [Allocation-Capable
Baseline JIT](allocation-capable-baseline-jit.md), and [Proof-Based Optimizing
JIT](proof-based-optimizing-jit.md).

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

## Tier 1: Baseline JIT

The baseline JIT synchronously compiles a whole eligible function from typed SSA
at a safepoint. Its priorities are low compilation latency, exact semantics,
predictable typed layouts, direct control flow, unboxed scalar operations,
straightforward register allocation, correct VM/native transitions, and
bounded executable memory.

Its initial non-speculative optimization set is limited to inexpensive proven
transformations:

- constant folding;
- unreachable-block removal;
- trivial copy propagation;
- branch simplification;
- direct-call resolution;
- removal of dead instructions with no effects;
- simple block layout.

Baseline native code makes no speculative runtime assumptions and therefore
does not require deoptimization.

## Tier 2A: Proof-Based Optimizing JIT

The Current forced first Tier 2A slice adds exact scalar algebraic identities,
same-block/dominating exact scalar GVN (including checked-I64 successful-check
reuse), and existing verified cleanup behind an independent bounded certificate
verifier. It is not automatically promoted. Its clean retained forced result is
Adopted at 2.984780x native speedup; that evidence does not measure automatic
promotion. Later candidates include:

- inlining and monomorphization under budgets;
- sparse conditional constant propagation;
- common-subexpression elimination;
- loop-invariant code motion;
- redundant bounds- and tag-check elimination;
- scalar replacement and escape analysis;
- stack or region placement;
- strength reduction;
- measured loop unrolling;
- vectorization where target and alias facts permit it;
- hot/cold layout from current-process counters.

This tier remains non-speculative where possible and initially has no general
deoptimization requirement.

## Tier 2B: Guarded Runtime Specialization

Tier 2B is **Deferred** until Tier 2A is correct and retained workloads show a
need for value- or shape-based specialization. Possible observations include
stable call targets, buffer lengths, Option/Result variants, object or closure
shapes, frequent constant arguments, and loop trip-count ranges.

Every runtime assumption requires:

- an explicit guard;
- a defined guard-failure side exit;
- exact state reconstruction;
- deoptimization or another explicit continuation contract;
- usefulness and guard-failure counters;
- compilation, code-size, and metadata limits.

Undefined behavior, unchecked assumptions, and whole-program restart presented
as deoptimization are rejected. Inline caches are not the default for statically
resolved direct calls.

## JIT State Machine

The implemented baseline-only auto state remains Current. The next automatic
promotion model is an **Accepted Implementation Selection**, not Current:

```text
BaselineCandidate -> BaselineCompiling -> BaselineNative
BaselineNative -> OptimizingCandidate -> OptimizingCompiling
  -> OptimizingPending -> OptimizingNative
BaselineCandidate | BaselineCompiling -> Disabled (VM remains current)
any optimizing state -> Disabled (baseline remains current)
```

The Nth exact baseline entry of the promotion root performs synchronous proof
optimization, lowering, and W^X installation but invokes the captured baseline
object. `OptimizingPending` is non-selectable; only a later entry can publish it.
An opaque token binds function, exact object, and tier. Once baseline is
installed, a session owns one current and at most one pending native selection
while retaining bounded stale objects
until drop; invalidated objects are never selectable. A newer explicit epoch
invalidates pending/current optimizing selection back to baseline.

There is no Current `Invalidated` function-state variant. Installed code objects
retain an implemented invalidated bit. The selected promotion model also does
not add one: invalidation unlinks the object's exact tokens and changes
selection/state directly. The complete ownership and transition contract is
[Proof-Based Optimizing JIT](proof-based-optimizing-jit.md).

Initial compilation is synchronous at a safepoint. Background compiler threads
are **Deferred** until VM outcomes, heap access, code-cache ownership,
cancellation, and synchronization are process-safe.

Transitions are available through test-only structured diagnostics and
benchmark counters, never normal stdout. Each function records a bounded retry
count, last structured failure reason, and the resource/configuration epoch in
which compilation was attempted. Current baseline-only failure transitions
remain exact:

- permanently unsupported function: `BaselineCompiling -> VmOnly(disabled)`;
- resource budget reached: `BaselineCompiling -> Observed`, with compilation
  suppressed until the relevant budget/configuration epoch changes;
- ordinary `auto` backend failure: `BaselineCompiling -> Observed` while a
  bounded retry remains, then `VmOnly(disabled)`; execution stays in the VM;
- forced JIT backend failure: leave installed lower-tier state unchanged and
  return an error so tests cannot silently pass via the VM;
- internal compiler inconsistency: fail verification rather than claim native
  execution.

The selected automatic model adds one optimization attempt per explicit epoch
under a bounded total. Optimization failure keeps baseline current, records a
structured reason, and suppresses the same epoch. A newer epoch unlinks and
invalidates pending/current optimized objects back to baseline before one
bounded retry; stale objects remain owned but cannot be selected.

Hotness counters saturate rather than reset on compilation failure. A changed
resource/configuration epoch may permit a bounded retry; an unchanged trigger
cannot cause an unbounded compile/fail loop.

## Hotness, Long Loops, And OSR

Function-call, loop-backedge, and optimization-promotion thresholds are runtime
configuration, not language semantics. Defaults are conservative and derived
from retained benchmarks.

Function-entry triggers alone cannot accelerate one invocation containing a
large loop. Loop-backedge observation and on-stack replacement are required,
with this sequence:

1. function-entry-triggered baseline compilation;
2. loop-backedge-triggered compilation;
3. baseline loop-header OSR entry;
4. optimizing-tier OSR;
5. deoptimization only if Tier 2B later lands.

A loop-header OSR entry has a verified mapping among function identity,
bytecode position, initialized VM locals, operand-stack values, typed SSA loop
parameters, and native ABI frame locations. The initial implementation may
support only exactly representable loop headers. Unsupported loop shapes stay
in the VM.

OSR preserves exact numerics, Unit/Option/Result/collection semantics, pending
traps, precise GC roots, resource handles, output, deterministic metering,
deadlines, and program arguments. Workload reports state separately when no
benefit is possible before OSR; Brainfuck Mandelbrot interpreted by lkjscript
is the principal long-running-loop acceptance workload.

## Native ABI And Representations

Statically known native values do not use the universal tagged VM `Value` in
hot paths:

- I64: native 64-bit integer;
- F64: native floating-point value;
- Bool: one canonical integer representation;
- Unit: zero-sized or canonical no-value representation;
- Str, Bytes, Slice, and views: typed pointer/length layouts;
- products: flattened or target-ABI layouts;
- Option: a proven typed niche or explicit tag;
- heap references: typed references visible to precise stack maps.

Explicit adapters own VM/native and runtime/native transitions. Native callers
call compiled callees directly without repeated boxing where compatible. Host
operations use a small versioned runtime ABI rather than embedding Rust
implementation details in generated code.

## Executable Code Objects

A callable code object records at least:

- entry address and code size;
- semantic and native ABI versions;
- source function or region identity;
- tier identity;
- relocation metadata;
- safepoints and precise GC stack maps;
- OSR entries when present;
- trap and side-exit metadata;
- accounted metadata and executable sizes;
- invalidation state.

Linux x86-64 executable memory follows W^X: allocate writable, emit and
relocate, transition to executable and non-writable, then execute. Pages are
never writable and executable simultaneously. Unsafe OS interaction remains in
`lkjscript-sys`; its safe API upholds Rust safety for every caller.

The code cache has configurable limits for executable bytes, code-object count,
compilation time, active or queued work, and metadata bytes. Start with bounded
retention. Eviction is **Deferred** until return addresses, direct calls, OSR
relationships, and invalidation are modeled safely.

## GC, Safepoints, And Outcomes

Native safepoints are required at allocations, allocation-capable host and
function calls, loop backedges, yields/deadline checks, and VM/native
transitions. Every safepoint has an exact map of live references. Arbitrary
machine words are never conservatively scanned as pointers.

Native frames become visible to the collector before allocation-capable native
code is accepted. Baseline code may initially call the existing allocator and
collector; escape analysis, regions, stack allocation, and generational
strategies remain later measured candidates.

Callable native execution does not begin until chunk validation, structured
process-safe VM outcomes, process-safe exit, explicit traps, exact local/stack
initialization, and the still-required semantic migrations are complete. The
JIT returns the same structured outcomes as the VM. Generated code does not
directly terminate the host process for language exit, trap, timeout, or a
resource limit.

## Minimal AOT Test Surface

File-based native emission is retained only where it shares the native backend
and helps:

- inspect generated machine code;
- test relocations and calling conventions;
- compare output and structured outcomes with the VM;
- use external disassemblers and debuggers;
- validate code generation without executable-memory machinery;
- measure backend compilation independently.

This test/AOT harness does not require training workloads and is not the main
optimization or product strategy. It must not grow profile generation, profile
merging, profile-use rebuilds, PGO-specific decisions, or persistent profile
formats.

## Engine Selection Contract

The current CLI implements all four modes below. Ordinary `run` defaults to
`auto` with a conservative 64-entry threshold, while explicit `vm` remains the
deterministic path:

```text
lkjscript run --engine vm <file.lkjscript> [--] [script-args...]
lkjscript run --engine auto <file.lkjscript> [--] [script-args...]
lkjscript run --engine baseline-jit <file.lkjscript> [--] [script-args...]
lkjscript run --engine optimizing-jit <file.lkjscript> [--] [script-args...]
```

Semantics are:

- `vm`: never execute JIT code;
- `auto`: start in the VM, collect bounded hotness, and tier automatically;
- `baseline-jit`: require the executable entry region and every reached user
  function to compile and execute at baseline native tier when first reached;
  a reached unsupported function is an error rather than a VM fallback (host
  runtime calls are explicit ABI calls, not required user functions);
- `optimizing-jit`: require the executable entry region and every reached user
  function at the proof-based optimizing tier; if Tier 2A is unavailable the
  engine is rejected before execution, and it never downgrades to baseline or
  VM while reporting optimizing success.

Normal defaults preserve fast startup and do not compile tiny programs merely
to demonstrate JIT existence. JIT diagnostics use stderr or separate structured
output and never contaminate program stdout.

## Benchmark Contract

Retained tiered results compare applicable variants among:

```text
reference VM
baseline JIT including compilation
baseline JIT steady state
optimizing JIT including compilation
optimizing JIT steady state
```

A result records repository/tree identity, workload and oracle hashes, CPU and
target mode, OS/kernel, tool versions, warmup, trigger latency, compilation
time, first-native-execution time, end-to-end and steady-state time,
time-to-break-even, compilation/OSR/fallback/deoptimization counts, native code
and metadata bytes, peak RSS/code cache, repetitions, dispersion/tails, and
artifact cleanup.

Compilation and warmup cost are part of the result. A faster native steady state
is not an end-to-end speedup when total execution remains slower.

Required workload classes include cold hello, direct lkjscript Mandelbrot,
numeric loops, call-heavy recursion, allocation and branch stress, byte/buffer
processing, a long OSR loop, one-shot HTTP, repeated warm server requests, and
Brainfuck Mandelbrot interpreted by lkjscript. Direct and
Brainfuck-interpreted Mandelbrot remain distinct workloads.

## Adoption Gates

Baseline JIT acceptance requires:

1. byte-for-byte output and return-value equality with the VM;
2. identical structured traps, errors, malformed-input behavior, and exact
   numerics;
3. GC correctness across native safepoints;
4. identical resource-limit behavior;
5. VM/native and native/native call correctness;
6. bounded code and metadata caches;
7. forced mode evidence that native code actually executed;
8. repeated performance evidence including compilation cost.

OSR additionally requires exact locals/stack/loop-header reconstruction, GC
correctness during transfer, timeout/cancellation behavior, repeated transfers,
and a workload that enters native code during one long invocation.

Proof-based optimizing JIT additionally requires differential SSA optimization
tests, isolated and combined variants, compile-time and code-size budgets,
whole-program benefit on a declared workload, and no hidden material retained
workload regression.

Guarded specialization and deoptimization have separate gates and are not
implied by a non-speculative optimizing tier.

## Implementation Sequence

1. With explicit equality current, finish explicit-main/effect-free-import,
   local-mutation/immutable-global, fixed-point-effect, chunk-validation, and
   structured-outcome prerequisites.
2. Implement typed SSA, verifier, differential evaluator, and isolated proven
   optimization passes.
3. Implement the selected owned
   [Linux x86-64 native backend](linux-x86-64-native-backend.md), shared native
   code-object boundary, and minimal non-PGO file emitter; add relocations, ABI
   tests, runtime calls, and precise stack maps before allocation paths.
4. Add bounded call counters and synchronous function-triggered baseline JIT,
   VM/native calls, direct native calls, and forced baseline testing.
5. Add sound ownership/coherent traits, exact native frame roots, allocation,
   barriers, recursion, and collection exercised while generated frames are
   active.
6. Add a distinct proof-based optimizing engine, then implement the selected
   process-local synchronous promotion boundary and run its retained threshold
   gate. The forced first engine is Current; automatic promotion remains
   disabled by default and unimplemented.
7. Add bounded loop-backedge counters and verified OSR in a later cycle; use
   unmodified Brainfuck Mandelbrot only as an appropriate long-loop diagnostic.
8. Consider guarded specialization and deoptimization only from measured need.
9. Validate Linux AArch64, direct Wasm, and server policies later; background
   compilation and persistent profiling require their own prerequisites or a
   new decision.

## Rejected

- Offline or ahead-of-time PGO as an accepted target.
- Persistent cross-run JIT profiles or native-code caches in this plan.
- Calling the observation hook a JIT.
- Calling emitted but unexecuted machine code a baseline JIT.
- Calling next-invocation compilation OSR.
- Calling abort or whole-program restart deoptimization.
- Hiding compilation cost behind steady-state-only claims.
- Carrying universal tagged VM values through typed native hot paths.
- A backend that independently interprets untyped syntax.
- Writable-and-executable pages.
- Background compilation before process-safe ownership and cancellation.
- Unchecked assumptions or undefined behavior as optimization mechanisms.
