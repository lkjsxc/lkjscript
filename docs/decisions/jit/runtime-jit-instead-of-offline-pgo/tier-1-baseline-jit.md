# Runtime JIT Instead of Offline PGO: Tier 1: Baseline JIT

[Authority](../runtime-jit-instead-of-offline-pgo.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Tier 1: Baseline JIT

The baseline JIT synchronously compiles a whole eligible function from typed SSA
at a verified tier boundary. Its priorities are low compilation latency, exact semantics,
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
[Proof-Based Optimizing JIT](../proof-based-optimizing-jit.md).

Initial compilation is synchronous at a verified tier boundary. Background
compiler threads are **Deferred** until VM outcomes, runtime-value access, code-cache ownership,
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
traps, typed structural homes, cleanup obligations, resource handles, output,
deterministic metering, deadlines, and program arguments. Workload reports state separately when no
benefit is possible before OSR; Brainfuck Mandelbrot interpreted by lkjscript
is the principal long-running-loop acceptance workload.
## Native ABI And Representations

Statically known native values do not use the universal closed VM `Value` in
hot paths:

- I64: native 64-bit integer;
- F64: native floating-point value;
- Bool: one canonical integer representation;
- Unit: zero-sized or canonical no-value representation;
- Str, Bytes, Slice, and views: typed pointer/length layouts;
- products: flattened or target-ABI layouts;
- Option: a proven typed niche or explicit tag;
- invocation references: typed values confined to exact frame homes and services.

Explicit adapters own VM/native and runtime/native transitions and preserve
scalar categories and bits without collector allocation. Native callers call
compiled callees directly where compatible. Host
operations use a small versioned runtime ABI rather than embedding Rust
implementation details in generated code.
