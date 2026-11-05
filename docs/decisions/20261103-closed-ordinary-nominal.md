# Closed ordinary nominal source profile

Date: 2026-11-03

Status: candidate publicly adopted on 2026-11-05. C0 prerequisites, complete public
readback, the paired helper observations and all 18 focused source/index/forest
controls passed. Renewed raw-source compiler fixed points, the full frozen-tree
seal and mainline integration remain pending. The accepted Profile 8 / Profile 7
callable baseline retains its original tree and evidence.

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
recursive nominal values, arbitrary List<T>, and full P_self completion. Existing
closed list substitutions and direct ByteBuffer operations retain their current
contracts. Both supported source roots remain () -> I64 and (Bytes) -> the exact
structural {diagnostic:I64,image:Bytes} reply.

## One current identity at each boundary

| Boundary | Candidate identity | Change |
| --- | --- | --- |
| LKJCI1 Mode 0 | Profile 9 | Nominal descriptor kinds 10/11 and Variant/Match instructions 22/23; Pass remains 21 |
| LKJCI1 Mode 1 | Profile 8 | Ordinary declaration payloads 3/4, tagged Record/Field selectors, Variant and SwitchVariant projection |
| Raw source carrier | LKJSG002 profile 3 | Unchanged |
| Source request | LKJSRQ01 profile 2 | Unchanged |
| Canonical schema | Epoch 1 | Existing source, unit, map, pack and artifact meanings/encodings retained |
| Native invocation | LKJPH001 | Existing structural Bytes-output lifetime and recovery contract retained |

Changed private producers/readers switch together; predecessors reject without
a fallback reader. Derived private inputs and generated binaries must be
regenerated after that switch. Authoritative GraphRepository meaning is not
rewritten to preserve stale derived bytes. A canonical format change would need
its own unambiguous identity; none changes in this candidate.

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

A further 257-case fixture checks the complete canonical dispatch inventory and
executes selected cases including canonical ordinals 255 and 256. The wire already
has wider ordinals; this is a framing/order/end-to-end control, not proof of a
removed capacity ceiling. The C1 controls and genuine C2/C3 fixed-point control
invoke the same behavior, rejection and wide-dispatch corpus through both output
modes. Strict artifact loading, exact C0 artifact agreement and independent
VM/reference/C0-native/source-native results are required before acceptance.

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

## Observed candidate evidence — 2026-11-05

The candidate graph was adopted through the public transaction and completely
read back: 80,845 live owners across 135 modules, 96 supplied types and 59,447
retirements. The complete active profile contains 1,371 pure and 27 external
callables. All 17 changed ordered signatures and the removed parameter were
verified. This is the current branch graph, while mainline delivery remains the
accepted callable implementation and its documentation followup.

| Candidate identity | Exact binding |
| --- | --- |
| Public graph revision | `rev_afe20e53acfcc20c40d43f6f31884b338c650420d4ff5c25e6aab2b6e3ae18ed` |
| Graph HEAD SHA-256 | `bf959fbc82b4921d8b07ac5d99a668662aa4ce6a17f05c8d84508508a8e56441` |
| Semantic state | `semantic_state_cf986535dac28efb204015b8694beba19ef057f5430febd7a5ee3a3c7f4ee510` |
| Complete Profile-3 carrier | 26,107,644 bytes; SHA-256 `30c40c5a0cb6b5be32b176e513515bb3f8d8fe2f541c8b2b736e637eac806d84` |
| Original Cargo-selected libtest SHA-256 | `6dfaed9473eff32e8f99e868cfc69551bc68123abd6fab50c335d12199592742` |

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
compiler/native throughput. Straight-path overhead is a concrete follow-up after
current pipeline behavior, fixed-point and frozen-tree acceptance.

Evidence lives under `.artifacts/capacity-unbounding/nominal-20261105/`:

| Receipt | SHA-256 |
| --- | --- |
| `prerequisites-jobs16-1/receipt.json` | `76dc8a5272ba869a1c0572c50f759225f7159619e44585a91bb29e97ab7fe7b4` |
| `adoption-1/preparation.json` | `d581004402d7630472267026ce8e3c948da716e7f51aac6d7d448e38ed506edf` |
| `adoption-1/post-submit-verification-v3.json` | `822c94f8a9546c71c95e69c414d22907528fe1daf4fb07e14b3a63ee68d3dd33` |
| `paired-measurement-audit-1/receipt.json` | `c6891079db1d30418858df9102d30f93aa7ffd140b925e37b70407669e52b3b0` |
| `focused-1/receipt.json` | `58b9f021825209b994f5229c3450f01544b801a7a7efd253f42204a4c0b59d30` |
| `focused-1/active-profile.json` | `6958a55dfeb37de905c32864012d0837a5e24b6c45a1d43fa7d7a1a70082132f` |

The paired audit revalidated both strict receipts and 48 evidence files without
rerunning a product or compiler test. It binds the original Cargo selection,
complete implementation snapshot, all supplied source types through each carrier,
public adoption and readback, and unchanged non-graph source. The focused receipt
then binds 18 successful controls and the complete active profile to the unchanged
implementation and adopted graph. Renewed C2=C3 and Ahost=A1=A2=A3 equations,
full frozen-tree gates and release-target verification remain pending.

## Required acceptance

1. Compile and admit the C0 producer/native prerequisite before adopting the new
   constructs into maintained source. Record actual nonzero focused test counts.
2. Use public GraphRepository operations for compiler proposals, then complete
   public readback, replay and the refreshed owner/type/expression/intrinsic/use/
   call inventory. Do not supply inferred types or custody certificates to the
   raw-source stages.
3. Exercise independent programs through VM, canonical reference, C0 native and
   source-produced native/artifact paths. Cover exact nominal identities, every
   case, absence versus Unit, nested acyclic values, callable parameters/results/
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
   the unchanged ABI. Complete fresh exact staged-tree gates and release-target
   verification before integration; bind terminal receipts to that exact tree.

The scoped results above do not establish full candidate acceptance or capacity
liberation. Canonical u32 indices, source carrier/object/
child/depth limits, recursive value lifecycle and broader storage limits remain
separate work. Full P_self and lexical Region support remain required by the
authoritative [self-hosting contract](../spec/selfhosting.md).
