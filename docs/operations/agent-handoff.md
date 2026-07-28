# Agent Handoff

## Purpose

Capture exact Current capability, accepted next contracts, sharp edges, and
verification discipline for autonomous continuation.

## Status

<!-- LKJ-STATUS id=agent-work-state status=current -->
<!-- LKJ-STATUS id=repository-graph-context status=current -->
<!-- LKJ-STATUS id=repository-topology status=current -->
<!-- LKJ-STATUS id=resource-profile-compiler status=current -->
<!-- LKJ-STATUS id=resource-profile-preallocation status=current -->
<!-- LKJ-STATUS id=resource-profile-shared-ledger status=accepted-target -->
<!-- LKJ-STATUS id=semantic-core-target status=accepted-target -->
<!-- LKJ-STATUS id=semantic-session status=current -->
<!-- LKJ-STATUS id=semantic-source status=current -->
<!-- LKJ-STATUS id=typed-holes status=current -->
<!-- LKJ-STATUS id=jit-auto-promotion status=accepted-selection -->
<!-- LKJ-STATUS id=memory-obligations status=current -->
<!-- LKJ-STATUS id=memory-tracing-ratchet status=current -->
<!-- LKJ-STATUS id=modules-and-packages status=current -->
<!-- LKJ-STATUS id=memory-plan status=accepted-contract -->
<!-- LKJ-STATUS id=deterministic-drop status=accepted-contract -->
<!-- LKJ-STATUS id=generation-safe-resources status=accepted-contract -->
<!-- LKJ-STATUS id=collector-free-value-island status=accepted-contract -->
<!-- LKJ-STATUS id=collector-free-deterministic-memory status=accepted-contract -->

Repository topology and graph/context, bounded task state, exact modules and
packages, canonical Semantic Source and local sessions, explicit capabilities,
generic ADTs and structured control, validated VM, callable baseline JIT, and
forced proof JIT are Current. `lkjscript.memory-obligations` and its inventory
and explain commands are Current descriptive evidence. The machine tracing
ratchet and `memory traced` expose the exact eleven allowed `HeapObj` families.

The authoritative memory plan, deterministic whole-place drop,
generation-safe resource table, and first collector-free value island are
Accepted Contracts until executable acceptance passes. The whole runtime still
uses a tracing collector for structural values; collector-free deterministic
memory is not Current.

## Product Intent

- Build one AI-primary, statically typed, memory-safe language and platform.
- Canonical source uses `.lkjscript`; removed spellings and contracts have no
  aliases.
- Compiler, evaluator, VM, baseline JIT, proof JIT, package, and Semantic Source
  consume one typed semantic authority.
- Keep ordinary source free of lifetime names, retain/release, general `free`,
  raw pointers, and memory-engine switches.
- Preserve exact capabilities, effects, outcomes, budgets, W^X, content
  identities, and proof checking.
- Keep unsafe Rust confined to `lkjscript-sys` behind safe caller contracts.
- Add no third-party Rust dependency without accepted external review.
- Prefer complete vertical slices and focused conformance over mocks.

## Current Memory Foundation

The stable-index non-moving `GcHeap` still traces reference values, exact roots,
and generated native stack maps. Wide VM i64/f64 values may be heap boxed.
`buf` remains a traced mutable object, `bytes` is a source `PLACEHOLDER`, and
`path` remains a traced byte object.

`ExecutableProgram` retains a narrow independently recomputed SSA inventory for
direct byte-vector owners represented through transitional `buf`, byte loans,
and direct typed resources. It is not an authoritative pre-backend memory plan.
`place-end` may still discard an active owner fact. VM resources still use
monotonic opaque tokens, explicit close, and teardown safety. The replacement
core table has reusable generation keys and deterministic cleanup accounting,
but is not yet wired into VM/native execution or compiler cleanup edges.

## Current Non-Memory Boundaries

- Compiler authority is resolved typed HIR, verified SSA, and validated
  reference bytecode; no backend reinterprets source syntax.
- Imports and packages are exact and content-addressed. The canonical lowercase
  vocabulary remains Accepted Contract while transitional `buf` exists.
- Borrowing is a bounded direct whole-place slice; borrowed returns,
  projections, aggregate partial moves, and resource-bearing aggregates remain
  rejected.
- Forced native claims require synchronous generated entry with zero fallback.
- Collection roots remain required for non-island native functions.
- General regions, sealed regions, weak links, shared-node counting, pools, ECS,
  and collector-free closures/lists/products/enums are later work.

## Accepted Next Sequence

1. Establish the pre-backend authoritative memory plan and independent verifier.
2. Elaborate exact whole-place cleanup for byte owners and all typed resources.
3. Replace monotonic resource tokens with reusable generation-bearing slots.
4. Add deterministic generation-safe unique byte storage.
5. Implement bytes, byte-vector slices, path, and remove `buf` atomically.
6. Unbox complete i64 and exact-bit f64 in typed VM slots.
7. Execute and verify the exact island through evaluator, VM, forced baseline,
   and forced proof tiers with zero collector interaction and fallback.
8. Expose compiler-derived plans, owners, loans, storage, and cleanup to agents.
9. Ratchet remaining structural traced families downward.

This order is an implementation contract, not a Current capability claim.

## Change Discipline

Update authority before public behavior. Keep Current, Accepted Contract,
Accepted Target, Deferred, Rejected, superseded, and historical evidence
distinct. Analysis failure is a compile error, never a tracing fallback.
Generated outputs belong under `target/`; retain compact negative evidence and
remove reproducible temporary outputs.

Use [Verification](verification.md). Record only commands that ran, including
failed attempts and explicit untested gates. Each coherent commit includes
exact `Tested:` and `Not-tested:` trailers and passes the 16×200 topology gate.
