# Wide Sequence capacity and source cursors

Status: candidate, not accepted. Both compiler-source adoptions, the second
measurement pair, 29 host controls, four custody controls and the C1/C2
100,001-child pilots have completed evidence. Build 9 and 72 package-source
controls passed, including the actual >256 MiB component crossing. The candidate
also removes ordinary normalized-preparation work and metadata defaults.
Historical preparation controls and the in-memory million-item component passed.
The current source-bound native phase controls and complete million-item disk
witness also passed. Maintained artifact equations and the final seal remain pending. The
[ordinary nominal decision](20261103-closed-ordinary-nominal.md#acceptance-evidence-and-delivery)
retains the accepted delivery boundary.

## Capability and representations

Remove the coupled default ceilings that obstruct a wide, flat, nonempty
Sequence, together with repeated whole-parent copies and the separate quadratic
temporary-discard lookup. Complete public graph admission remains mandatory.

| Coupled boundary | Candidate contract |
| --- | --- |
| Sequence 100,000 children | No default child quota; preserve nonempty ordered typed references, exact endpoints, unique containment, lexical context, types and custody. |
| Owner 4 MiB and source envelope 8 MiB | Progressive owner decoding, input-framed source spans and unbounded ordinary storage/read policies. Other callers of the finite envelope wrapper retain their exact allowances. |
| CompilerUnit 8 MiB codec / 4 MiB storage and 1,000,000-item quotas | Progressive nested decoding and fallible growth; retain checked instruction/table/local counts, ordinals and protocol invariants. |
| Artifact manifest 4 MiB, 10,000 packages, 1,000,000 owners/segments and 16 GiB bundle | Progressive inventories, checked bundle framing, exact reachable-owner correspondence and unbounded ordinary artifact I/O. |
| Shared packed 128 MiB admission | Audited linear entries for Owner, CompilerUnit and ArtifactManifest; unrelated recursive packed codecs retain their separate bounds. |
| Package-source 256 MiB / 1,000,000 objects, generic map entries and link read sets | No default byte/object/map-entry quota across producer, decoder, collector, publication metadata and public export/stage I/O; explicit optional allowances remain exact. |
| Normalized preparation 10,000,000 derivation steps / 256 MiB modeled metadata | Independent optional u64 allowances default to None across signature checking, concrete closure and code sharing; semantic termination, physical allocation and exact runtime attachment remain separate obligations. |

For the same canonical value, field order, tags, checksums, digest domains and
encoded bytes remain unchanged. Canonical schema epoch 1, LKJSG002 profile 3,
LKJSRQ01 profile 2, Mode 0 profile 9 and Mode 1 profile 8 remain current.
Newly adopted meaning and implementation-bound artifacts retain their own
measured identities. Complete raw export includes unused supplied types and all
required owners/retirements; it does not prune, normalize or supply semantic
evidence that the source compiler must derive.

Variable owner/unit/manifest collections decode progressively, including nested
names, effects, implementation operands, read views, signatures and instructions.
An element must exist before reserving its vector slot; byte/string extents are
proved before copying. Buffer growth is fallible. The progressive packed decoder
reports allocation failure as Resource; malformed payloads, trailing bytes and
noncanonical encodings remain Corrupt. Checked arithmetic and representation
conversions retain their path-specific validation. Semantic checks remain
mandatory. This is not a general recursive Clone/Drop redesign.

Carrier counts, offsets and spans and dense compiler counts/ordinals retain
checked u32 representations. Source byte APIs retain nonnegative I64 indexing.
Reference-owner counts still require exact correspondence with distinct reachable
owners, using checked arithmetic and rejecting missing or unreachable records.
The 4 MiB artifact pack and 16 KiB fixed-row map leaf targets remain physical
layout choices; additional segments and valid oversized singleton objects carry
aggregate growth.

## Source traversal and custody

Shared absolute carrier cursors replace per-child copies of a complete Sequence
parent in scalar completion, canonical artifact completion, custody traversal and
artifact Local-context lookup. They validate the row/kind, operation, span,
payload end and ordinal before selecting the exact typed expression reference.
A 17-byte reference count must fit the remaining payload before multiplication.
Complete envelope and graph checks remain prior obligations. Initial entry and
other passes may still copy an owner; no whole-compiler linearity claim follows.

Authored order, each nonfinal discard, the final result's use/custody mode, outer
lexical context, first-trap identity and live-owner cleanup are preserved.
Nested final-child context and malformed cursor states remain independent controls.

The first measurement pair exposed a separate source-level cost: all temporary
discard events shared the Sequence parent bucket, so repeated child lookups
could inspect N(N-1)/2 records. The corrective public proposal changes only
source-custody-walk::after-sequence and source-custody-lower::selected.
Proof tag 7 is keyed by the completed child, retaining its Sequence parent as
source; instruction tag 6 (Drop) queries that same pair. Unique containment
remains required. The detail word, full custody snapshot, other lookup keys and
generic first/latest behavior are unchanged; the final child emits no temporary
discard event.

Both source adoptions used public change plan/apply and complete readback.
Ordered signatures and named declarations/parameters remained exact; body edits
can create and retire expression owners. Four custody controls cover nonfinal
ByteBuffer disposal and final transfer. Genuine evidence is compared through
VM, reference and C0-native probes; framed old-key, wrong-parent and missing-event
lookup mutations are checked through VM/reference. Those internal mutations do
not constitute native corrupt-proof execution.

## Package-source correction

The first million-item disk attempt reached public preparation but failed with
Resource/package_source_budget at the generic map-entry limit, before fixture
publication. Its failed receipt is `capacity-1000001-c2-1/receipt.json` in the evidence root
below. The composed-callable admission path
reconstructs candidate owners through the shared package-source reader even
without package dependencies. Removing only that map guard would have left
the collector's distinct-object and retained-byte ceilings on the same path.

The correction therefore updates container encoding/decoding/inventory checks,
CollectingStore, generic map reconstruction, link read-set metadata, public
package export/stage I/O and discovery together. Complete callable/source proof,
private and unused owners, exact keys and original-source readback remain.

PackageContainer::decode defaults to unbounded decoder allowances.
decode_with_limits accepts independent Option<u64> byte/package/object
allowances: None is unbounded; Some(0), Some(n) and Some(u64::MAX) remain finite,
without sentinel conversion or clamping. The independent package-count profile
is still checked. Existing semantic/read permission ledgers and their joint
failed-read reservation accounting are unchanged; saturated observations never
grant or remove permission.

The decoder checks the complete minimum header/selection/object framing before
inventory growth, then uses remaining-slice extents, strict unique ordering and
canonical domain/digest checks. Encoder sizes and wire conversions are checked.
Flat vectors and byte copies reserve fallibly; map inventories grow only from
entries actually returned by checked traversal. The executed component control
encoded and decoded a 268,435,810-byte container with 268,435,457 payload bytes
across five objects and retained those objects through the collector. This is a
wire/collector witness, separate from the public million-owner semantic proof.

## Normalized preparation correction

The second million-item attempt published its real disk fixture and C2 emitted
both ELF and canonical artifact outputs. Independent C0 preparation then failed
with Resource/normalized_instantiation_storage at the former 256 MiB modeled
metadata default. Its failed receipt remains capacity-1000001-c2-2/receipt.json.
Those completed earlier stages do not establish host artifact equality, fixture
VM/reference/native execution, recovery frames or a successful capacity terminal.

NormalizedPreparationLimits carries independent Option<u64> derivation-work and
modeled-metadata allowances. Ordinary prepare_with_control selects None for both;
prepare_with_limits preserves Some(0), Some(n) and Some(u64::MAX) as exact finite
allowances. One pair of ledgers spans function/signature validation, concrete
type/layout/witness closure and final code sharing. It is not reset between
subpasses, and no maximum-valued sentinel stands for unbounded permission.
A refused charge preserves the prior ledger. Advisory local-liveness work may
decline extra storage and leave valid code less optimized.

Strict artifact admission remains a prerequisite. Exact type applications are
visited once; constructor-bearing parameter-flow cycles in a callable strongly
connected component still reject, including composed and foreign selections.
Recursive forwarding, permutation and closed-type reset remain admissible when
the exact application closure is finite. Effect specialization retains fixed
original templates and deduplicated closed arguments over a finite requirement
universe. These semantic arguments establish termination independently of the
removed cumulative work ceiling. Repaired canonical-and-compiled strict-loader
negatives continue to test expanding cycles and unused function values.

The type_derivation_steps and type_metadata_bytes observations saturate with
separate type_derivation_saturated and type_metadata_saturated flags. They model
cumulative proof work and storage; they are neither RSS nor remaining permission.
VM invocation preserves these observations and attaches preparation metadata plus
its affine-variant inventory to local/shared allocation accounting. It stages
local counters and shared finite reservations before committing either side.
Saturated metadata cannot establish any finite byte allowance, including
Some(u64::MAX); an unbounded byte dimension does not bypass a finite collection
allowance. Cancellation and resource refusal remain distinct from invalid meaning.

Instruction, continuation and tail-call graph vectors reserve physical capacity
fallibly with checked extents. A selected modeled allowance does not guarantee a
host allocation, and this change makes no general allocator, Clone/Drop or deep
value-lifecycle claim. The immutable builtin has a separate bounded exact
retention admission. Saturated observations cannot prove that finite retained
size; it has no uncached fallback and is not claimed unbounded by this slice.

The original build-13 image passed 82 selected preparation controls, including
exact finite policies, cancellation, shared attachment and independently repaired
semantic rejection. Its wrapper failed while parsing one multiline libtest result;
the separate retained-evidence audit reparsed all 82 successful product results
without rerunning or relabeling the failed wrapper.

The same historical image completed a public in-memory 1,000,001-item Sequence:
strict host loading, 644,003,269 modeled metadata bytes, 19,000,062 derivation
steps, VM/reference/C0 result 42 and cleanup. Both preparation saturation flags
were false. This is an executed component witness; it does not provide public
CLI file authoring, a real-disk fixture, selfhost generation/emission or 17-frame
recovery. Its exact source and image remain build 13. The reviewed later changes
affect only phase-harness policy and its compiler fixture; they do not enter the
82 selected controls or this component's call path.

## Native phase caller policy

The selected capacity harness uses an explicit None address-space allowance.
It requires an unlimited inherited hard limit, launches with unlimited soft/hard
limits, and restores the unlimited soft limit after deliberate finite mmap
refusal. Actual child limits are read at startup and restoration. Ordinary
controls retain Some(32 GiB); Some(u64::MAX) is a checked RLIMIT representation
refusal rather than an infinity sentinel. This changes test caller policy, not
compiler meaning, phase ABI, fault thresholds or runtime custody.

The current build-15 image passed all three focused controls in 63.61 seconds:
four Echo processes cover both backends under finite and unbounded policies,
and one independently checked compiler fixture covers all 17 recovery frames.
Together they recorded five processes, 109 frames and 14 actual policy
observations. The compiler fixture adds 4,096 discarded literal prefixes before
its unchanged Bytes echo result so the existing 8 MiB in-work cancellation
trigger is observed. Cancellation returned status 1; retry reproduced the initial
artifact, retained outputs survived, and final owned/anonymous mappings were zero.
The earlier small-fixture run remains failed in focused-native-policy-1/receipt.json;
its completion before the cancellation trigger is not a passing recovery result.
The complete million-item disk process is recorded separately below.

## Evidence and its limits

Evidence paths below are under
`.artifacts/capacity-unbounding/wide-sequence-20261107/`.
Each result is limited to the provenance its receipt actually records. The four
custody controls and capacity pilots retain complete source and original-image
bindings. The historical 29-host receipt records an executable path without an
independent per-run image digest or complete source proof; its historical
attribution comes from the retained checkpoint. No historical result is relabeled
as a current implementation run.

| Evidence | Completed scope |
| --- | --- |
| `adoption-2/receipt.json` | Both adopted cursor/discard changes have complete public readback. Current graph: 81,456 owners, 97 supplied types, 61,339 retirements; carrier 26,611,855 bytes. |
| `measurement-pair-2.json` | Six literal/Local fixtures at 512/2048/4096 children; twelve matching ELF/artifact outputs using the same original build-7 image. |
| Historical host receipt retained by `execution-checkpoint-3.json` | 29 outcomes passed. The receipt is executable-path-only; checkpoint attribution does not supply an independent per-run binary digest or complete source proof. Fresh full verification supplies current coverage. |
| Four custody controls retained by the pilot audits | Four selections passed with their recorded complete source/graph and original-image bindings. |
| `capacity-100001-literal-audit-1.json` | C1 public-API fixture, both outputs, strict loading, independent host/VM/reference/native behavior, cleanup and 17 native phase frames. |
| `capacity-100001-c2-audit-1/audit.json` | Fresh genuine C2/C3 preparation, terminal original-source recheck, identical C1 fixture/requests/outputs, the same independent controls and 17 phase frames. |
| `source-container-capacity-correction-2/manifest.json` and `package-source-root-patch-1/manifest.json` | Exact reviewed five-file correction; execution evidence is recorded separately below. |
| `host-build-9/receipt.json` | Original Cargo-selected CLI/library build passed with identical complete before/after inputs. |
| `focused-package-source-1/receipt.json` | 72 selections passed in 92.299 seconds, including exact optional allowances, malformed framing, joint-read accounting, source/closure rejection, public file-reader policy and the >256 MiB codec/collector crossing. |
| `capacity-1000001-c2-2/receipt.json` | Failed at independent C0 metadata preparation after public disk publication and both C2 outputs; no terminal equality, execution or recovery pass. |
| `host-build-13/receipt.json` and `focused-preparation-runner-3/retained-regular-2-audit.json` | Original build and 82 product controls passed. The original regular wrapper remains failed; corrected retained parsing separately verifies all 82 results. |
| `focused-preparation-million-audit-1/receipt.json` | Historical build-13 public in-memory component crossed the seven quantities below; strict artifact, VM/reference/C0 result 42 and cleanup passed. |
| `native-phase-policy-independent-review-1.json` and `native-phase-compiler-fixture-correction-1/manifest.json` | Exact later test-harness transitions preserve historical preparation attribution; they are not fresh preparation executions. |
| `host-build-15/receipt.json` and `focused-native-policy-independent-audit-2/receipt.json` | Current source build and three focused phase controls passed; five processes, 109 frames and 14 actual kernel-policy observations. |
| capacity-1000001-c2-3/receipt.json and capacity-1000001-c2-audit-3/audit.json | Actual build-15 public disk witness passed both outputs, exact Ahost agreement, strict loading, VM/reference/C0/C2 execution, seven measured crossings and 17 recovery frames. |

All six second-pair ELF observations were lower; artifact observations were equal
or higher. For example, the 4096-literal case changed from 11,549 to 1,139 ms
for ELF and from 1,441 to 1,553 ms for artifacts. These are single ordered
whole-compilation observations, not statistical speed estimates or isolated
cursor attribution. The quadratic lookup diagnosis is separate source evidence.

The two 100,001-child pilots crossed only the former 100,000-child ceiling:

| Measured quantity | Pilot value |
| --- | ---: |
| Complete Sequence owner | 1,700,092 bytes |
| Complete CompilerUnit | 1,100,387 bytes |
| Instructions | 200,002 |
| Reference owners | 100,003 |
| Complete artifact bundle | 39,399,475 bytes |

The owner/unit/instruction/reference quantities remained below their former
limits. Bundle size does not prove an individual object crossing. Fixture
authoring used public GraphRepository create, decode_compact_change,
prepare_authored_change and publish; it is public-API evidence, not execution of
a CLI file-input crossing. The compiler-source adoptions separately used the CLI.

The later build-13 public in-memory component crossed seven coupled quantities:

| Measured quantity | Former default | Actual component value |
| --- | ---: | ---: |
| Sequence children | 100,000 | 1,000,001 |
| Complete Sequence owner bytes | 4,194,304 | 17,000,092 |
| Complete CompilerUnit bytes | 8,388,608 | 11,000,387 |
| Instructions | 1,000,000 | 2,000,002 |
| Reference owners | 1,000,000 | 1,000,003 |
| Modeled preparation metadata bytes | 268,435,456 | 644,003,269 |
| Modeled preparation derivation steps | 10,000,000 | 19,000,062 |

Its complete artifact was 267,505,213 bytes. The seven quantities measured here
and in the disk witness below do not independently demonstrate the removed 1,000,000-runtime-owner threshold, the
individual 4 MiB ArtifactManifest threshold, 10,000 artifact packages,
1,000,000 artifact segments or the 16 GiB bundle allowance. The preparation
observation reported three runtime owners; reference-owner population is a
different quantity. Total bundle size does not prove any individual manifest
size. The separate 72-control >256 MiB codec/collector witness has wire/collection
scope; it is not this semantic graph's proof.

Host phase wall times for this one component run were 120,968 ms for build,
196,814 ms for link, 19,386 ms for strict loading and 1,596 ms for preparation.
They are observations of separate phases, not modeled work or memory measures.
The component has no selfhost generations, compiler emissions or same-process
17-frame recovery. The completed fresh disk witness below supplies those separate
obligations with its own source and executable identities.

The explicit C2 harness path creates a fresh complete public S, builds C1=C0(S,E),
actually executes C2=C1(S,E) and C3=C2(S,E), checks byte equality and rechecks the
original repository before using C2. It substitutes no cached generation. Default
C1 selection remains lazy. Pilot C2/C3 images matched at 22,005,001 bytes, but
this preparation does not establish the separate maintained artifact equation.

C2 was selected for the larger workload after the same-fixture pilot recorded
41,762,852 first-artifact allocation requests, versus C1's 463,666,427.
C0-generated boxed arithmetic and source-emitted machine-word arithmetic differ;
no exact allocation-callsite attribution is claimed. The C2 phase process's
observed VmHWM was 4,982,988 KiB. VmPeak is lifetime virtual high-water memory,
VmHWM lifetime RSS high-water memory and VmRSS current sampled RSS; observations
are not final-peak guarantees. They remain separate from slab/request counters
and modeled evaluator allocations. No million-item memory extrapolation follows.

The 17 phase frames cover artifact/ELF output survival, semantic rejection,
cancellation, actual mmap refusal, retry and final output release. They do not
establish the separate munmap-refusal control or a C3 phase process.

## Renewed million-item disk witness

The fresh build-15 control completed public GraphRepository authoring on real
disk storage for 1,000,001 literal children and 1,000,007 live owners. The request
proposal was 26,000,385 bytes. Both the control and its terminal audit passed
with unchanged source, graph, index and original executables; the independently
replayed audit matched the retained result apart from its observation timestamp.

The emitted ELF was 31,065,800 bytes and the canonical artifact 267,505,246 bytes.
Strict loading passed, and the artifact matched independent Ahost bytes exactly.
VM, reference, C0-native and the emitted C2-native program returned 42 with the
required cleanup. The current witness independently measured the same seven
crossings tabulated above: 1,000,001 children, 17,000,092 owner bytes, 11,000,387
unit bytes, 2,000,002 instructions, 1,000,003 reference owners, 644,003,269 modeled
preparation bytes and 19,000,062 derivation steps; both preparation saturation
flags were false. The five additional manifest/inventory thresholds qualified
above remain unproven; total bundle size is not an individual-manifest measurement.

Before compiling that fixture, the control genuinely executed C2=C1(S,E) and
C3=C2(S,E) on complete maintained source, compared their identical 22,005,001-byte
images and rechecked the original repository. That compiler-source preparation
is distinct from the million-item fixture input. It does not establish the
maintained Ahost=A1=A2=A3 equation or the complete fixed-point negative/mutation
suite, which remain obligations of the upcoming seal.

The C2 phase process, PID 3849808, passed all 17 frames under explicit None.
Startup and post-refusal restoration observed unlimited soft and hard limits.
Artifact/ELF output survival, semantic rejection, cancellation, mmap refusal,
retry and final release passed; final owned and anonymous mappings were zero.
The verified native phase terminal reports 1,177,729 ms elapsed. This is the
single phase-process observation, not the total suite time or a memory metric.

| Completed disk-witness stage | Observed elapsed ms |
| --- | ---: |
| Public disk authoring | 888,068 |
| C2 ELF compilation | 336,781 |
| C2 artifact compilation | 630,263 |
| Independent host build | 306,877 |
| Independent host link | 460,574 |
| Independent strict loader | 15,779 |
| Independent normalized preparation | 1,326 |

These are stage wall times from one actual run, not statistical speed estimates.
The result is public API and disk-storage evidence; it does not claim an executed
CLI file-input crossing. The source-bound records are
capacity-1000001-c2-3/receipt.json,
capacity-1000001-c2-audit-3/audit.json and
capacity-1000001-c2-independent-review-3/receipt.json. Full candidate acceptance
is still pending.

## Remaining acceptance and bounds

Preserve the completed preparation component and 82 controls under their original
build-13 bindings. Current build-15 phase controls and the complete disk witness
have separate actual source/image receipts. Their success does not establish the
maintained compiler artifact equation, independent mutation/negative controls or
the frozen-tree acceptance seal.

The completed 1,000,001-child C2 real-disk workload supplies both outputs,
host equality, strict loading, independent execution, all seven measured
crossings and 17 phase frames under its actual source/image binding. This closes
the selected capacity witness, not the remaining maintained-compiler acceptance
obligations or the five unproven inventory/manifest thresholds.

The complete source candidate still requires genuine C2=C3 and
Ahost=A1=A2=A3, independent mutation/negative oracles, original-repository checks,
bootstrap, literal selfhost, named controls, all 29 fresh full gates, release
target and terminal acceptance audit. Bind the actual frozen source/tree/graph
and original executables, regenerate/verify guides, and integrate only the
accepted tree. Earlier failed or historical attempts retain their own identities.
Later status/decision acceptance prose belongs to a separate documentation-only
commit and targeted verification receipt; it does not inherit the full sealed
tree identity.

Separate package/readiness 10,000-count and 100,000 dependency-edge limits remain,
as do the aggregate 10,000,000 compiler-unit bound, fixed CompilationManifest
envelope, separate bounded exact builtin retention, unrelated per-form/object/
packed/storage/data/service limits and checked source/IR representations. The candidate does not remove general structural-depth
or boxed-value lifecycle guards, default VM/reference call/value limits, broader
reference recursion or generated non-tail native CALL stack growth.

Tier 2 Milestone B remains open for owned nominal deconstruction, general List<T>,
witnesses/source-tied read views, Regions and the other semantic families recorded
in [status](../status.md). Native interning, finer arena rollback, primitive
fusion, debugger attribution and future custody-worker concurrency remain
separate proposals; none is evidence for this acceptance.
