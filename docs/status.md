# Current status

Updated 2026-11-05 (Asia/Tokyo). Accepted typed meaning is the sole editable
program authority; maintained source proposals are adopted through public transactions.

## Current delivery boundary

Accepted and integrated on `origin/main`.
The closed ordinary nominal implementation is `6bf7559a`,
with corrected tree `25b21080`. The
[decision](decisions/20261103-closed-ordinary-nominal.md#acceptance-evidence-and-delivery)
binds retained compiler evidence to original tree `2a30272f` and fresh corrective
gates to `25b21080`. Milestones A and C and the closed
pure callable expansion retain their accepted scope. Full Tier 2 Milestone B
(P_self) remains open. No release archive or production deployment is published.

## Accepted capabilities

The source compiler covers the maintained scalar/compound closure, structural
records/fields, `List<I64>`, lexical control flow, direct monomorphic recursion, closed I64
list substitutions, exact intrinsic signatures and binary filters. ByteBuffer
consumption, synchronous borrows, branch-balanced disposal, reverse lexical
cleanup and returned-owner transfer remain checked; live custody prevents unsafe
tail transfer. Both ELF and canonical artifacts are built from complete raw source.

FunctionValue, prefix Bind and Invoke support exact copyable captures, callable
fields/results, ordinary indirect calls and custody-aware graph-tail frame reuse.
Empty Bind preserves identity without an environment. Text and StaticText retain
distinct semantic identities. The [callable decision](decisions/20261101-closed-pure-callables-profile8.md)
retains that expansion's separate implementation and evidence.

The ordinary nominal slice adds closed, nongeneric, acyclic, copyable
Record/Variant values and exhaustive Match through both outputs. Exact nominal
identity, absence versus Unit and authored initializer order are preserved.
All-arm ByteBuffer custody includes owned buffer results, nested Match and separate
synchronous reads before, inside and after Match. General borrowed helpers and
a loan spanning Match remain outside this profile.

Migrated opaque child ownership uses iterative cleanup. VM and reference have
independent iterative semantic comparison; raw structural observation remains a
separate fallible operation. The selected pure canonical-reference driver retains
graph activations and expression continuations on the heap, with exact custody,
call-depth/tail accounting, cancellation and modeled allocation obligations.
This does not make every reference route or generated native CALL stack unbounded.

## Compiler graph and delivery

| Identity | Current implementation |
| --- | --- |
| Implementation / corrected tree | `6bf7559a` / `25b21080` |
| Mainline delivery | Fast-forwarded into `origin/main` at `6bf7559a` |
| Public graph revision | `rev_afe20e53…` |
| Semantic state | `semantic_state_cf986535…` |
| Complete carrier | Profile 3; 26,107,644 bytes; exact binding in the nominal decision |

Public adoption and complete readback cover 80,845 live owners across 135 modules,
96 supplied types and 59,447 retirements. The complete active inventory has 1,371
pure and 27 external callables. All 17 changed ordered signatures and the removed
parameter were checked. Inventory does not replace semantic/custody admission.

The implementation uses LKJSG002 profile 3, LKJSRQ01 profile 2, Mode-0 profile 9,
Mode-1 profile 8 and canonical schema epoch 1. Changed private formats switch
together without predecessor fallback; incompatible derived inputs require regeneration.

## Verification

Prerequisite and focused runs retain their separate bindings. The literal
selfhost, named controls and raw fixed point below ran on original frozen tree
`2a30272f`; corrected bootstrap, public CLI, full and release checks belong to
`25b21080` under the decision’s guarded evidence composition.

| Validation | Result |
| --- | --- |
| C0 native/producer prerequisites | 17 passed; 0 failed or ignored |
| Focused source/index/forest selections | 18 passed; 0 failed or ignored |
| Literal selfhost script | 296 passed, 0 failed, 11 ignored across three selections |
| Maintained native closure / complete inventory / 8193-prefix | Each 1 passed; 0 failed or ignored |
| Whole maintained raw-source fixed point | 1 passed; 0 failed or ignored; actual C2=C3 and Ahost=A1=A2=A3 |
| Corrected-tree bootstrap / focused public CLI control | Passed; exact public control 1 passed, 0 failed or ignored |
| Complete fresh full check, all 29 gates | 29 passed, all fresh; 0 reused or unrun |
| Release target | Passed; original bootstrap-bound verifier, exit 0 |
| Corrective acceptance and terminal audit | Passed; corrected tree accepted with final relevance guards |

The C2/C3 controls check 42 nominal behavior inputs and 20 negative mutations.
The wide fixture constructs, admits and lowers all 257 cases and Match arms into
ELF and artifacts; each artifact’s complete canonical dispatch inventory is checked.
Recorded runtime checks select seven cases, including canonical ordinals 255 and 256.
Strict loading, exact host artifact agreement, compiler-source mutation sensitivity,
same-process C1/C2 recovery and final unchanged-repository checks passed in the
retained original-tree raw fixed-point control. The decision distinguishes each result and its binary evidence:
the literal script does not separately record its selected test-image hash.

The 8193-prefix crossing retains its explicit 32 GiB process allowance.
Its largest unit remains below the separate 4 MiB object and 8 MiB unit-codec
ceilings; it does not establish those capacity removals.

## Remaining work

Selected next action: remove wide Sequence count/framing gates together with the
scalar and artifact cursors’ repeated whole-parent copies. Public-transaction
admission and owner/unit envelope crossing remain unverified; no new capacity
closure is claimed.

The same-binary 64-block helper pair reduced If/Jump VM instructions from 2,171,227
to 447,208, while the straight control rose from 146,587 to 272,159 (+85.7%) and
from 41,541,957 to 57,316,566 modeled cumulative bytes (+38.0%). The decision
retains both cases and single-run timings. This whole-batch helper comparison
does not isolate indexing or establish whole-compiler speed; straight-path overhead
and admission-stage measurement remain follow-up work. Forest early completion
improves proof work but establishes no new capacity removal.

Tier 2 Milestone B remains open for OwnedRecord/OwnedChoice and affine
deconstruction, general `List<T>`, owned witnesses and source-tied read views,
lexical Regions, generic/recursive nominal values, general generic specialization,
task/effectful callables and resource/owned captures. Consecutive boxed-spine
Clone admission remains depth 256;
structural type/AST/identity, owned-storage escape and codec paths retain separate
bounds. Default call/value-stack policies, broader reference routes and native
CALL growth are unchanged. Owned-record layout preparation retains its 64 MiB
cumulative metadata default and depth-256 guard. Canonical unit/object envelopes,
source/IR coordinates and storage/data/queue/HTTP/dev-tool families remain separate work.

The accepted [native phase ABI](spec/native-phases.md) promotes the complete Bytes
result into a separate output context before scratch disposal. Arbitrary-value
promotion, finer subpass lifetimes and heap-backed native calls are not established.

## Accepted milestone records

| Milestone | Accepted implementation | Verification tree(s) | Acceptance |
| --- | --- | --- | --- |
| A — maintained raw-source self-compilation | `010ce014` | `7c4b670f` | [Seal](decisions/20261101-active-source-acceptance.json), [decision](decisions/20261101-active-source-self-compilation.md) |
| C — same-process native scratch reclamation | `a1e4c9dd` | `43041af8` | [Seal](decisions/20261101-native-phase-acceptance.json), [decision](decisions/20261101-native-phase-reclamation.md) |
| Closed pure callable expansion | `8526ed77` | `3de34681` | [Acceptance](decisions/20261101-closed-pure-callables-profile8.md#acceptance-and-mainline-delivery) |
| Closed ordinary nominal expansion | `6bf7559a` | `25b21080`; retained compiler controls `2a30272f` | [Acceptance](decisions/20261103-closed-ordinary-nominal.md#acceptance-evidence-and-delivery) |

Each implementation retains its exact acceptance scope. Same-process reclamation
covers C1 and C2 processes; C3 byte equality does not establish a separate C3 phase
process. Later documentation commits are distinct from tested implementation trees.
