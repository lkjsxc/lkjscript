# Performance Roadmap: Sequence

[Authority](../performance-roadmap.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Sequence

```text
truthful semantics and safety
  -> reproducible category scorecard
  -> resolved typed HIR
  -> AI-first semantic core migration
  -> chunk validation and structured VM outcomes
  -> typed SSA, verifier, and differential evaluator
  -> owned Linux x86-64 native code-object backend
  -> function-triggered scalar baseline JIT
  -> ownership, coherent traits, and exact native roots
  -> allocation-capable baseline JIT
  -> forced first proof-based optimizing JIT
  -> Semantic Source and aggregate resource-budget foundations
  -> selected synchronous automatic proof promotion and retained threshold gate
  -> broader proof passes
  -> shared AOT/artifact identity and measured cache candidates
  -> loop-triggered JIT and OSR in a later cycle
  -> guarded specialization and deoptimization only when justified
  -> optional explicit local PGO only after common AOT identity
  -> direct Wasm and additional targets
```

The VM remains the cold tier, deterministic/debugging path, unsupported-platform
fallback, and correctness oracle. A minimal file-based native emitter remains a
shared-backend test surface; it is not an AOT-first or PGO strategy.

The authoritative tier, state-machine, executable-code, OSR, GC, failure,
engine-selection, and rejection contract is
[Runtime JIT Instead of Offline PGO](../../decisions/jit/runtime-jit-instead-of-offline-pgo.md).
## Current Interpreter

The VM uses dense bytecode, contiguous stacks, complete inline I64 and exact-bit
F64 values, precise non-moving mark-sweep collection for structural values, and
return-adjacent frame reuse. Source is compiled on every CLI invocation. Host
effects block synchronously. Linux x86-64 now has a callable scalar baseline
compiler, bounded code objects, explicit engines, and a retained scalar
performance result. Generated forced baseline execution reached 46.146x
same-commit VM execution and auto reached 1.653x process-wall speedup at
threshold 64 on the 100,000-call F64 workload. Host-independent native
reference/allocation and the forced first optimizing tier now exist. The clean
retained forced protocol adopted that optimizing tier at 2.984780x native and
1.478776x process-wall speedup over same-commit forced baseline on its declared
workload; this remains a narrow forced-tier result. There is no automatic
optimizing promotion, OSR, guarded specialization, or deoptimization; the
scalar baseline and forced optimizing results are not full-language claims.

Historical debug figures and single-shot C comparisons lack preserved machine,
variance, or artifact data and remain diagnostic rather than baselines. The
Brainfuck Mandelbrot workload now supplies a retained long-running-loop VM
result that will later expose the need for loop-triggered JIT and OSR.
## Phase 0: Policy Cutover — Current

The historical planning cutover rejected offline PGO and made runtime JIT the
primary adaptive strategy. The later execution portfolio retains JIT primacy
while permitting measured AOT/cache and optional explicit local PGO after
shared identity. It defines the VM/baseline/proof-based/guarded tiers and fixes
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
   [Linux x86-64 Native Backend](../../decisions/execution/linux-x86-64-native-backend.md).
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
median 3.556024 ms. The next dependency is exact ownership-aware native roots
and allocation, not lowering the function-entry threshold to manipulate one
benchmark.
## Phase 5: Ownership, Native Roots, And Allocation

1. Establish sound affine ownership, lexical borrow, drop, and coherent core
   trait facts before exposing unrestricted references.
2. Extend typed SSA and verification with ownership, regions, alias classes,
   allocation, barriers, and exact frame/root facts.
3. Register every active native frame explicitly and validate non-empty stack
   maps at collecting safepoints.
4. Add versioned allocation and write-barrier runtime calls, then force
   collection while generated frames retain live references.
5. Add products, Option/Result, Str, bytes, byte-vector, List, and typed-resource runtime paths and bounded
   recursion without weakening forced-mode behavior.
6. Retain scalar native performance as a regression gate and measure general
   allocation/reference workloads against same-commit VM.

The exact completion boundary is [Allocation-Capable Baseline
JIT](../../decisions/jit/allocation-capable-baseline-jit.md). A helper call or emitted
root metadata without active native collection is not completion.
## Phase 6: Proof-Based Optimizing JIT

Add measured passes justified entirely by static types, SSA, effects, ownership,
and proven control flow. Begin with constant propagation, branch cleanup,
dead-effect-free instructions, inlining under budgets, CSE, LICM, redundant
check elimination, scalar replacement, escape analysis, strength reduction,
and hot/cold layout from current-process counters. Add unrolling and
vectorization only where target and alias facts permit and measurements retain
them.

Promotion uses bounded current-process observations that are discarded on exit.
This tier is non-speculative and does not imply general deoptimization. The
exact pass and forced-engine boundary is [Proof-Based Optimizing
JIT](../../decisions/jit/proof-based-optimizing-jit.md).

The next slice is an **Accepted Implementation Selection**, not Current. It
keeps auto baseline compilation at 64 VM entries and adds an initially disabled
CLI opt-in for proof promotion after exactly N baseline entries of one scalar-
entry root. The Nth baseline entry synchronously proves, lowers, and W^X-
installs but calls captured baseline code; only a later entry publishes the
pending optimized object. Exact tokens bind function/object/tier. Coexisting
baseline/optimizing objects, one current plus optional pending selection,
bounded unselectable stale retention, one attempt per epoch, total attempt
bounds, structured failures, same-epoch suppression, and epoch invalidation
back to baseline are mandatory. Main stays VM; generated reference helpers may
allocate internally but are not VM/native entries. Forced tiers do not change.

The retained clean locked release gate compares auto baseline-only with exact
optimizing thresholds 64/256/1,024/4,096 using deterministic randomized ordering,
at least four warmups and 31 samples, exact oracle/streams/state/proof/W^X,
forced sentinels, and allocation/reference correctness. Adoption requires at
least 1.10x median process speedup, improvement greater than twice combined MAD,
p95 no more than 5% worse, compile cost repaid, both historical scalar medians
within 5%, and no repeated attempt or fallback. Select the largest passing
threshold within twice combined MAD of the fastest passing process median;
otherwise retain the rejection and keep optimizing disabled.
