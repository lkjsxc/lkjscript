# Maintained active-profile raw-source self-compilation

Date: 2026-11-01 (Asia/Tokyo).
Decision: implement the complete maintained compiler's source feature closure,
remove the aggregate admission dependencies that blocked its public adoption, and
verify native self-compilation and canonical artifacts before mainline integration.
This document records completed focused evidence before the final staged-tree seal;
it does not declare full `P_self`, native phase reclamation or release publication.

## Meaning authority and input boundary

`tools/selfhost/project` is the editable meaning authority. All compiler changes,
including the three bulk-byte intrinsic declarations and their subsequent helper
adoption, were admitted through public `tx` operations and read back through public
owner/signature inspection. The initial complete-source transaction created 28,088
owners and updated 85 existing owners. Readback checked all 59,279 then-current
owners over 15 pages, all 65 changed parameter orders and all 12 deletions.
Later public transactions advanced the graph to its current complete inventory.
Proposal text is not an alternative accepted program.

The production exporter reads a complete current public package container and
borrows its original owner, type, retirement and revision-core bytes. It neither
performs source semantic admission nor supplies normalized instructions, inferred
types, custody certificates, owner subsets or cached generated code. Public export
and the one Rust bootstrap remain explicit trust boundaries. Complete source
requests are materialized before target oracles run.

The current source carrier is `LKJSG002` profile 3. It binds the original canonical
RevisionCore and claimed RevisionId as well as logical source directories. The
caller supplies TargetId and output mode through `LKJSRQ01` profile 2. Invalid core
claims pass through the raw framer to exercise source rejection; Rust does not
repair them. Predecessor profiles reject without fallback. This private framing
change does not alter the canonical meaning schema epoch. The complete carrier
hash is required because semantic-state alone does not bind unused supplied types.

## Current authoritative closure

| Binding | Value |
| --- | --- |
| Base Git commit | `71405a389a422a8fa51850ef70ba9f34040aca56` |
| Base Git tree | `d1036ebca89238a914d221c7ba3d0d951b4f0f21` |
| Graph revision | `rev_cd5e4e228db3c5abadec2e3e28bb2fbda602c3812177a6ab666fcf2240eb992a` |
| Public HEAD SHA-256 | `39d171a01e15cdf6fc9999c61dfcd0511cb7dc7a9600f274dcb3fa6b2d883082` |
| Semantic state | `semantic_state_01600719c76251652588c725f184f5b392b63d3a69f4ae36f52d18ba486fc606` |
| Source carrier SHA-256 | `404a5a97895b4156f67ef3c14c1f612007e4c238bed4054e88128677047e04bd` |
| Revision core SHA-256 | `776b75dd3e2c21f884a266675106def01e70e4d931a00c9ecce6482d490dda01` |
| ELF request SHA-256 | `af77da8b989cdc21df27eac3965edfbf60d71dc305a59c2f1e88e9262a111f0f` |
| Artifact request SHA-256 | `35c4b049bf7f8ecdabd0a2fe4a8177ff8ace5ca5e57519639f43f716a83c357b` |

The carrier is 17,598,834 bytes; each request is 17,598,866 bytes. The independently
refreshed profile contains 118 modules, 59,883 owners, 88 types, 35,651 retirements,
1,069 pure functions and 27 externals. There are 50,246 expressions, 4,514 bindings,
20 generic application sites and 76 structural record descriptors. The four targets
and all unused/untaken source participate in admission. Parameter-use counts are
3,595 unrestricted, 300 consume and one borrow. There are no source dependencies
or blobs in this active closure. Full proposal replay agrees on 1,103 canonical
units and the projected native backend closure.

The base Git tree above is not the tested working tree. Focused receipts bind their
complete modified/new/deleted file inventory and actual Cargo-selected executable.
The final acceptance commit must separately identify the exact finalized staged
Git tree and its new terminal receipts.

## E1, E2, custody and E3

The source implementation admits complete framing and object bindings, endpoints,
containment and lexical scopes, namespaces, contextual type binders, structural
DAGs and non-expansive nominal parameter flow before active typing/lowering.
Its supported active closure includes Text/StaticText, Bytes, ByteBuffer, List<I64>,
structural records, pure function descriptors and the closed list substitutions.
All intrinsic signatures and parameter-use modes are checked, including uncalled
externals. Structural field/order admission is separate from expression evaluation
order. Unit/Bool/I64, Text, Record, Field, List, Sequence, Let, If, Local and Call
have source typing and lowering paths.

Custody is derived within lkjscript. Consume moves, synchronous borrowed buffer
reads, branch-state joining and disposal, reverse-lexical cleanup and returned-owner
transfer remain explicit. A live owned local forces ordinary Call plus cleanup and
Return rather than TailCall. Unsupported arbitrary borrowed helpers and affine
aggregate/witness/Region families do not silently pass the active profile.

The ELF path builds the current profile-6 normalized input itself and invokes the
maintained emitter. The artifact path derives complete canonical units, ordered
reference/type tables, summaries, public interfaces, logical/physical maps,
pack partitions and profile-7 artifact input within lkjscript. It preserves all
compilation owners, not merely the selected entry's reachable closure. Target
selection affects ELF entry behavior but not the complete canonical artifact.
Physical split/pack targets are not logical collection quotas. Every generated
artifact re-enters the independent strict loader and provenance inventory.

## Dependency-closed capacity changes

The larger compiler first exposed source/package aggregate visit/read allowances,
then publication preview's 64 MiB staged-object allowance, then independent
canonical-reference proof work/storage allowances. These owners now distinguish
ordinary `None`, exact selected finite permission and passive saturated observation.
Optional cache retention failures fall back to complete validation; saturation is
never converted into an exact proof bound or buffer extent. Publication previews
stage and validate before atomic exposure and preserve prior state on failure.

The complete transaction bytes were identical on the reference-allowance refusal
and successful public adoption: SHA-256
`74215cf4a7298573de9d9768ad88407861d14e027654c002126cb2897244fab8`.
The former path returned `reference_read_view_proof_work`; the repaired default path
accepted the entire workload, followed by public readback and complete proposal
replay. Focused controls preserve zero, one-short, exact, finite MAX, cancellation,
late invalid meaning and saturation behavior. Seeded-counter tests are arithmetic
controls, not claims that their small fixtures executed ten million operations.

Source/package observation reports now carry explicit saturation fields and use
current `*-semantic-2` report selectors at the changed offline report boundaries.
Missing fields and predecessor selectors reject; no canonical epoch change or
legacy reader is introduced. Reference type/source/metadata observations carry
independent exactness flags through parent/child aggregation.

Canonical unit byte/item limits, bounded owner/type envelopes, u32 source-carrier
coordinates, recursive lifecycle/depth guards, map key/value sizes and catalog/GC
limits are separate remaining families. The larger source carrier and artifact do
not prove removal of a per-unit or per-object limit. Actual allocation/I/O failure
and checked contiguous/target-address extents remain errors.

## Byte movement and native scalar allocation

The closed pure intrinsics are:

- `core.buffer.append-slice(Bytes, I64 start, I64 end, consume ByteBuffer)`;
- `core.buffer.append-i64-le(I64, consume ByteBuffer)`;
- `core.bytes.read-i64-le(Bytes, I64 offset)`.

Append bounds require `0 <= start <= end <= source length`; word operations retain
all signed I64 bits and allow unaligned reads. Bounds, complete required capacity,
modeled allowance and physical allocation precede mutation. VM/reference append
uses physical 64 KiB cancellation chunks and returns no partial owner on failure.
Native paths preserve affine ownership and checked allocation; process-level
termination remains their cancellation boundary, not resumable phase cancellation.
Independent evaluator and native expectations cover empty/full/truncated/negative/
unaligned spans, signed extremes, overflow, refusal and retained output lifetime.

Source intrinsic admission and both output paths were implemented and checked
before adopting these primitives into compiler helpers. `wire::range`, fixed-width
word routines and related compiler loops avoid per-byte boxed temporary work.
The Rust native backend additionally interns immutable I64 values -1 through 255
and both Booleans in read-only objects. Its table is a physical optimization, not
a semantic value ceiling; other values keep their ordinary checked allocation.
A native control checks tags/payloads, unchanged allocator cursors for cached values,
MIN/MAX/outside-table values and retention across later calls.

Pinned comparison uses the same 17,263,085-byte artifact request, SHA-256
`9902bc145c7613b1ce0135c75d3c1a17d478127ddfb0617a7467c217ae2c4ee8`,
and the same compiler source carrier. Both experiments explicitly select a
24 GiB process address-space policy. The earlier allocator stopped with
`native_allocation`, no output, and sampled peak VmSize/RSS of 24,463,132 /
24,411,000 KiB. The scalar-pool backend completed in 18.304s, sampled peak
VmSize/RSS 16,041,940 / 15,798,460 KiB, and exact oracle artifact SHA-256
`52b8dce87b028aea8e7d83bf7e95f6e9c4fb61d8d704d20c180bc9a8bc32ce47`
(16,783,727 bytes). A failed timing is not a successful before/after speed baseline.

For the current 17,598,866-byte request, native artifact emission completed in
19.303s, sampled peak VmSize/RSS 16,757,452 / 16,452,192 KiB, matching Ahost.
These single-run process samples do not measure allocator traffic, copied bytes,
peak live language data or cumulative evaluator charges. No phase scratch is
reclaimed in these native processes; exit is not Milestone C.

## Complete raw-source fixed-point component

`selfhost_source_artifact_whole_maintained_raw_source_fixed_point` copies and
checks the original repository before any C0 compilation, runs native generations
with empty environments and only raw request bytes, and rechecks the original
repository's complete bytes and public source afterward. Each native invocation
has an explicit caller test deadline; the retained launch experiment also selected
32 GiB address space. Neither is a default language/compiler admission allowance.

| Result | Bytes | SHA-256 |
| --- | ---: | --- |
| C1 Rust bootstrap | Recorded in component log | `238ae6ebf1fd01770e5770bf8a04bb3bb185256b7fdad9bb51f17b151835b151` |
| C2 = C3 source-produced ELF | 3,869,405 | `ecef194cc2751ebb73d59ce4fed4e5c3c0353179e2affa7787a29aafd3d53b59` |
| Ahost = A1 = A2 = A3 | 16,947,150 | `59e1d8a6b9867e2ab2c43636dd8b75a0b4df8d2ca1ad7fe1d6607ef74f3430e1` |
| Mutated compiler ELF via C2/C3 | 3,869,408 | `24d3618b416e989ab63a1411274090f442421dc7cc83d0282be3c1e5ae4d3d19` |
| Mutated compiler artifact via C2/C3 | 16,947,099 | `41c53cee45bb252696f9fbd53718db7f960207e78dc5acc4f7dc900a755b66ad` |

Each original artifact independently loads 63,269 objects and 1,103 units. Complete
package revision, semantic state, interface, unit key/source identity and canonical
bytes match C0. C2/C3 execute independent 42/43 behavior and a compiler-source
mutation changes the malformed-request diagnostic from 12 to 77. There are 82
active-profile cases (26 positive, 56 negative) over both native generations and
both modes, plus selected binary/cleanup/tail controls and 12 bulk scalar cases
with three binary input sizes. Type/custody, unused declarations/types, untaken
branches and unselected targets are checked. Separate hash-consistent fixtures
reach diagnostics 12–19 and explicit allowance diagnostic 6 instead of stopping
at an unintended framing error.

The component command passed 1 test, zero failures and zero ignored in 524.24s;
its outer command, including release test compilation, took 714.09s. This closes
the Milestone A component for this source, not all Tier 2 requirements.

## Retained focused evidence

Paths below are relative to `.artifacts/capacity-unbounding/`. They identify
individual historical source-bound runs; only a later fresh staged-tree seal can
accept the whole finalized candidate. Reusing a digest does not retest later edits.

| Evidence | Receipt SHA-256 |
| --- | --- |
| `source-stage-controls-20261101/whole-source-round2/receipt.json` | `29c9087d37072113fb723469e2c0cc195366a412259e1e1e045f29cb177fb551` |
| `resume-ef8c/whole-source-outer-round3/receipt.json` | `f99e026b92963917f67c1af9828397b8fc8ec0328ff409f86ef1366705d06329` |
| `source-stage-controls-20261101/inventory-round2/receipt.json` | `51ccae1bf1b62ad57407b5bc214a27b3811799a651a90e32fe932d5d741a4176` |
| `source-admission-drafts-20261030/reference-round3/receipt.json` | `57c729f761d562c0ff3d6806b646c9bb8fcc91051d84b894a4cef74623c283fe` |
| `source-admission-drafts-20261030/public-former-boundary-round2/receipt.json` | `bb708fa3e281267388f46100597a9c1b9d5b14a0d90978a1429799366547b495` |
| `source-stage-controls-20261101/semantics-round2/receipt.json` | `861f2d5c24990d66e84f63c38b04b2fb064f49e6552429e03e92502c19d96bd0` |
| `resume-8d2f91/closure-round1/receipt.json` | `722a914a8e97825f59257d052e06fc7be56f97f723b17e72e4fe1050fb37ac55` |
| `resume-ef8c/pre-gates-round3/receipt.json` | `eb1b13dbd05849e0e7a5773e9c195abb02b9b987995dfadc390e9f8d076118c7` |
| `resume-ef8c/measure-before-pinned/receipt.json` | `c981944250a7c98c5a24c7e2d99c7dd15168412cec375c40ff41395881376ee6` |
| `resume-ef8c/measure-after-pinned/receipt.json` | `dd6dd96783dd7814985d645d9fcd0fda884ff85684ed020a0397b3059ed94024` |
| `resume-ef8c/measure-current-source/receipt.json` | `c5d967f43d9675a9798ff715810af11da61db4cc8e1718001ec726106cc95664` |
| `source-stage-controls-20261101/gc-round2/receipt.json` | `8b800a2bca9aef1a506a6bbd1b4ec17ea9fc23c7d782c640ddcadd784aeb8b1b` |

Public GC preserved the exact 21,795,545-byte transport, SHA-256
`86e2aeda5ad71147deb5c1e4e5e2f88ffb13b92ad20c7aaa1ae2afd8d5d3cecc`,
and its HEAD while compacting 200 packs to seven (191,469,725 bytes). All immutable
history remains reachable. Pack-byte totals are separate from live native memory.

## Seal and next dependency batch

Finalize code, proposals, accepted graph, generated discovery, specifications and
status before staging. Bootstrap the exact declared HEAD/tree; keep index and
working bytes quiescent throughout the literal selfhost script, named maintained
native closure and complete owner/type inventory, explicit raw-source fixed point,
fresh 29-gate full registry and release target. Record the final seal hashes in the
commit/external record, then fast-forward main and independently check remote
ancestry. A documentation-only follow-up must distinguish itself from that tested
tree. The old native-closure diagnostic fixture now selects unsupported F64 rather
than Bytes; its scope remains Tier 1 backend reproduction, not a second source proof.

The next architectural priority is sound child-phase allocation contexts and
survivor promotion with repeated native compilation in one process, including
rejection and cancellation. The current process heap and scalar pool do not provide
that capability. Remaining `P_self` owned aggregates/Match, closures/Invoke,
explicit implementation witnesses/read views and lexical Regions remain required.
The coupled capacity ledger remains authoritative for independent storage, codec,
lifecycle and service ceilings. No arbitrary constant increase is completion.
