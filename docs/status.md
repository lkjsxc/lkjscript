# Current status

Updated 2026-11-09. Native raw-I64 graph slots: accepted and integrated into main.
Accepted typed meaning is the sole editable program authority; maintained source
proposals are adopted through public transactions.

## Current capability and delivery

The [native raw-I64 implementation](decisions/20261109-native-i64-slots.md#acceptance-evidence-and-delivery)
derives scalar local/operand representations after optimizer expansion. Exact I64
Code parameters and results use a raw direct ABI, including direct tail calls.
External, callable and compound boundaries retain their checked boxed adapters.
A declined representation plan keeps boxed internal slots with the same direct
ABI. Raw and empty locals are excluded from owner scans; uncertain boxed slots
remain checked. Entry adaptation occurs once per activation and is bypassed by
bytecode Jump(0).

Tested implementation tree: `9d71c3e6`, based on `7f7cdc77`.
Implementation commit: `acf74027`.
Delivery: Pushed to main; advertised and fetched `origin/main` independently matched `acf74027` at 2026-11-09T11:07:09.686Z.
The canonical decision binds the complete frozen source, actual executable
images, completed controls and allocation comparisons. Later documentation
verification remains separate from implementation acceptance.

## Completed verification

| Validation | Recorded outcome |
| --- | --- |
| Focused native representation, boundary, cleanup and cancellation controls | 23 passed; separate ignored allocation probe passed |
| Combined maintained raw-source compiler control and independent audit | Passed; genuine generations, C2=C3, strict Ahost=A1=A2=A3 and independent behavior/mutation/negative controls |
| Same-process C1/C2 raw controls | Passed; 17 frames per process, output survival and final owned mappings zero |
| Staged bootstrap and typed verifier bridge | Passed on the frozen implementation tree |
| Changed check | 29 passed fresh; 0 reused/unrun; root 3,093 passed, 0 failed, 22 ignored |
| Fresh full check | 29 passed fresh; 0 reused/unrun; root 3,093 passed, 0 failed, 22 ignored |
| Release target and aggregate acceptance | Passed; complete source/index seal, bound execution images and actual joined terminals accepted |

## Allocation observations

At 4,096 and 8,192 iterations, the unchanged normalized-input scalar probe records
6 Rust allocation requests per invocation for both the scalar and ordinary
identity-call paths, down from 12,297 and 24,585. Bytes slicing records 4,102 and
8,198, down from 16,393 and 32,777. The maintained backend is unchanged and matches
these counts. Each measured run covers 12 processes, 36 invocations and 132
observed frames, with independent VM/reference outputs, zero saturation flags
and final owned mappings zero.

The complete maintained raw-source compiler comparison records C1-Rust artifact
requests decreasing from 872,780,680 to 97,298,484 and ELF requests from
1,123,766,130 to 66,443,017. The artifact retry matches the first artifact
invocation. C2-source requests remain 63,427,596 for artifact and 47,655,928 for
ELF. These are passive per-invocation request counts including scratch and output
promotion; they do not establish allocated bytes, RSS, peak memory, allocation
category shares or wall-clock speedup.

The maintained graph is unchanged: 81,458 live owners, 97 supplied types and
61,470 retirements; the complete carrier is 26,640,313 bytes. LKJSG002 profile 3,
LKJSRQ01 profile 2, Mode-0 profile 9, Mode-1 profile 8 and canonical schema epoch 1
remain unchanged. This is a native representation improvement. Full Tier 2
Milestone B remains open. Publication state: no release archive or production deployment published.

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
and four repeated parent-copy paths. Its canonical record retains the complete
million-child disk witness and independent execution/recovery results. That
witness does not independently prove the removed 1M-runtime-owner threshold,
individual 4 MiB ArtifactManifest, 10K artifact packages, 1M artifact segments or
16 GiB bundle threshold. The >256 MiB codec/collector witness has separate scope.
Public disk GraphRepository authoring is exercised; an executed CLI file-input
crossing is not claimed.

Package-source collection/publication and normalized preparation use reviewed
optional policies. Exact finite allowances, complete semantic/custody proof,
checked representations, physical allocation and atomic VM/shared attachment
remain independent obligations.

## Remaining work

Next, measure immutable probe preparation in the canonical-unit control and
align focused builds with the required workspace Cargo producer. The next
semantic slice is closed, copyable `List<T>` admission and lowering across
raw source, ELF and canonical artifacts.

Tier 2 Milestone B remains open for OwnedRecord/OwnedChoice and affine
deconstruction, general `List<T>`, owned witnesses and source-tied read views,
lexical Regions, generic/recursive nominal values, general generic specialization,
task/effectful callables and resource/owned captures.

Opaque-child cleanup and selected semantic comparison/reference continuation
routes are iterative. General structural/boxed-value construction, copying,
comparison, serialization, destruction and failure cleanup remain open.
Boxed-spine Clone admission is depth 256; type/AST/identity, owned-storage escape
and codec paths keep separate bounds. Owned-record layout preparation retains
its 64 MiB metadata default and depth-256 guard. Default call/value-stack policies,
broader reference routes and generated non-tail native CALL growth are unchanged.

Separate package/readiness/edge limits, the CompilationManifest aggregate,
bounded exact builtin retention, untouched object domains, source/IR coordinates
and storage/data/queue/HTTP/dev-tool families remain open. The accepted
[native phase ABI](spec/native-phases.md) promotes the complete Bytes result before
scratch disposal. Arbitrary-value promotion, finer subpass lifetimes and
heap-backed native calls are not established.

## Milestone records

| Milestone | Implementation | Verification tree(s) | Canonical record |
| --- | --- | --- | --- |
| A — maintained raw-source self-compilation | `010ce014` | `7c4b670f` | [Seal](decisions/20261101-active-source-acceptance.json), [decision](decisions/20261101-active-source-self-compilation.md) |
| C — same-process native scratch reclamation | `a1e4c9dd` | `43041af8` | [Seal](decisions/20261101-native-phase-acceptance.json), [decision](decisions/20261101-native-phase-reclamation.md) |
| Closed pure callable expansion | `8526ed77` | `3de34681` | [Acceptance](decisions/20261101-closed-pure-callables-profile8.md#acceptance-and-mainline-delivery) |
| Closed ordinary nominal expansion | `6bf7559a` | `25b21080`; retained compiler controls `2a30272f` | [Acceptance](decisions/20261103-closed-ordinary-nominal.md#acceptance-evidence-and-delivery) |
| Wide Sequence capacity | `411886df` | `75ae1da2`; retained compiler controls `45fe92e4` | [Acceptance](decisions/20261107-wide-sequence-capacity.md#acceptance-evidence-and-delivery) |
| Artifact linking projection reuse | `47e98941` | `0a4ef322` | [Acceptance](decisions/20261108-link-package-projection-reuse.md#acceptance-evidence-and-delivery) |
| Linear source-directory merge | `0dbb9233` | `01c34060`; retained compiler controls `933f87a0` | [Acceptance and measurements](decisions/20261108-source-directory-merge.md#acceptance-evidence-and-delivery) |
| Native raw-I64 graph slots | `acf74027` | `9d71c3e6` | [Record](decisions/20261109-native-i64-slots.md#acceptance-evidence-and-delivery) |

Each implementation retains its exact scope. Same-process reclamation covers C1
and C2 processes; C3 byte equality does not establish a separate C3 phase process.
Prior source-directory timing, adoption and correction details remain in that
canonical record.
