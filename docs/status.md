# Current status

Updated 2026-11-05 (Asia/Tokyo). Accepted typed meaning is the sole editable
program authority; maintained source proposals are adopted through public transactions.

## Current delivery boundary

Milestones A and C are accepted on main: complete maintained raw-source
self-compilation and same-process native invocation scratch reclamation.
The Profile 8 / Profile 7 closed pure callable batch is also accepted and
integrated on main as `8526ed77`, with exact tested tree `3de34681`.
Public graph adoption/readback, complete proposal replay, refreshed inventory,
raw-source `C2 = C3` and `Ahost = A1 = A2 = A3`, the pure-tail preflight,
all 29 fresh full gates and `release target` passed for that implementation.
Full `P_self` support remains open. No release archive or production deployment
is published. The candidate below has a separate acceptance obligation.

## Accepted capabilities

The accepted source compiler covers the maintained scalar/compound closure, structural
records/fields, `List<I64>`, lexical control flow, direct recursion, closed I64
list substitutions, exact intrinsic signatures and binary filters. ByteBuffer
consumption, synchronous borrows, branch-balanced disposal, reverse lexical
cleanup and returned-owner transfer remain checked; live custody prevents unsafe
tail transfer. Both ELF and canonical artifacts are built from complete raw source.

The accepted compiler adds FunctionValue, prefix Bind and Invoke, exact copyable
captures, callable fields/results, ordinary indirect calls and custody-aware
graph-tail frame reuse. Empty Bind preserves identity without an environment.
Text and StaticText retain distinct semantic identities. Rust Mode 1 emits all
five canonical callable operations. The [batch decision](decisions/20261101-closed-pure-callables-profile8.md)
records source/backend contracts, full identities and acceptance receipts.

Migrated opaque child ownership uses iterative cleanup. VM and reference have
independent iterative semantic comparison; raw structural observation remains a
separate fallible operation. The selected pure canonical-reference driver retains
graph activations and expression continuations on the heap, with exact custody,
call-depth/tail accounting, cancellation and modeled allocation obligations.
This does not make every reference route or generated native CALL stack unbounded.

## Mainline compiler graph

| Identity | Accepted implementation |
| --- | --- |
| Implementation commit / tested tree | `8526ed77` / `3de34681` |
| Public graph revision | `rev_73e5ce5e…` |
| Semantic state | `semantic_state_9f562cab…` |
| Complete carrier | Profile 3; 21,549,935 bytes; exact bindings in the batch decision |

The accepted readback covered all 69,148 owners across 17 pages. Its `F_active`
contains 123 modules, 89 supplied types and 1,199 callables (1,172 pure and 27
external), with 3,937 unrestricted, 329 consume and one borrow parameter, and all
36 generic application sites. It includes 58,718 expressions, 47,263 retirements
and 1,206 unit owners. Inventory is not a substitute for semantic/custody admission.

The accepted implementation uses `LKJSG002` profile 3, `LKJSRQ01` profile 2,
Mode-0 profile 8, Mode-1 profile 7 and canonical schema epoch 1.
Profile 6 has no Mode-0 fallback; incompatible derived inputs require regeneration.

## Accepted verification

| Selection on the exact tested tree | Result |
| --- | --- |
| Paired pure-tail preflight | Passed |
| Literal selfhost script | 264 passed, 0 failed, 9 ignored across three selections |
| Maintained native closure | 1 passed, 0 failed, 0 ignored |
| Complete owner/type inventory | 1 passed, 0 failed, 0 ignored |
| Explicit 8193-prefix crossing | 1 passed, 0 failed, 0 ignored |
| Whole maintained raw-source fixed point | 1 passed, 0 failed, 0 ignored |
| Complete compact full check | 29 fresh passed gates; 0 reused, failed or unrun |
| Release target | Passed; no release publication |

The accepted seal combines seven completed round-3 operations with the later
fresh compact full check and release-target invocation on the same unchanged
tree and original bootstrap verifier. The earlier archived full check's copied
gate executable is an explicitly recorded procedural deviation; its full/release
operations are excluded from repaired acceptance. The batch decision binds each
original receipt and records the independent behavior, rejection controls and
same-process recovery scope. Reconciliation on 2026-11-05 rechecked all raw
tracked files and 58 evidence files; that audit ran no compiler tests or gates.

The 8193-prefix crossing retains its explicit 32 GiB process allowance.
Its largest unit remains below the separate 4 MiB object and 8 MiB unit-codec
ceilings; it does not establish those capacity removals.

## Active candidate and remaining limits

The [ordinary nominal candidate](decisions/20261103-closed-ordinary-nominal.md)
extends both outputs with closed nongeneric acyclic copyable Record/Variant
values and exhaustive Match, including all-arm ByteBuffer custody. Its private
Mode-0/Mode-1 profiles are 9/8; the raw carrier, request and canonical schema
identities are unchanged. Forest proof now detects complete rooting during each
doubling pass, reducing required work for shallow forests while preserving exact
finite allowances and cycle rejection. This is an algorithm change, not a new
capacity-removal claim. Public adoption/readback passed at `rev_afe20e53…`: 80,845
live owners in 135 modules, 96 supplied types, 59,447 retirements and all 17
changed signatures. The active inventory contains 1,371 pure and 27 external
callables. Its complete Profile-3 carrier is 26,107,644 bytes; the decision binds
its digest and semantic state. Seventeen C0 native/producer controls passed.

The same-binary 64-block helper pair reduced If/Jump VM instructions from 2,171,227
to 447,208 while the straight control rose from 146,587 to 272,159. The decision
retains both modeled-byte changes and single-run timings; this is a whole-batch
helper comparison, with no isolated indexing or whole-compiler speed claim.
Straight-path overhead is follow-up work after current capability acceptance.
All 18 focused source/index/forest controls passed: raw nominal behavior and
rejection, 257-case dispatch, proposal replay and complete active-profile checks.
Renewed C2=C3 and Ahost=A1=A2=A3, the full frozen-tree seal and release-target
verification remain pending; mainline still ends at the accepted callable batch.

Tier 2 Milestone B remains open. OwnedRecord/OwnedChoice and affine
deconstruction, owned witnesses and source-tied read views, lexical Regions,
generic/recursive nominal values, general generic/effectful callables and
arbitrary list element types remain separate source families. Consecutive boxed-spine Clone admission remains
depth 256; structural type/AST/identity, owned-storage escape and codec paths keep
their separate bounds. Default call/value-stack policies, broader reference routes
and native CALL growth are unchanged. Owned-record layout preparation retains its
64 MiB cumulative metadata default and depth-256 guard. Canonical unit/object
envelopes, source/IR coordinates and storage/data/queue/HTTP/dev-tool families
remain separate capacity work.

The accepted [native phase ABI](spec/native-phases.md) promotes the complete Bytes
result into a separate output context before scratch disposal. Arbitrary-value
promotion, finer subpass lifetimes and heap-backed native calls are not established.

## Accepted milestone records

| Milestone | Accepted implementation | Exact tested tree | Acceptance |
| --- | --- | --- | --- |
| A — maintained raw-source self-compilation | `010ce014` | `7c4b670f` | [Seal](decisions/20261101-active-source-acceptance.json), [decision](decisions/20261101-active-source-self-compilation.md) |
| C — same-process native scratch reclamation | `a1e4c9dd` | `43041af8` | [Seal](decisions/20261101-native-phase-acceptance.json), [decision](decisions/20261101-native-phase-reclamation.md) |
| Closed pure callable expansion | `8526ed77` | `3de34681` | [Acceptance and receipt bindings](decisions/20261101-closed-pure-callables-profile8.md#acceptance-and-mainline-delivery) |

Each implementation retains its own exact acceptance scope. C covers repeated
invocations in one long-lived C1 process and one C2 process; C3 byte equality does
not claim a separate C3 phase process. Later documentation commits are distinct
from the tested implementation trees.
