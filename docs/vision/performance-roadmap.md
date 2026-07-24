# Performance Roadmap

## Purpose

Define a measured runtime-JIT-first path toward category-leading performance
without turning aspiration into a current release claim.

## Status

The reference bytecode VM, exact I64/F64 execution, precise mark-sweep, resolved
typed HIR, verified normalized SSA, selected owned Linux x86-64 backend, bounded
code objects, and callable allocation-free scalar baseline JIT are **Current**.
The old observation hook is removed. Native references/allocation, loop OSR,
proof-based optimizing JIT, and direct Wasm are **Accepted Targets**. Guarded
specialization is **Deferred** until justified. Offline PGO is **Rejected by
Product Decision**.

## Sequence

```text
truthful semantics and safety
  -> reproducible category scorecard
  -> resolved typed HIR
  -> AI-first semantic core migration
  -> chunk validation and structured VM outcomes
  -> typed SSA, verifier, and differential evaluator
  -> owned Linux x86-64 native code-object backend
  -> function-triggered baseline JIT
  -> loop-triggered baseline JIT and OSR
  -> proof-based optimizing JIT
  -> guarded specialization and deoptimization only when justified
  -> direct Wasm and additional targets
```

The VM remains the cold tier, deterministic/debugging path, unsupported-platform
fallback, and correctness oracle. A minimal file-based native emitter remains a
shared-backend test surface; it is not an AOT-first or PGO strategy.

The authoritative tier, state-machine, executable-code, OSR, GC, failure,
engine-selection, and rejection contract is
[Runtime JIT Instead of Offline PGO](../decisions/runtime-jit-instead-of-offline-pgo.md).

## Current Interpreter

The VM uses dense bytecode, contiguous stacks, tagged small I64 values, boxed
wide I64/F64 values, precise non-moving mark-sweep collection, and
return-adjacent frame reuse. Source is compiled on every CLI invocation. Host
effects block synchronously. Linux x86-64 now has a callable scalar baseline
compiler, bounded code objects, explicit engines, and a retained scalar
performance result. Generated forced execution reached 46.146x same-commit VM
execution and auto reached 1.653x process-wall speedup at threshold 64 on the
100,000-call F64 workload. There is no native reference/allocation support,
OSR, optimizing tier, or deoptimization; the scalar result is not a
full-language claim.

Historical debug figures and single-shot C comparisons lack preserved machine,
variance, or artifact data and remain diagnostic rather than baselines. The
Brainfuck Mandelbrot workload now supplies a retained long-running-loop VM
result that will later expose the need for loop-triggered JIT and OSR.

## Phase 0: Policy Cutover — Current

The planning cutover now rejects offline PGO, makes runtime JIT the primary
adaptive strategy, defines the VM/baseline/proof-based/guarded tiers, and fixes
the contracts for local ephemeral hotness, synchronous compilation, states,
fallback, resource budgets, code objects, W^X, safepoints, OSR, and forced
testing. That policy is now realized by callable scalar forced/auto engines; the former
observation hook is removed.

## Phase 1: Semantic And Runtime Prerequisites — Current

Explicit main/effect-free imports, local-only `var`/`set`, product-threaded
state, fixed-point effects, whole-chunk validation, and structured process-safe
outcomes are Current prerequisites. Native eligibility remains narrower than
the VM and rejects any path whose GC roots, host effect, or resource behavior is
not implemented exactly.

## Phase 2: Typed SSA — Current

1. Lower resolved HIR to blocks with explicit parameters, exact types, effects,
   calls, and trap edges.
2. Implement an SSA verifier.
3. Implement a differential SSA evaluator or equivalent oracle.
4. Temporarily prove the existing bytecode lowering equivalent, then cut
   reference bytecode over to verified SSA before native lowering is called
   authoritative; delete the sibling semantic lowering.
5. Establish isolated non-speculative passes and differential pass tests.

Typed SSA is the only optimization authority. No independent
bytecode-to-machine-code semantic compiler is accepted.

## Phase 3: Native Code Objects — Current Scalar Subset

1. Define semantic and native ABI versions and typed representations.
2. Use only the implemented owned emitter selected by
   [Linux x86-64 Native Backend](../decisions/linux-x86-64-native-backend.md).
3. Retain opt-in generated-byte dumps for external disassembly/debugging and
   ABI/differential tests; there is no offline PGO or persistent cache.
4. Implement versioned runtime-call adapters and VM/native transitions.
5. Implement W^X executable memory through the safe `lkjscript-sys` boundary.
6. Add precise safepoints and stack maps before allocation-capable native paths.
7. Bound executable bytes, object count, compile time, work, and metadata.

Current scalar code objects own entries, code/page-accounted size,
source-group/tier identity, ABI versions, relocations, runtime calls, scalar
safepoints and exact empty reference maps, source/trap/outcome maps, compile and
resource accounting, invalidation, and native entries. Forced tests call them;
emission alone is still not a JIT claim.

## Phase 4: Function-Triggered Baseline JIT — Current Scalar Subset

1. Add bounded saturating function-entry counters.
2. Compile whole eligible functions synchronously at a safepoint.
3. Install and call baseline code objects.
4. Support exact scalar VM/native entry/return adapters and direct unboxed
   native-to-native calls inside a compatible compiled group.
5. Keep optimizations inexpensive and non-speculative.
6. Add forced baseline mode that errors rather than silently falls back.
7. Measure trigger, compilation, first native execution, end-to-end time,
   steady state, break-even, code cache, and fallbacks before performance
   claims. **Complete for the retained scalar workload.**

Ordinary `run` now uses `auto` with a conservative 64-entry threshold; explicit
`vm` remains deterministic, and auto leaves tiny or unsupported work there.
Thresholds 1/64/1,024 had overlapping process distributions on the retained
100,000-call workload. The selected middle value protects short programs while
reaching native code at median 0.297720 ms rather than waiting to the 1,024-call
median 3.556024 ms. The next performance dependency is loop-triggered OSR, not
lowering the function-entry threshold to manipulate one benchmark.

## Phase 5: Loop Hotness And OSR

1. Add bounded saturating loop-backedge counters without ordinary
   per-instruction counters.
2. Trigger compilation from long-running loops.
3. Define verified loop-header mappings from bytecode VM state to typed SSA and
   native frame locations.
4. Transfer exactly representable loops into baseline native code.
5. Leave unsupported loop shapes in the VM.
6. Validate GC, traps, deadlines, metering, output, resources, and arguments
   during and after transfer.
7. Measure OSR latency and whole-program benefit on long loops, especially
   Brainfuck Mandelbrot interpreted by lkjscript.

OSR is required rather than cosmetic. Compiling only for the next function
invocation is not OSR, and reports must state when a workload cannot benefit
before OSR exists.

## Phase 6: Proof-Based Optimizing JIT

Add measured passes justified entirely by static types, SSA, effects, ownership,
and proven control flow. Begin with constant propagation, branch cleanup,
dead-effect-free instructions, inlining under budgets, CSE, LICM, redundant
check elimination, scalar replacement, escape analysis, strength reduction,
and hot/cold layout from current-process counters. Add unrolling and
vectorization only where target and alias facts permit and measurements retain
them.

Promotion uses bounded current-process observations that are discarded on exit.
This tier remains non-speculative where possible and does not imply general
deoptimization.

## Phase 7: Guarded Specialization

Proceed only when retained evidence shows proof-based optimization is
insufficient. Every value/shape specialization has an explicit guard, exact
side exit, state reconstruction, usefulness/failure counters, and bounded code
and metadata cost. Deoptimization restores correct VM or lower-tier state; it is
not abort or whole-program restart.

Background compilation remains deferred until runtime ownership, cancellation,
heap access, code-cache synchronization, and outcomes are process-safe.
Persistent cross-run profiles and native-code caches remain outside the plan.

## Phase 8: Portability And Product Work

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

Every candidate follows [experiments.md](experiments.md) and the
[performance scorecard](performance-scorecard.md). Correct output, return
values, structured traps/outcomes, malformed-input behavior, GC, resource
limits, and ABI conformance precede timing. Forced modes prove native execution.

Applicable results compare VM, baseline including compilation, baseline steady
state, optimizing including compilation, and optimizing steady state. They
record trigger and compilation latency, first native execution, break-even,
compilation/OSR/fallback/deoptimization counts, code and metadata bytes, peak
RSS/cache, repetitions, dispersion/tails, and cleanup. A faster steady state is
not called an end-to-end speedup when total execution is slower.

The exact active completion boundary is
[Callable Linux x86-64 Baseline JIT Cycle](../decisions/callable-baseline-jit.md).

## Rejected And Deferred

**Rejected by Product Decision:** offline PGO, instrumented training builds,
profile generation/merge/use, persistent PGO artifacts, PGO-specific decisions,
and any gate requiring JIT to beat PGO AOT.

**Deferred:** guarded specialization/deoptimization until Tier 2A evidence,
background compilation until process-safe ownership, eviction until native
relationships are modeled, persistent profiles/caches pending a new explicit
decision, and non-Linux native backends until Linux x86-64 passes.
