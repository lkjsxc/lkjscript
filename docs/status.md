
# Current status

Updated 2026-11-07; implementation status: accepted.
Accepted typed meaning is the sole editable program authority; maintained source
proposals are adopted through public transactions.

## Current delivery and verification

Wide Sequence implementation: `411886df`, corrected tree
`75ae1da2`. Delivery: integrated into `main` and pushed; the advertised remote matched the implementation commit. The
[canonical acceptance record](decisions/20261107-wide-sequence-capacity.md#acceptance-evidence-and-delivery)
separately binds retained compiler computations on `45fe92e4` and fresh
corrective gates on `75ae1da2`. Documentation has its own later tree and
fresh changed-check receipt; it does not inherit the implementation seal.

| Validation and source binding | Recorded outcome |
| --- | --- |
| Original literal selfhost, `45fe92e4` | 302 passed, 0 failed, 14 ignored across three selections |
| Original maintained native closure / complete inventory / 8193-prefix | Each 1 passed; 0 failed or ignored |
| Original maintained raw-source fixed point | 1 passed; actual C2=C3 and Ahost=A1=A2=A3, independent controls and final repository recheck |
| Corrected-tree bootstrap and verifier bridge | Passed: fresh bootstrap invocation and exact typed verifier bridge; Cargo reuse recorded |
| Corrected selected library and controlled exact capacity test | Library build and exact inventory passed; one controlled exact test passed (1/0/0) with the explicit 600 s watchdog and two owned contenders. |
| Corrected fresh complete full check | 29/29 fresh passed; 0 reused, 0 unrun; root library 3,082 passed / 0 failed / 20 ignored; all 16 workspace streams retained |
| Corrected release target | Passed: `x86_64-unknown-linux-musl` static-musl target policy |
| Corrective acceptance and independent terminal audit | Passed: corrected candidate accepted; exact private-test projection and source, image, receipt, stream and actual-session bindings verified |

The canonical decision binds retained original-tree computations and fresh
corrected-tree results under exact-function relevance guards, with unchanged
surrounding source and all other file/mode inputs.

No implementation `check changed` pass is claimed. Corrective acceptance requires
all 29 fresh full gates. This two-file acceptance prose requires a separate fresh
changed5 check and documentation commit after the exact implementation commit.

The [ordinary nominal record](decisions/20261103-closed-ordinary-nominal.md#acceptance-evidence-and-delivery)
retains the earlier delivery and its distinct original/corrected-tree verification.
Milestones A and C and the callable/ordinary-nominal expansions keep their accepted
scope. Full Tier 2 Milestone B (P_self) remains open. No release archive or
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

The complete disk witness exercised 1,000,001 children, a 17,000,092-byte owner,
an 11,000,387-byte unit, 2,000,002 instructions, 1,000,003 reference owners,
644,003,269 modeled preparation bytes and 19,000,062 derivation steps, without
preparation saturation. Both outputs, strict loading, exact Ahost agreement,
VM/reference/C0/C2 result 42 and 17 recovery frames passed. The separate historical
build-13 preparation controls and component retain their original attribution.

Those seven quantities do not independently prove the removed 1M-runtime-owner
threshold, individual 4 MiB ArtifactManifest, 10K artifact packages, 1M artifact
segments or 16 GiB bundle threshold. The >256 MiB codec/collector witness has
separate component scope. Public disk GraphRepository authoring is exercised;
an executed CLI file-input crossing is not claimed.

Both public compiler-source adoptions cover 81,456 live owners, 97 supplied types
and 61,339 retirements; the complete carrier is 26,611,855 bytes. LKJSG002 profile 3,
LKJSRQ01 profile 2, Mode-0 profile 9, Mode-1 profile 8 and canonical schema epoch 1
are unchanged. Exact identities and historical evidence remain in the decision.

## Remaining work

A possible next measurement is scoped reuse of an already validated link
projection within one operation. The completed disk witness observed a 460,574 ms
host link stage; this does not attribute the cost, establish an implementation or
demonstrate an improvement. The second Sequence pair had lower ELF observations
and equal/higher artifact observations. The earlier nominal helper pair likewise
included a straight-path regression. Their canonical decisions retain both sides;
neither comparison establishes a uniform or whole-compiler speedup.

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

Each implementation retains its exact scope. Same-process reclamation covers C1
and C2 processes; C3 byte equality does not establish a separate C3 phase process.
Later documentation commits remain distinct from tested implementation trees.
