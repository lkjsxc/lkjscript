# Performance Roadmap: Phase 7: Loop Hotness And OSR — Later Cycle

[Authority](../performance-roadmap.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Phase 7: Loop Hotness And OSR — Later Cycle

1. Add bounded saturating loop-backedge counters without ordinary
   per-instruction counters.
2. Trigger compilation from long-running loops.
3. Define verified loop-header mappings from bytecode VM state to typed SSA and
   native frame locations.
4. Transfer exactly representable loops into native code.
5. Leave unsupported loop shapes in the VM.
6. Validate GC, traps, deadlines, metering, output, resources, and arguments
   during and after transfer.
7. Measure OSR latency and whole-program benefit on long loops, including
   unmodified Brainfuck Mandelbrot where a final diagnostic is justified.

Function-entry promotion does not accelerate one already-running invocation and
must never be labeled OSR. OSR is not required in the allocation/optimizing
cycle.
## Phase 8: Guarded Specialization

Proceed only when retained evidence shows proof-based optimization is
insufficient. Every value/shape specialization has an explicit guard, exact
side exit, state reconstruction, usefulness/failure counters, and bounded code
and metadata cost. Deoptimization restores correct VM or lower-tier state; it is
not abort or whole-program restart.

Background compilation remains deferred until runtime ownership, cancellation,
heap access, code-cache synchronization, and outcomes are process-safe.
Persistent cross-run profiles and native-code caches remain outside the plan.
## Phase 9: Portability And Product Work

Only after Linux x86-64 correctness and callable-JIT acceptance, validate Linux
AArch64 ABI assumptions, keep
direct Wasm aligned with typed SSA, measure server-oriented tier policies, and
consider product breadth without freezing representation defects. Browser,
package/update, GUI, and general server/framework products retain their own
gates.
## Native Representations

Reference tagged `Value` is not the native hot-path ABI. Native lowering uses
I64/F64/Bool scalars, typed pointer/length views, flattened products, specialized
Option layouts, and typed heap references visible in precise stack maps. Generic
code is monomorphized where measured code growth permits. Dynamic dispatch is
explicit rather than the default call path.

Vec, Slice, Bytes, Str, views, and fixed products are performance-default data
shapes. Candidate memory strategies include unique owned buffers, regions,
stack placement, worker-local generational collection, immutable shared bytes,
and explicit GC references where cycles require them. These remain measured
candidates, not assumed JIT results.
## Runtime Observation And Privacy

Function-entry, loop-backedge, and later selected block counters are bounded,
saturating, process-local, consumed by the same process, and discarded on exit.
They are never uploaded and are not telemetry. No training workload, profile
merge, profile-use rebuild, persistent profile artifact, or cross-run native
cache is part of this roadmap.
## Resource Modes

Normal safety mode checks deadlines/epochs at loop backedges, calls,
allocations, host calls, yields, and VM/native transitions. Deterministic
metering is a distinct explicitly slower block/instruction-counted mode. Heap,
stack, code cache, code metadata, compilation time/work, handles, tasks, queues,
IO volume, wall time, and allocation volume receive host-configurable limits;
unlimited execution is explicit trusted local mode.
## Benchmark And Adoption Rules

Every candidate follows [experiments.md](../experiments.md) and the
[performance scorecard](../performance-scorecard.md). Correct output, return
values, structured traps/outcomes, malformed-input behavior, GC, resource
limits, and ABI conformance precede timing. Forced modes prove native execution.

Applicable results compare VM, baseline including compilation, baseline steady
state, optimizing including compilation, and optimizing steady state. They
record trigger and compilation latency, first native execution, break-even,
compilation/OSR/fallback/deoptimization counts, code and metadata bytes, peak
RSS/cache, repetitions, dispersion/tails, and cleanup. A faster steady state is
not called an end-to-end speedup when total execution is slower.

The accepted repository-wide sequence is bounded topology, repository
intelligence graph/context, agent work state, first Semantic Source operations,
and aggregate budget profiles. Their authorities are indexed in [Platform
Decisions](../../decisions/platform/README.md). The automatic promotion
selection remains a later
exact implementation contract in [Proof-Based Optimizing
JIT](../../decisions/jit/proof-based-optimizing-jit.md). The callable baseline and
forced optimizing tiers remain the Current foundations.
## Rejected And Deferred

**Rejected:** mandatory uploaded telemetry, hidden fallback, incomplete cache
keys, and any gate requiring JIT to beat PGO AOT independent of workload cost.

**Deferred:** optional explicit local PGO until common SSA/AOT/artifact identity,
guarded specialization/deoptimization until Tier 2A evidence, background
compilation until process-safe ownership, cache eviction until native
relationships are modeled, production AOT/cache implementation until complete
artifact identity, and non-Linux native backends until the required foundation
slices pass.
