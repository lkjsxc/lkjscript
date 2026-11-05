# Closed ordinary nominal source profile

Date: 2026-11-03

Status: Accepted and integrated on `origin/main`.
Implementation and composed verification bindings are recorded below. The accepted
Profile 8 / Profile 7 callable baseline retains its original tree and evidence.

## Decision and scope

Extend the source compiler and both private backend projections with closed,
nongeneric, acyclic, copyable ordinary Record/Variant values and exhaustive
Match. The complete path includes C0 native prerequisites, source framing and
lexical scope, exact nominal typing, all-arm ByteBuffer custody, native lowering
and execution, canonical declaration/instruction emission, exact summaries and
strict artifact provenance. A decoder or projected-input success alone is not
the source-driven capability.

The slice excludes OwnedRecord/OwnedChoice and affine owned deconstruction,
implementation witnesses and associated read views, lexical Regions, generic or
recursive nominal values, arbitrary `List<T>`, and full P_self completion. Existing
closed list substitutions and direct ByteBuffer operations retain their current
contracts. Both supported source roots remain () -> I64 and (Bytes) -> the exact
structural {diagnostic:I64,image:Bytes} reply.

## One current identity at each boundary

| Boundary | Identity | Change |
| --- | --- | --- |
| LKJCI001 Mode 0 | Profile 9 | Nominal descriptor kinds 10/11 and Variant/Match instructions 22/23; Pass remains 21 |
| LKJCI001 Mode 1 | Profile 8 | Ordinary declaration payloads 3/4, tagged Record/Field selectors, Variant and SwitchVariant projection |
| Raw source carrier | LKJSG002 profile 3 | Unchanged |
| Source request | LKJSRQ01 profile 2 | Unchanged |
| Canonical schema | Epoch 1 | Existing source, unit, map, pack and artifact meanings/encodings retained |
| Native invocation | LKJPH001 | Existing structural Bytes-output lifetime and recovery contract retained |

Changed private producers/readers switch together; predecessors reject without
a fallback reader. Derived private inputs and generated binaries must be
regenerated after that switch. Authoritative GraphRepository meaning is not
rewritten to preserve stale derived bytes. A canonical format change would need
its own unambiguous identity; none changes in this slice.

## Semantic and lowering commitments

Nominal identity includes package and declaration. Complete member vectors are
nonempty and canonical, and all child types are already resolved and copyable.
Equal physical layouts do not equate distinct declarations. Record layout uses
canonical selector order. Public authoring preserves initializer evaluation order
with ordered lexical temporaries when that order differs from layout order;
traps must retain that authored sequence through lowering and execution. Variant
absence differs from present Unit at source, projection and native representation
boundaries.

Every Match arm belongs to the scrutinee's exact Variant definition and the
complete canonical CaseReference vector is required. Payload binders are fresh,
nonparameter, arm-local and explicitly annotated with the exact payload type;
present Unit still has a binder. Arm result types agree, including ByteBuffer
when its ordinary custody rules are satisfied. All arms begin with the same
post-scrutinee state. Compute the final custody intersection over all arms, then
derive reverse-lexical disposal for every actual arm independently; a pairwise
cleanup against one representative earlier arm is insufficient.

Whole-inventory admission includes unused declarations and supplied types,
unselected cases, uncalled functions, untaken branches and unselected targets.
Malformed framing, exact endpoints, containment, namespaces, contextual type
scope, nominal parameter-slot flow and type/custody errors retain their distinct
diagnostics. Legal nominal recursion outside this profile rejects with 13 after
E1; it is not relabeled a language-invalid structural cycle. Missing or wrong
payload annotations are 19. Unsupported generic definitions retain precedence
over nongeneric argument-arity errors.

Mode 0 projects exact nominal descriptors and dense case ordinals only after
source admission. Native shape checks retain declaration identity and explicit
payload presence, and dispatch initializes only the selected payload local.
Copyable nominal values use the existing Call/Bind/Invoke/tail machinery; a live
ByteBuffer still forces the retained call/cleanup continuation. Native ordinary
CALL and physical frame limits are not removed.

Mode 1 remains a representation projection of canonical pre-tail units. It
preserves member-reference/type encounter order and original instruction PCs;
case table indices need not be numerically sorted. Match payload annotations
remain source Type dependencies and do not alone create extra unit type entries.
Copyable binders do not introduce synthetic Unit/StoreLocal cleanup. Declaration
units, Field/Case owners, complete summaries and required source Type closure
must agree with C0 and the strict loader, including unused nominal definitions.

The Match plans retain an absolute arm cursor and a checked label index
instead of rescanning earlier arms and trace fragments. Custody repair reuses
validated completed evidence and appends per-arm repairs; it does not retain a
growing immutable arm-history or recursively replay nested Match analysis.
The scoped paired observations below cover the trace-packing helper. They do not
measure Match analysis or custody-repair savings.

## Independent control contract

The raw nominal fixture defines 42 behavior inputs: eleven nominal/closure cases,
fifteen existing branch-custody cases, fifteen affine-result/nested-Match/
synchronous-buffer-read cases, and one outer absent-case path. These are behavior
inputs, not 42 separately selected tests. The same inventory is checked for the
original seed 42 and a source mutation to 46. It covers a Match returning one
owned ByteBuffer, lexical nesting of Match, and separate synchronous length reads
before, inside and after Match. It does not claim a loan spanning Match or admit
general borrowed helper functions.

Twenty complete-carrier negative mutations preserve framing and rebind changed
owners/types before checking their intended semantic layer. They cover exact
nominal/member/binder identity, uncalled and untaken errors, affine result reuse,
continuation use after a branch moves an owner, and unsupported supplied types.
A host-valid borrowed-helper declaration remains an explicit unsupported-profile
control with diagnostic 13; it is not reported as a custody failure. Earlier
framing/scope/type-layer checks and independent host admission remain separate.

A further 257-case fixture constructs every Case owner, constructor and Match arm.
C1 controls and actual C2/C3 generations admit and lower the complete fixture into
ELF and canonical artifacts. Each artifact strictly loads, agrees with C0 and has
all 257 dispatch identities, payload bindings, targets and joins checked. Runtime
selects authored cases 0, 1 and 2 plus cases at canonical ordinals 0, 128, 255 and
256: seven distinct cases in the recorded C2/C3 run. It does not execute all 257.
Selected cases run through the artifact-backed VM, independent reference, C0 native
and each source-produced ELF. The VM uses the first strictly loaded artifact (C2
in the paired run); both generations’ artifacts are separately loaded and compared.
The wire already has wider ordinals; this is a framing/order/end-to-end control,
not proof of a removed capacity ceiling. The behavior and rejection corpora retain
their independent controls through both output modes.

## Forest proof work

Each ancestry-doubling pass reserves one work unit per owner before visiting the
complete row table. It computes doubled parents and the conjunction that all
resulting parents are the root sentinel. A complete successful pass ends the
proof immediately, without later doubling or a redundant final-root scan.
If any row remains unrooted, distance doubling continues until it covers the owner
count, followed by the separately charged root scan that rejects cycles. A rooted
prefix cannot hide another cyclic component. u32 owner counts bound the largest
doubled distance to 2^32, within the checked I64 representation.

Independent parent-distance fixtures specify exact charges: a 33-owner star needs
33 visits rather than the old seven full passes (231); a nine-owner chain needs
four passes (36); a five-owner forest containing a disconnected two-cycle needs
20 visits to establish diagnostic 15. With allowance 19 the latter instead returns
resource refusal 6. These are source-derived control expectations, not measured
speed, allocation or RSS results. Empty/singleton, zero/one-short/exact/full-u64,
malformed framing and cancellation/retry controls preserve the separate policy,
validity and cleanup contracts. None remains unbounded; every Some(u64) remains
finite, including u64::MAX. Passive observations do not become execution quotas.

## Acceptance evidence and delivery

The compiler graph was adopted through the public transaction and completely
read back: 80,845 live owners across 135 modules, 96 supplied types and 59,447
retirements. The complete active profile contains 1,371 pure and 27 external
callables. All 17 changed ordered signatures and the removed parameter were
verified. Mainline delivery: Fast-forwarded into `origin/main` at `6bf7559a`.
The implementation commit identifies the corrected tree. Retained original
compiler computations and fresh corrective gates remain separately bound; later
documentation changes have a separate tree.

| Identity | Exact binding |
| --- | --- |
| Implementation commit / corrected tree | `6bf7559a2e0d486072d83128fe508079e97ca6d8` / `25b21080093d88ff2d04ed09d8fa665ff0c04274` |
| Original compiler-evidence tree | `2a30272f152c40a6e3bf471e0bb71d05b33de7c7` |
| Corrected public integration fixture | `tests/public_cli/native_elf.rs`; SHA-256 `6801b9bf81b4e925d940d7bd1e458bf1c85141b75b9854179cf79ad51e98d263` |
| Mainline integration | Fast-forward to `6bf7559a2e0d486072d83128fe508079e97ca6d8` |
| Public graph revision | `rev_afe20e53acfcc20c40d43f6f31884b338c650420d4ff5c25e6aab2b6e3ae18ed` |
| Graph HEAD SHA-256 | `bf959fbc82b4921d8b07ac5d99a668662aa4ce6a17f05c8d84508508a8e56441` |
| Semantic state | `semantic_state_cf986535dac28efb204015b8694beba19ef057f5430febd7a5ee3a3c7f4ee510` |
| Complete Profile-3 carrier | 26,107,644 bytes; SHA-256 `30c40c5a0cb6b5be32b176e513515bb3f8d8fe2f541c8b2b736e637eac806d84` |
| Prerequisite/paired/focused Cargo-selected libtest SHA-256 | `6dfaed9473eff32e8f99e868cfc69551bc68123abd6fab50c335d12199592742` |
| Retained original raw-source C2 = C3 | 21,967,484 bytes; SHA-256 `53de2e2ed6ca38b1471ccff3ec1c35e65e4fe0536cbeffd50880bfef82de492b` |
| Retained original Ahost = A1 = A2 = A3 | 22,639,950 bytes; SHA-256 `3e14b950adb49c96131de7ed6c4a441a0f871dc7511cafdfabb0f0db874e7190` |

The original libtest passed 17 C0 native/producer controls, with no failures or
ignored controls. The subsequent focused run passed all 18 selections, with no
failures or ignored tests, in 342.14 seconds as reported by libtest. Its three
raw-nominal controls exercised the 42-input behavior corpus, 20 semantic mutations
and complete 257-case dispatch inventory with selected executions. The other
selections covered nominal flow, canonical trace indexing, proposal replay, the
complete active profile and exact forest proof controls. Both paired helper
stages used that unchanged executable
and their respective complete public graphs. Their exact input trace hashes,
independent expected packed bytes and output hashes agree case by case. Both
strict measurement validators passed; all retained live-custody counters are zero
and both primary modeled counters are unsaturated.

### Scoped corrective composition

The original seal remains failed: its full check recorded 28 fresh gate passes
and a failed workspace-test gate. Reproduction with the original selected public
integration executable confirmed that the handcrafted input still requested
Profile 8 and was rejected with exit 65 and `compiler_rejected:2\n`. The single
corrected file changes that independent valid literal to Profile 9, extracts its
existing checksum refresh, adds a checksum-consistent Profile 8 rejection, then
restores and reseals Profile 9 before the separate invalid-magic rejection.
Public change plan/apply, handcrafted bytes without an exporter, and detached
output `42` remain the control contract. The tree delta is exactly +21/-6 lines
in that integration test; compiler, library, maintained source and graph inputs
are unchanged.

The original-prefix audit revalidated the first seven genuinely completed
operations and 609 evidence files. It reports `candidate_accepted=false`; neither
that audit nor the failed original full receipt accepts either candidate. Original
bootstrap and its verifier bridge remain provenance. The five retained semantic
operations—literal selfhost, maintained native closure, complete owner/type
inventory, 8193-prefix and raw-source fixed point—keep their original tree identity
and actual computations. They are not reported as rerun on the corrected tree.

Reuse requires all other 2,615 indexed paths and raw file/mode inventories to
match, with unchanged complete graph projection, every retained selfhost input,
supplied-type carrier, original library and CLI images, and verifier bytes.
Supporting Cargo dependency evidence excludes the changed integration test.
The corrective driver checks these bindings at entry and completion, including
absence of unstaged, untracked or hidden-index changes. Any relevant input,
implementation, image, policy or evidence mismatch invalidates the corresponding
reuse; the affected computation must be renewed. This is a checked projection of
relevant inputs, not equality of the two complete source trees.

Acceptance additionally requires fresh corrected-tree bootstrap and runtime digest
bridge, a fresh Cargo-selected public CLI test build and exact repaired control,
a complete fresh full check of all 29 gates, release-target verification, and an
independent terminal audit. Old full gate results are not reused. The completed corrective receipt and independent terminal audit below bind
the accepted corrected tree.

### Retained original-tree compiler results

The completed selections on original frozen tree `2a30272f` report:

| Selection | Result |
| --- | --- |
| Literal selfhost: selfhost_ / source_input_ / native_elf_ | 249 / 12 / 35 passed; 0 failed; 11 / 0 / 0 ignored |
| Maintained native closure / complete owner-type inventory / 8193-prefix | Each 1 passed; 0 failed or ignored |
| Whole maintained raw-source fixed point | 1 passed; 0 failed or ignored; 2170.01 seconds reported by libtest |

### Fresh corrected-tree gates

These operations are required on corrected tree `25b21080`; their terminal
outcomes are not supplied by the original-prefix audit.

| Validation | Result |
| --- | --- |
| Corrected bootstrap and focused public CLI control | Passed; exact public control 1 passed, 0 failed or ignored |
| Complete fresh full check, all 29 gates | 29 passed, all fresh; 0 reused or unrun |
| Original bootstrap-bound verifier release target | Passed; original bootstrap-bound verifier, exit 0 |
| Corrective acceptance and independent terminal audit | Passed; corrected tree accepted with final relevance guards |

The retained original-tree raw-source control actually executes C2=C1(S,E) and
C3=C2(S,E) on the complete
carrier, and obtains the canonical artifact from each of C1, C2 and C3. All three
artifacts strictly load and agree exactly with the host oracle. Actual C2 and C3
controls also check the independent 42-input nominal behavior corpus and 20 negative
mutations. Both generations compile the entire 257-case fixture into both outputs
and check every canonical artifact dispatch entry; runtime verification selects
the seven cases described above, including canonical ordinals 255 and 256. The
behavior corpus retains its seed mutation; compiler-source mutation separately changes the
emitted compiler and its diagnostic. Same-process recovery uses C1 and C2; no
separate C3 reclamation process is claimed. Both the inner-equation marker and
the final original-repository-unchanged marker passed.

Bootstrap and named controls retain their own selected-image proofs. The literal
selfhost script retains its lease, selection log and before/after input hashes,
but no separate selected-libtest image digest. The prerequisite hash above or a
later full-checker image proof must not be retroactively attributed to that run.

The measured operation is the normalized-VM invocation of
`source-unit-code-pack::project` on an internal no-Match trace. Each graph stage
has one 64-block invocation per case, after two small warmups. Both outputs contain
386 packed instructions and 18,560 bytes.

| Case | VM instructions, before → after | Modeled cumulative allocated bytes, before → after | Observed helper wall time, before → after |
| --- | ---: | ---: | ---: |
| If/Jump labels | 2,171,227 → 447,208 (−79.4%) | 210,621,765 → 73,954,022 (−64.9%) | 273.969 → 53.405 ms |
| Straight control | 146,587 → 272,159 (+85.7%) | 41,541,957 → 57,316,566 (+38.0%) | 18.207 → 32.427 ms |

The branch-heavy improvement and straight-path regression are both retained.
Instruction counts are deterministic observations for these exact graphs and
inputs. Allocated bytes are modeled cumulative accounting, not actual allocator
traffic, copied bytes, peak live memory, RSS or native slab usage. Timings are
single observations around the helper invocation; graph preparation/export, input
construction, VM setup and output checking/disposal are outside that timer.
The whole nominal/index/forest batch changed between graph stages. This pair does
not isolate indexing as the cause, establish asymptotic scaling, or measure full
compiler/native throughput. Straight-path overhead remains a concrete follow-up.

Evidence lives under `.artifacts/capacity-unbounding/nominal-20261105/`:

| Receipt | SHA-256 |
| --- | --- |
| `prerequisites-jobs16-1/receipt.json` | `76dc8a5272ba869a1c0572c50f759225f7159619e44585a91bb29e97ab7fe7b4` |
| `adoption-1/preparation.json` | `d581004402d7630472267026ce8e3c948da716e7f51aac6d7d448e38ed506edf` |
| `adoption-1/post-submit-verification-v3.json` | `822c94f8a9546c71c95e69c414d22907528fe1daf4fb07e14b3a63ee68d3dd33` |
| `paired-measurement-audit-1/receipt.json` | `c6891079db1d30418858df9102d30f93aa7ffd140b925e37b70407669e52b3b0` |
| `focused-1/receipt.json` | `58b9f021825209b994f5229c3450f01544b801a7a7efd253f42204a4c0b59d30` |
| `focused-1/active-profile.json` | `6958a55dfeb37de905c32864012d0837a5e24b6c45a1d43fa7d7a1a70082132f` |
| `seal-round1/raw-source-fixed-point.stdout` | `4a2f9ea65bb3a6540646f3d304c206a7c319c2a7e077396c6f1f393fbd774fa9` |
| `seal-round1/raw-source-fixed-point.stderr` | `1dfa423b079936c925d8caa6610151c9edf839e256ab1d1b19f9dfa5905e3bf3` |
| `seal-round1/receipt.json` — failed historical seal | `b010806bd588e6b1b636279652670e48ba03d7261001745c625a5e41b2e4a6d4` |
| `public-cli-profile-repair-1/original-reproduction.json` | `aecd107a6d0aa568c41cd9f6140b38dbd8d5638f66e40dd65dace6a26453cb2e` |
| `corrective-public-profile9/prefix-audit-original-2/receipt.json` — prefix audit only | `c4d1a3c08cf5620c16449c5e0caf6c701f846f0836be5acb4a61b393c0bdcf90` |
| `corrective-public-profile9/run-round1/receipt.json` | `8e9061f17c89cae7aeaaaf9ba9647a1297292e152787431f3a345d147fa72823` |
| `corrective-public-profile9/audit-terminal1/receipt.json` | `a2f121ced2c721dfb31c3fed00ad56901920364a5957f0fc39f9df9d3b73f2d5` |

The paired audit revalidated both strict receipts and 48 evidence files without
rerunning a product or compiler test. It binds the original Cargo selection,
complete implementation snapshot, all supplied source types through each carrier,
public adoption and readback, and unchanged non-graph source. The focused receipt
then binds 18 successful controls and the complete active profile to the unchanged
implementation and adopted graph. The raw-source logs bind the completed generation
equations, independent behavior and rejection controls, native phases and final
unchanged-repository check on the original tree. Corrected-tree full/release
acceptance, final relevance guards and terminal audit:
Corrected bootstrap/bridge and the repaired public control passed; full completed all 29 gates freshly and release target passed. Entry/exit relevance checks and the independent terminal audit accepted the corrected tree. Retained compiler computations keep their original tree identity; the original seal remains failed.

## Acceptance obligations

1. Compile and admit the C0 producer/native prerequisite before adopting the new
   constructs into maintained source. Record actual nonzero focused test counts.
2. Use public GraphRepository operations for compiler proposals, then complete
   public readback, replay and the refreshed owner/type/expression/intrinsic/use/
   call inventory. Do not supply inferred types or custody certificates to the
   raw-source stages.
3. Exercise independent programs through VM, canonical reference, C0 native and
   source-produced native/artifact paths. Cover exact nominal identities, complete
   canonical case inventories, the specified runtime cases, absence versus Unit,
   nested acyclic values, callable parameters/results/
   captures, canonical layout with preserved authored initializer order, and both
   root signatures.
4. Use hash-consistent layer-reaching negatives for missing/extra/wrong fields or
   arms, wrong annotations, sibling binder escape, same-shape foreign nominal
   identity, unused unsupported definitions/types/cases, and hidden branch errors.
   Include three-or-more-arm custody joins with earlier live/later moved owners,
   reordered arms, reverse cleanup of multiple locals, affine results, nested
   Match, synchronous buffer reads and live-owner tail fallback. Private-wire controls and explicit
   finite-policy refusal remain separate from raw-source semantic controls.
5. Strictly load every emitted canonical artifact, verify provenance and exact
   C0 unit/artifact agreement, then repeat the program behavior/mutations through
   actual C2/C3 raw-source generations. Refresh C2=C3 and Ahost=A1=A2=A3 for the
   complete maintained source closure and declared deterministic environment.
6. Exercise native phase failure/cancellation recovery and output survival through
   the unchanged ABI. Complete fresh corrected-tree bootstrap, the repaired public
   control, all 29 full gates and release-target verification before integration.
   Retain original compiler computations only under the relevance guards above;
   bind corrective and terminal-audit receipts to the exact corrected tree.

This acceptance is limited to the closed, nongeneric, acyclic, copyable ordinary
nominal slice and its stated ByteBuffer control-flow contract. It does not close
Tier 2 Milestone B or remove an additional capacity ceiling. OwnedRecord/OwnedChoice,
affine deconstruction, general `List<T>`, witnesses and source-tied read views,
lexical Regions, generic/recursive nominal values, general generic specialization,
task/effectful callables and resource/owned captures remain open. Direct monomorphic
recursion and the recognized list intrinsics’ closed I64 substitutions remain admitted.
Canonical u32/source/IR coordinates, object/unit envelopes, boxed-spine and other
structural/AST/identity/escape/codec depth bounds, default VM/reference policies,
native CALL growth, and owned-layout 64 MiB/depth-256 defaults retain their separate
contracts. Storage/data/queue/HTTP/dev-tool capacities and arbitrary-value native
promotion remain separate work under the authoritative
[self-hosting contract](../spec/selfhosting.md). Release-target verification does
not publish a release archive or deploy a product.
