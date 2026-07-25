# Proof-Based Optimizing JIT

## Purpose

Define a distinct non-speculative optimizing tier whose transformations are
proved by verified typed-SSA facts and whose generated code is actually called.

## Status

Deterministic baseline SSA normalization is **Current**. An optimizing engine,
optimizing code objects, optimized execution, and automatic promotion are an
**Accepted Target**. No baseline code is labeled optimizing.

## Selected First Delivery

**Accepted Next Slice; not Current until forced generated execution and retained
measurement pass.** The first optimizing pipeline is deliberately narrower than
the full progression below:

1. scalar algebraic identities whose exact typed operation proves the rewrite;
2. dominator/same-block global value numbering for exact deterministic scalar
   expressions, including a duplicate checked operation only after the first
   identical operation proves the same check succeeded;
3. ordinary verified copy/branch/unreachable/dead-code cleanup.

Every rewrite is represented by a stable-ID edit certificate. An independent
bounded verifier recomputes type, operation, operand, dominance/order, effect,
trap, ownership, frame-state, and safepoint legality, applies only certified
edits to a private clone, compares the exact candidate, and then runs the normal
SSA verifier. Only an opaque `VerifiedOptimizedProgram` reaches optimizing
lowering. Missing, stale, reordered, excessive, or forged edits fail closed.

The selected forced engine is `--engine optimizing-jit`. It compiles the full
required supported group before source effects, installs only `Tier::Optimizing`
objects, actually enters optimized main, and never substitutes baseline or VM.
It reuses ABI-2 exact roots, allocation runtime sites, active-frame bounds, W^X
installation, and structured outcomes. It adds no guards, deoptimization, OSR,
background compilation, or hidden source assumptions.

Automatic promotion remains disabled until same-commit forced correctness and
benchmark evidence select thresholds. Inlining, general SCCP, range/check
elimination, LICM, escape/scalar replacement, tail calls, unrolling, hot/cold
layout, and host-capability optimization remain later accepted slices; the
small first pipeline is not represented as completing those items.

## Engine And State

The accepted forced engine is:

```text
lkjscript run --engine optimizing-jit program.lkjscript
```

It optimizes every required reachable supported user function, verifies each
pass result, installs objects identified as optimizing tier, invokes optimizing
entries, and fails visibly if required compilation is unavailable. It never
downgrades to baseline or VM while reporting success.

The tier state is:

```text
VmOnly
Observed
BaselineCompiling
BaselineNative
OptimizingCandidate
OptimizingCompiling
OptimizedNative
Disabled
Invalidated
```

Compilation remains synchronous, single-owner, bounded, and non-reentrant.
Function-entry promotion is not OSR.

## Optimization Authority

Verified typed SSA is the only optimization IR. Passes consume resolved
function, trait implementation, generic instantiation, ownership, lifetime,
alias, effect, range, layout, safepoint, and frame-state facts. The backend may
select instructions but may not invent semantic assumptions or reinterpret
source names.

The tier is proof-based and non-speculative. There are no guards,
deoptimization, undefined-behavior assumptions, offline profiles, or
persistent code caches.

## Initial Pass Pipeline

Each pass has a separate deterministic implementation and work/code-growth
budget. The accepted progression is:

1. sparse conditional constant propagation;
2. copy propagation, branch simplification, and effect-aware DCE;
3. bounded direct-call inlining and resolved trait-call direct dispatch;
4. bounded common-subexpression elimination/global value numbering;
5. borrow-derived alias refinement;
6. range analysis and proven bounds/tag-check elimination;
7. loop-invariant code motion;
8. escape analysis, aggregate scalar replacement, and stack allocation;
9. proof-based self-tail-call elimination;
10. strength reduction and measured bounded loop unrolling;
11. current-process hot/cold block layout.

Only passes with evaluator/VM differential evidence and an adoption reason
become Current. Vectorization is **Deferred** until exact alias and target
legality facts exist and measurement justifies it.

## Escape And Identity

Allocation may be removed only when ownership, lifetime, escape, effects, and
identity prove it unobservable. Products and Option/Result payloads may remain
scalar values or stack storage when no identity-observing operation, escaping
reference, finalizer, runtime call, or safepoint requires an object. Otherwise
the allocation and exact root remain.

Bounds and tag checks are removed only for a proven range/tag fact. The exact
check remains whenever proof is absent.

## Pass Discipline

Every pass:

- verifies its input and output;
- is deterministic under stable identities;
- has explicit work, growth, depth, and iteration limits;
- preserves effects, ownership, drop, roots, safepoints, and frame states;
- has focused positive and negative tests;
- is differentially compared with the independent evaluator and VM;
- exposes bounded before/after diagnostics and pass statistics.

A budget exhaustion leaves a verified less-optimized program; it does not skip
verification or weaken semantics.

## Automatic Promotion

After forced correctness and performance acceptance, `auto` may promote a
baseline-native function using bounded ephemeral process-local counters. The
policy considers baseline entries, estimated remaining work, compile cost,
function size, loop presence, allocation rate, prior failures, and code-cache
budget. Candidate thresholds are measured before a default is selected. Tiny
functions are not promoted where compile cost cannot reasonably be repaid.
Counters are discarded at process exit and are not telemetry.

## Metrics And Acceptance

Metrics distinguish baseline and optimizing compile/pass/install/entry times,
object bytes, metadata, cache peaks, allocation/collection facts, failures,
fallbacks, and exact tier transitions. Forced tests require nonzero optimizing
entries and zero baseline/VM downgrade.

The aspirational adoption gate is at least 1.20x optimizing native execution
speed over same-commit baseline on one declared general workload, with no
hidden material retained-workload regression and no more than 5% median scalar
native regression absent an accepted cross-workload decision. Negative results
remain recorded and do not permit a false Current claim.

## Deferred And Rejected

OSR, background compilation, speculation, guards, deoptimization, vectorization,
persistent profiles/caches, and offline PGO are **Deferred** or **Rejected** as
specified above. Backend-only hidden optimization, unchecked hints, benchmark-
specific passes, and calling baseline code optimizing code are **Rejected**.
