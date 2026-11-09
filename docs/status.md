# Current status

Updated 2026-11-09. Linear source-directory merge delivered; native raw-I64 candidate in verification.
Accepted typed meaning is the sole editable program authority; maintained source
proposals are adopted through public transactions.

## Current delivery and verification

Linear raw-source directory disjointness: corrected implementation tree
`01c34060`, based on `9a804993`; retained compiler controls use `933f87a0`.
Implementation commit: `0dbb9233`.
Delivery: integrated into `main` and pushed; independently verified on `origin/main`
at 00:31 UTC.
The native-allocation measurement is checkpointed as `2f2c22e3`. Its focused
measurement and callable regression passed. The derived
[native raw-I64 candidate](decisions/20261109-native-i64-slots.md), based on
`7f7cdc77`, is on `compiler/20261109-native-i64-slots`. It includes direct graph
arguments/results, boxed callable/aggregate adapters and typed tail-owner scans.
Fresh candidate acceptance and integration remain pending.
The [canonical record](decisions/20261108-source-directory-merge.md#acceptance-evidence-and-delivery)
binds both source trees, the private-test correction, public graph adoption and
measurements. Each result retains its actual execution tree. Later documentation
verification remains separate from implementation acceptance.

| Validation | Recorded outcome |
| --- | --- |
| Focused directory, framing, inventory and adoption controls | 7 passed on retained tree; corrected affected test separately passed |
| Identical-carrier read-nominal control | Passed on retained tree; VM, reference and Rust-native |
| Maintained raw-source fixed point and independent audit | Passed on retained tree; C2=C3 and Ahost=A1=A2=A3 |
| Same-process C1/C2 raw controls | Passed on retained tree; 17 frames each, final owned mappings zero |
| Corrected staged bootstrap and typed verifier bridge | Passed |
| Corrected changed check | 29 passed fresh; 0 reused/unrun; root 3,086 passed, 0 failed, 21 ignored |
| Corrected fresh full check | 29 passed fresh; 0 reused/unrun; root 3,086 passed, 0 failed, 21 ignored |
| Release target and final acceptance audit | Passed; final source, image and terminal audit accepted |

On the same retained 26,611,855-byte carrier, one measured read-nominal pair
(including graph admission) changed VM time from 247.055 s to 231.185 s,
reference time from 1,398.984 s to 1,321.052 s, and Rust-native filter time from
10.849 s to 10.192 s. VM instructions decreased 7.05%; reference expressions
decreased 5.57%. These are valid-carrier phase measurements, without separate
read-graph timing, whole-test/compiler or statistical claims. Cumulative modeled
allocation is separate from RSS.

The current public adoption has 81,458 live owners, 97 supplied types and 61,470
retirements; its complete carrier is 26,640,313 bytes. The decision records its
exact revision, semantic state, core and carrier identities. The paired
measurement uses the earlier retained carrier, not these different source bytes.
LKJSG002 profile 3, LKJSRQ01 profile 2, Mode-0 profile 9, Mode-1 profile 8 and
canonical schema epoch 1 remain unchanged.

The merge advances ordered live/retired cursors directly over complete keys,
replacing a binary search and temporary key slice per live owner. It preserves
framing, semantic rejection and exact explicit allowances. This is an algorithm
improvement, not a new capacity milestone or semantic-profile expansion.

The [projection reuse](decisions/20261108-link-package-projection-reuse.md#acceptance-evidence-and-delivery),
[Wide Sequence](decisions/20261107-wide-sequence-capacity.md#acceptance-evidence-and-delivery)
and [ordinary nominal](decisions/20261103-closed-ordinary-nominal.md#acceptance-evidence-and-delivery)
records retain their earlier accepted trees and evidence. Milestones A and C
retain their scope. Full Tier 2 Milestone B remains open; no release archive or
production deployment is published.

## Language and capacity scope

The source compiler covers the maintained scalar/compound closure, structural
records/fields, `List<I64>`, lexical control flow, direct monomorphic recursion,
closed I64 list substitutions, exact intrinsic signatures and binary filters.
FunctionValue, prefix Bind and Invoke support exact copyable captures, callable
fields/results, ordinary indirect calls and custody-aware graph-tail frame reuse.
Empty Bind preserves identity. Text and StaticText remain semantically distinct.

Closed, nongeneric, acyclic, copyable Record/Variant values and exhaustive Match
retain exact nominal identity, absence versus Unit and authored initializer order.
ByteBuffer results, nested Match, branch-balanced disposal, reverse lexical
cleanup and synchronous reads before/inside/after Match remain checked. General
borrowed helpers and a loan spanning Match remain outside this profile.

Wide Sequence removes the selected child/owner/unit/instruction/reference limits
and four repeated parent-copy paths. Temporary-discard lookups are rekeyed to
eliminate the quadratic parent-bucket scan.
Package-source collection/publication and normalized preparation use the reviewed
optional policies. Exact finite allowances, complete semantic/custody proof,
checked representations, physical allocation and atomic VM/shared attachment
remain independent obligations.

The Wide Sequence record retains the complete million-child disk witness and
its independent execution/recovery results. It does not independently prove
the removed 1M-runtime-owner threshold, individual 4 MiB ArtifactManifest,
10K artifact packages, 1M artifact segments or 16 GiB bundle threshold. The
>256 MiB codec/collector witness has separate component scope. Public disk
GraphRepository authoring is exercised; an executed CLI file-input crossing
is not claimed.

## Remaining work

The [native allocation baseline](decisions/20261109-native-scalar-allocation.md)
passed on normalized input (Tier 1): 12 processes, 36 invocations and 132 observed
frames, with independent VM/reference outputs and final owned mappings zero.
At 8,192 scalar iterations, Rust makes 24,585 allocation requests and the
maintained backend makes 6; a 17-byte slice adds one request per iteration in both.
The ordinary identity call adds no allocator requests in this fixture.

The current candidate derives local/operand representations after optimizer
expansion, keeps exact I64 graph signatures raw across direct calls, and adapts
boxed storage/callable boundaries. Whole-body boxed fallback retains the same
direct ABI. Focused boundary, backedge, cleanup, trap and cancellation controls
precede renewed allocation measurements, combined raw fixed-point/phase proof and
fresh changed/full/release acceptance. No whole-compiler allocation share or deep
non-tail stack bound is established.

Tier 2 Milestone B remains open for OwnedRecord/OwnedChoice and affine
deconstruction, general `List<T>`, owned witnesses and source-tied read views,
lexical Regions, generic/recursive nominal values, general generic specialization,
task/effectful callables and resource/owned captures.

Opaque-child cleanup and selected semantic comparison/reference continuation
routes are iterative. General structural/boxed-value lifecycle work remains:
boxed-spine Clone admission is depth 256, and type/AST/identity, owned-storage
escape and codec paths keep separate bounds. Default call/value-stack policies,
broader reference routes and generated non-tail native CALL growth are unchanged.
Owned-record layout preparation retains its 64 MiB metadata default and depth-256
guard. Separate package/readiness/edge limits, the CompilationManifest aggregate,
bounded exact builtin retention, untouched object domains, source/IR coordinates
and storage/data/queue/HTTP/dev-tool families remain open.

The accepted [native phase ABI](spec/native-phases.md) promotes the complete Bytes
result into a separate output context before scratch disposal. Arbitrary-value
promotion, finer subpass lifetimes and heap-backed native calls are not established.

## Milestone records

| Milestone | Implementation | Verification tree(s) | Canonical record |
| --- | --- | --- | --- |
| A — maintained raw-source self-compilation | `010ce014` | `7c4b670f` | [Seal](decisions/20261101-active-source-acceptance.json), [decision](decisions/20261101-active-source-self-compilation.md) |
| C — same-process native scratch reclamation | `a1e4c9dd` | `43041af8` | [Seal](decisions/20261101-native-phase-acceptance.json), [decision](decisions/20261101-native-phase-reclamation.md) |
| Closed pure callable expansion | `8526ed77` | `3de34681` | [Acceptance](decisions/20261101-closed-pure-callables-profile8.md#acceptance-and-mainline-delivery) |
| Closed ordinary nominal expansion | `6bf7559a` | `25b21080`; retained compiler controls `2a30272f` | [Acceptance](decisions/20261103-closed-ordinary-nominal.md#acceptance-evidence-and-delivery) |
| Wide Sequence capacity | `411886df` | `75ae1da2`; retained compiler controls `45fe92e4` | [accepted](decisions/20261107-wide-sequence-capacity.md#acceptance-evidence-and-delivery) |
| Artifact linking projection reuse | `47e98941` | `0a4ef322` | [Acceptance](decisions/20261108-link-package-projection-reuse.md#acceptance-evidence-and-delivery) |
| Linear source-directory merge | `0dbb9233` | `01c34060`; retained compiler controls `933f87a0` | [Record](decisions/20261108-source-directory-merge.md#acceptance-evidence-and-delivery) |

Each implementation retains its exact scope. Same-process reclamation covers C1
and C2 processes; C3 byte equality does not establish a separate C3 phase process.
Later documentation commits remain distinct from tested implementation trees.
