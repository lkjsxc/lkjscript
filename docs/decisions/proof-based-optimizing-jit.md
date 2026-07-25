# Proof-Based Optimizing JIT

## Purpose

Define a distinct non-speculative optimizing tier whose transformations are
proved by verified typed-SSA facts and whose generated code is actually called.

## Status

Deterministic baseline SSA normalization and the forced first proof-based
pipeline described in Selected First Delivery are **Current** on Linux x86-64.
The exact automatic baseline-to-proof-optimizing slice below is an **Accepted
Implementation Selection**, not yet Current. Every broader optimization listed
below remains an **Accepted Target**. No baseline code is labeled optimizing,
and the implemented `auto` remains baseline-only.

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

Automatic promotion remains disabled. The clean `cc967ff` forced-tier protocol
measured a 2.984780x optimizing native-execution speedup over same-commit
baseline on the declared workload, exceeded its noise gate, and passed both
historical scalar gates. Every predeclared criterion passed, so performance
of the forced first proof-optimizing tier is **Adopted**. This does not select or
measure promotion thresholds and makes no OSR, deoptimization, or speculation
claim. Inlining, general SCCP, range/check elimination, LICM, escape/scalar
replacement, tail calls, unrolling, hot/cold layout, and host-capability
optimization remain later accepted slices; the small first pipeline is not
represented as completing those items.

## Engine And State

The accepted forced engine is:

```text
lkjscript run --engine optimizing-jit program.lkjscript
```

It optimizes every required reachable supported user function, verifies each
pass result, installs objects identified as optimizing tier, invokes optimizing
entries, and fails visibly if required compilation is unavailable. It never
downgrades to baseline or VM while reporting success.

The Current implementation's tier state is:

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
Function-entry promotion is not OSR. The selected automatic state model below
supersedes this list only when its implementation becomes Current.

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

## Accepted Implementation Selection: Automatic Baseline-To-Proof Promotion

This section is authoritative for the next implementation slice and is **not
yet Current**. It selects a process-local, synchronous auto session. Existing
automatic baseline behavior is unchanged: ordinary `auto` still uses the
64-VM-entry baseline threshold, the threshold call compiles synchronously but
runs in the VM, and only a later call may select baseline native code.
Automatic optimizing promotion will initially be disabled by default and
available only through explicit CLI opt-in, with a separate deterministic
optimizing-threshold control. The retained gate will test exact optimizing thresholds 64,
256, 1,024, and 4,096 baseline entries; no default optimizing threshold is
selected by this record.

Optimizing hotness counts exact baseline-native entries of the promotion root.
VM entries, generated helper entries, direct native callees, compile attempts,
and install events do not increment it. An exact opaque entry token identifies
the source function, owning code object, and tier; counters, current selection,
and invocation all validate that token rather than matching a source name or
raw address.

On the Nth baseline entry, the session captures the exact current baseline
object and entry token, transitions synchronously through proof optimization,
lowering, and bounded W^X installation, and retains the resulting optimizing
object as pending. That triggering entry must still invoke the captured
baseline object. Only a later root entry may publish and select the pending
optimizing object, so the earliest optimizing entry is N+1. Publication checks
that the captured baseline is still current and both tokens remain valid.
There is no on-stack replacement, background compilation, deoptimization,
guard, or speculative assumption.

The selected automatic states are:

```text
BaselineCandidate
BaselineCompiling
BaselineNative
OptimizingCandidate
OptimizingCompiling
OptimizingPending
OptimizingNative
Disabled
```

The state and exact object ownership are explicit: once baseline is installed,
an auto-native entry has one selectable current object and at most one non-
selectable pending object. Baseline and optimizing objects coexist under exact
session ownership after
promotion; selecting optimizing code does not destroy its baseline recovery
object. Invalidated or superseded objects are marked stale, unlinked from all
selectable entry tokens, and retained under a bounded stale-object/mapping
budget until synchronous session drop. A stale object is never selectable or
invocable.

Each tier records saturating exact-entry counters, the explicit configuration/
resource epoch, attempts, structured failures, and same-epoch suppressions.
There is at most one optimizing attempt per epoch and a bounded total attempt
count. Failure keeps the baseline object current and records a structured tier
reason; another entry in the same epoch is suppressed rather than retried. A
newer explicit epoch permits at most one further attempt within the total
bound. It also invalidates any pending or current optimizing object and selects
the still-valid baseline object before counting new promotion eligibility.
Promotion transitions to `Disabled` when the total attempt bound is exhausted
or the root is permanently unsupported; baseline remains current. Reentrancy cannot observe or publish
an object in `OptimizingCompiling` or `OptimizingPending`.

The auto VM adapter remains scalar-only at function entry. A generated group
may contain reference-signature helpers that call, allocate, collect, and use
exact roots internally, but such helpers cannot transfer references between VM
and native code, cannot become auto entry roots, and cannot increment a root's
promotion count. Source main remains in the VM. Forced `baseline-jit` and
`optimizing-jit` semantics are unchanged: each requires its requested tier and
reports an engine error rather than falling back.

## Selected Automatic Metrics And Limits

The implementation must expose the configured baseline/optimizing enables and
thresholds; exact epoch; per-tier attempts, failures, and same-epoch suppressed
attempts; every state transition; exact baseline-root entries before the first
optimized entry; time from session start and threshold trigger to first
optimized entry; and exact entries, opaque object IDs, entry tokens, and code
bytes by tier. It also retains proof/certificate/checker facts, W^X transition
and mapping accounting, stale-object invalidations, current/pending selection,
and zero-fallback evidence. Normal stdout remains untouched.

Limits separately bound current plus stale retained mappings and bytes, object
count, per-epoch and total attempts, optimizer discovery/check/reconstruction/
cleanup work, certificate records/bytes, code and retained metadata. Hitting a
limit is a structured tier failure; it cannot make an object selectable, weaken
proof checking, or evict a still-callable object.

## Predeclared Automatic Promotion Benchmark

The implementation does not become enabled by default without a retained clean,
locked release result. The standard-library harness must use deterministic
randomized ordering with at least four warmups and 31 unremoved measured samples
per case. It compares auto baseline-only with auto optimizing opt-in at exact
thresholds 64, 256, 1,024, and 4,096, plus unchanged forced-baseline and forced-
optimizing sentinels and allocation/reference correctness runs. Every sample
checks the exact independent oracle, stdout/stderr expectations, state and token
transitions, tier/object entries, proof/certificate acceptance, W^X, bounded
attempts, and zero forced or auto fallback.

A candidate is adoptable only if all of these predeclared criteria pass:

1. median end-to-end process time is at least 1.10x faster than auto
   baseline-only;
2. its baseline-only median improvement is greater than twice the sum of the
   two process-time MADs;
3. its nearest-rank p95 process time is no more than 5% worse than baseline-only;
4. measured optimization, lowering, and install cost is repaid before workload
   completion, both in the recorded break-even entry and cumulative saved time;
5. exact results, streams, state/token/object transitions, proof, W^X,
   allocation/reference behavior, and limits all pass;
6. each epoch has at most one attempt, there is no repeated attempt or fallback,
   and the first optimizing entry occurs only after exactly N baseline entries;
7. the unchanged forced scalar sentinel's native and process medians are each
   no more than 5% above the retained 7,647,935 ns and 9,372,036 ns historical
   medians.

Among candidates passing every criterion, the selected threshold is the
largest whose process median is statistically indistinguishable from the
fastest passing candidate: the absolute median difference must be no greater
than twice the sum of their process MADs. If no candidate passes, automatic
optimizing remains disabled by default and the complete rejection, including
all samples and attempted thresholds, is retained. A passing benchmark still
requires a later documentation/implementation change to make its selected
default Current.

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

The adopted forced workload demonstrates 2,424 optimizing versus 13,656
baseline generated code bytes, 72 checked-I64 GVN records, and 10,001
optimizing entries with zero baseline entry or fallback. Current-thread stack
bounds are queried once per invocation rather than once per generated frame
reservation; every reservation still uses the cached guarded bounds.

At `cc967ff`, four warmups and 31 measured samples per forced case in one
deterministically randomized interleaving produced exact native medians of
1,999,889 ns baseline (MAD 10,469 ns) and 670,029 ns optimizing (MAD 2,174 ns):
2.984780x. The 1,329,860 ns improvement exceeded twice the combined MAD,
25,286 ns. Process-wall medians were 3,565,363 and 2,411,023 ns, a 1.478776x
speedup. Exact I64 `3333`, optimizing entries, zero downgrade, W^X, proof, and
code-byte gates all passed. Same-commit scalar forced baseline measured
7,982,586 ns native and 9,207,038 ns process wall. Against the retained callable
baseline's 7,647,935 and 9,372,036 ns, the ratios were 1.043757 and 0.982395;
both passed the 1.05 ceiling. Every predeclared criterion passed, so the forced
first-tier performance verdict is **Adopted**.

The earlier `063668e` run remains retained and **Rejected**. Its local optimizer
result was 2.930761x, but scalar native measured 8,182,742 versus 7,647,935 ns,
a failing 1.069928 ratio; scalar process wall passed at 9,340,049 versus
9,372,036 ns, ratio 0.996587. The later performance recovery followed folding
the mandatory generated-function entry poll into ABI-2 frame registration,
removing a separate runtime transition without weakening polling or optimizer
proofs. Neither cross-commit scalar comparison attributes performance to the
optimizer, and adoption does not erase the first run's negative evidence.
Automatic promotion remains disabled and unmeasured. Its next implementation
and benchmark are selected above but are not Current; OSR, deoptimization, and
speculation were not measured or added.

## Deferred And Rejected

OSR, background compilation, speculation, guards, deoptimization, vectorization,
persistent profiles/caches, and offline PGO are **Deferred** or **Rejected** as
specified above. Backend-only hidden optimization, unchecked hints, benchmark-
specific passes, and calling baseline code optimizing code are **Rejected**.
