# Proof-Based Optimizing JIT

## Purpose

Define a distinct non-speculative optimizing tier whose transformations are
proved by verified typed-SSA facts and whose generated code is actually called.

## Status

Deterministic baseline SSA normalization and the forced first proof-based
pipeline described in Selected First Delivery are **Current** on Linux x86-64.
Automatic promotion and every broader optimization listed below remain
**Accepted Targets**. No baseline code is labeled optimizing, and `auto` remains
baseline-only.

## Selected First Delivery

**Current forced slice.** The first optimizing pipeline is deliberately narrower
than the full progression below:

1. scalar algebraic identities whose exact typed operation proves the rewrite;
2. dominator/same-block global value numbering for exact deterministic scalar
   expressions, including a duplicate checked operation only after the first
   identical operation proves the same check succeeded;
3. ordinary verified copy/branch/unreachable/dead-code cleanup.

Every rewrite is represented by a stable-ID edit certificate. An independent
bounded checker builds checker-private immutable definition, type, constant,
expression, predecessor, reachability, and dominance indexes. It does not call
the discovery identity/GVN routines, discovery eligibility predicates or
discovery dominator implementation. The checker independently derives exact
type, operation, operand, dominance/order, effect, trap, ownership,
frame-state, and safepoint legality, applies the ordered records to a private
input clone, compares the exact candidate by reference (including F64 bits),
and then runs the normal SSA verifier. Only an opaque
`VerifiedOptimizedProgram` reaches optimizing lowering. Missing, stale,
reordered, excessive, or forged edits fail closed.

The selected forced engine is `--engine optimizing-jit`. It compiles the full
required supported group before source effects, installs only `Tier::Optimizing`
objects, actually enters optimized main, and never substitutes baseline or VM.
It reuses ABI-2 exact roots, allocation runtime sites, active-frame bounds, W^X
installation, and structured outcomes. It adds no guards, deoptimization, OSR,
background compilation, or hidden source assumptions.

Automatic promotion remains disabled. The retained `063668e` forced-tier
protocol measured a 2.930761x optimizing native-execution speedup over
same-commit baseline on the declared workload, and the improvement exceeded
its noise gate. The complete adoption verdict is nevertheless **Rejected**
because the separate historical scalar sentinel's native median regressed
6.99%, above the predeclared 5% ceiling. This does not reverse the Current
forced correctness slice, but it prevents a retained optimizing performance
adoption claim and does not select promotion thresholds. Inlining, general
SCCP, range/check elimination, LICM, escape/scalar replacement, tail calls,
unrolling, hot/cold layout, and host-capability optimization remain later
accepted slices; the small first pipeline is not represented as completing
those items.

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

## Current First-Pipeline Boundary

`lkjscript-ir` exposes bounded `optimize` and separate `verify_optimization`
boundaries. Only the latter can create the opaque `VerifiedOptimizedProgram`
used by optimizing lowering. Ordered records retain stable function, block, and
value IDs plus the expected canonical operation, exact operands, rewrite family,
and replacement. Public preflight limits functions, blocks, function and block
parameters, instructions, operands, frame facts, recursive type nodes, metadata
items, aggregate string/metadata bytes, certificate records and certificate
bytes before cloning or general verification. Work, allocation-sized index
construction, dominance intersections, expression probes, comparisons,
certificate storage, reconstruction, cleanup, and final validation consume one
aggregate configured work cap. Internal optimization also uses one budget across
discovery and independent checking; cleanup iterations and instruction growth
are aggregate caps rather than fresh per-phase allowances.

The current algebraic vocabulary is I64 xor/or with zero, I64 and with all-ones,
idempotent I64 and/or, and exact Bool double-not. Current GVN handles identical
deterministic scalar comparisons, Bool not, I64 bit operations, F64 arithmetic,
and checked I64 arithmetic/division only when the earlier identical dominating
operation establishes successful completion. Allocation, identity/memory/host,
affine, root, frame-state, and safepoint operations are ineligible. Existing
verified copy, branch, unreachable, empty-block, effect-aware dead-code,
direct-call, and canonical-order cleanup follows certified edits.

The opaque verified input is preflighted, and the independent checker recomputes
the canonical complete record sequence without consuming discovery state,
checks every record and budget, applies edits to a private clone, verifies that
edit-stage SSA, verifies every cleanup stage, requires exact equality with the
supplied candidate, and runs ordinary SSA verification again. Discovery and the
checker ignore unreachable blocks for rewrites; ordinary unreachable cleanup
therefore removes valid disconnected diamonds and loops deterministically,
while reachable dominance matches the ordinary verifier's path mathematics.
Missing, stale, reordered, forged, non-dominating, effectful, and over-budget
proofs fail closed.

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

Budget exhaustion never mutates or invalidates the verified input. The current
forced engine reports it as a visible engine error before effects; a later
automatic policy may retain a separately verified less-optimized tier without
claiming the failed optimizing object. No path skips verification or weakens
semantics.

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
fallbacks, and exact tier transitions. Optimization statistics retain separate
executed discovery, checker, reconstruction, cleanup, and ordinary-validation
pass counters; `optimizing_passes` is their actual optimizing-pass total rather
than a value inferred from object and iteration counts. Certificate and retained
optimization metadata byte fields are explicitly named `_estimate`: the
certificate estimate is 8 bytes of canonical header plus 31 fixed bytes and 4
bytes per operand for each record, and retained metadata adds eight bytes per
reported scalar statistic. These are deterministic accounting formulas, not
Rust allocator-size claims. Forced tests require nonzero optimizing entries and
zero baseline/VM downgrade.

The retained forced workload demonstrates 2,724 optimizing versus 13,956
baseline generated code bytes, 72 checked-I64 GVN records, and 10,001
optimizing entries with zero baseline entry or fallback. Current-thread stack
bounds are queried once per invocation rather than once per generated frame
reservation; every reservation still uses the cached guarded bounds.

At `063668e`, four warmups and 31 measured samples per forced case in one
deterministically randomized interleaving produced native medians of 1.997375
ms baseline (MAD 0.016721 ms) and 0.681521 ms optimizing (MAD 0.003567 ms):
2.930761x. The 1.315854 ms improvement exceeded twice the combined MAD,
0.040576 ms. Exact I64 `3333`, optimizing entries, zero downgrade, W^X, proof,
and code-byte gates all passed. Same-commit scalar forced baseline measured
8.182742 ms native and 9.340049 ms process wall. Against the retained callable
baseline's 7.647935 and 9.372036 ms, the ratios were 1.069928 and 0.996587.
Thus process wall passed while native execution exceeded the 5% regression
ceiling.

The mechanically complete adoption verdict is **Rejected**, despite the
workload-local optimizing win. The scalar comparison is a sentinel across code
evolution rather than an attribution experiment: its source is unchanged, but
compiler, metrics, native ABI, stack checks, binary, and surrounding generated
code evolved since the retained baseline. Negative evidence remains visible;
a later accepted cross-workload decision or repaired scalar result requires a
new predeclared protocol rather than reinterpretation. Automatic promotion is
still disabled and unmeasured.

## Deferred And Rejected

OSR, background compilation, speculation, guards, deoptimization, vectorization,
persistent profiles/caches, and offline PGO are **Deferred** or **Rejected** as
specified above. Backend-only hidden optimization, unchecked hints, benchmark-
specific passes, and calling baseline code optimizing code are **Rejected**.
