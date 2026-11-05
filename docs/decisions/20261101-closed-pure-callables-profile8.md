# Closed pure callables, profile 8 and runtime child ownership

Status: renewed whole-source fixed point, four explicit controls and the repaired normalized runtime preflight passed; the paired pure-tail preflight, new full staged-tree acceptance and integration remain pending.

Updated 2026-11-03 (Asia/Tokyo).

Milestones A and C are already accepted. Their seals establish the earlier
maintained-source fixed point and same-process native phase reclamation. This
decision extends the supported source semantics and changes a private backend
projection. Public graph adoption/readback, complete proposal replay and the
refreshed active-profile inventory have passed for the snapshot recorded below.
The source refinements have a second public adoption/readback binding below;
its complete replay/profile and selected source-driven behavioral family passed,
including the 8193-prefix crossing under the unchanged explicit 32 GiB allowance.
The reference cancellation-control correction passed its separate focused
invocation. Earlier CLI/discovery preparation and public compaction passed with complete
transport equality and unchanged graph HEAD. Both failed staged-tree attempts,
the third-adoption repair and the later test-only profile correction are recorded
below with their distinct source and evidence bindings. Renewed fixed point and
the repaired runtime preflight subsequently passed. The paired pure-tail preflight,
fresh staged-tree acceptance and mainline integration remain pending.
Full `P_self` support, Milestone B, is not established
by this batch.

## Problem and decision

Named functions, prefix binding and indirect invocation need one coherent path
from source admission to canonical artifacts and standalone native execution.
A shallow Function type can hold a dynamically deep environment, so a type-depth
check cannot justify a default environment-depth limit or recursive cleanup.
Native indirect transfer must also preserve captured arguments, caller
continuations and outstanding ByteBuffer custody.

Implement the closed pure callable subset together with its source admission,
both lowering routes, backend checks, native transfer and ordinary-value storage
dependencies. Keep exact semantic identities in source analysis and canonical
artifacts; use separately checked execution identities in the private projection.
The normative contracts remain [self-hosting](../spec/selfhosting.md),
[source input](../spec/selfhosting-source-input.md) and
[native ELF](../spec/native-elf.md).

## One current contract at each boundary

| Boundary | Contract after this cut |
| --- | --- |
| Normalized Mode 0 | `LKJCI001`, profile 8 only; profile 6 is rejected without fallback |
| Canonical artifact Mode 1 | Profile 7, unchanged |
| Complete source carrier | `LKJSG002`, profile 3, unchanged |
| Source request | `LKJSRQ01`, profile 2, unchanged |
| Canonical schema and instruction encoding | Schema epoch 1, unchanged |
| Native phase invocation | Existing `LKJPH001` ownership and Bytes-result contract |

Profile 8 distinguishes the incompatible private mixed Record/Function
descriptor layout, exact full function signatures and typed callable transfers.
The profile remains a u32 at offset 8; the independent optional-policy tag stays
at offset 12. The complete mode word at offset 112 must match exactly. No dual
profile-6/profile-8 reader or legacy migration path is retained. Maintained
producers, consumers and fixtures move together; incompatible derived backend
inputs and caches must be regenerated. Historical receipts keep their original
profile identities.

Canonical FunctionValue, BeginBind, Capture, Bind and Invoke already have their
current encodings. The Rust Mode-1 exporter now emits all five, preserving their
original instruction PCs; the current source artifact/behavior controls passed.
Native projection uses checked Pass rows for binding markers, preserving PCs
without treating marker metadata as a trusted capture certificate.

## Supported semantic scope

The source subset includes Unit, Bool, I64, Text/StaticText, Bytes, `List<I64>`,
structural records and closed pure Function values over admitted copyable types.
Callable parameters, results, factory returns and structural callable fields
use exact types. The existing closed I64 specializations of list length/get
remain supported; this adds no general generic specialization or new intrinsic.

Bind evaluates the callee first and then its captures in authored order. Empty,
partial, repeated and complete prefix binding produce the exact residual
signature. Empty Bind preserves the callee and creates no environment. Invoke
requires every remaining argument and preserves immutable captures for reuse.
Task/effectful signatures, owned/resource captures, retained borrows and
implementation/read-view witnesses are excluded. ByteBuffer-consuming or
borrowing declarations remain directly callable; they have no exposed callable
signature. An otherwise eligible callable may use owned ByteBuffer locals and
must discharge their custody before returning.

## Source and backend responsibilities

Source E1/E2 admission resolves endpoints and containment, checks lexical scope,
all supplied owners/types, exact signatures, ordered captures and custody.
Uncalled functions, untaken branches and unselected targets remain in scope.
Source Text and StaticText identities remain distinct even where their runtime
execution shapes coincide. Private descriptor interning uses mapped immediate
children and exact field-name windows; canonical lowering retains source types
and authored identities independently of that interning.

Source E3 derives both profile-8 native input and profile-7 canonical artifact
input from the complete raw request. It supplies no host-prepared semantic
certificate to a self-produced stage. TailInvoke is selected only with the
exact terminal tuple, matching result and released owned locals. Otherwise it
emits Invoke, required cleanup and Return. The backend independently validates
the projected descriptor, stack and custody transitions.

Rust native and source-native implementations retain separate private callable
layouts. Ordinary Invoke reserves its full outgoing argument window before
copying. Eligible graph tails reuse the current frame, move overlapping explicit
arguments in the correct direction before installing captures, clear only
nonparameter locals and preserve the original return record. External targets
retain their ordinary call convention. Rust native transfer also checks live
owned-buffer locals before using a tail hint. Both implementations checkpoint
even empty transfer loops. Ordinary non-tail native CALL growth remains physical
stack growth.

## Safe ordinary-value lifecycle

Opaque Values, Fields and Captures own immutable children through fallible shared
owners, using pinned `triomphe` 0.1.17 for safe fallible allocation and consuming
unique-owner extraction. The repository's unsafe-code prohibition remains
unchanged. Cloning these handles is shallow. Builders reserve storage before
publication and retain cleanup ownership if construction fails. Their release
protocol consumes the final strong owner safely and drains children through
already reserved links. The common queue also handles migrated list/map payload
owners, affine retirement and ready read-view cleanup. It makes no allocation
during destruction. `RawValue`, `RawArguments` and `RawValues` route their
guarded error/cancellation teardown through this protocol.

This scope is exact: `NormalizedValue` still derives Clone and has no custom Drop.
Naked raw Box-backed Option/Result/Variant spines can therefore recurse under
direct Clone or direct drop. Consecutive boxed-spine admission guards remain.
`owned_storage::StorageEscapeCheck` also retains recursive Function/Record/List/Map
traversal, its depth-greater-than-256 rejection and admission-item visit limit;
that owner is outside the selected closed ordinary callable route. These are
remaining lifecycle dependencies, not solved by the opaque-owner release path.

Raw structural observation comparison is a shared fallible, cancellable
operation. It compares callable metadata and captures, including distinct
environments, structurally; pointer identity can only skip the same immutable
storage. Allocation refusal is an error, never inequality. This operation is
separate from language equality and its incomparability rules.
Bounded Debug reports shape without traversing deep children. No raw-value Hash
or Ord implementation is introduced.
Borrowed static allocation-failure diagnostics avoid another allocation on the
selected error path; outward diagnostic conversion may still allocate.

### Independent semantic comparison

The ordinary-value lifecycle must cover admitted finite recursive values such as
`Node { children: List<Node> }`. Their language equality path uses two
independently implemented iterative semantic walkers: the VM schedules
borrowed value occurrences; the canonical reference retains ordered sibling
cursors. Both implementations passed the five semantic lifecycle controls in the
focused runtime group recorded below. A false result never skips later
comparability checks, including a
Function hidden after an unequal prefix or incompatible aggregate shape. Shared
owners are visited by occurrence, without a pointer-identity shortcut. Functions,
capabilities and other non-comparable values retain rejection. Language F64
equality is numeric. The walkers' separate graph-test observation mode compares
F64 bits while retaining the semantic comparability domain; it is not the shared
raw structural operation described above.

Relative pointers compare their exact type and complete relative coordinate:
arena, slot, generation and offset. Equality does not read the referent, validate
access or transfer custody; live access still requires the arena's capability
checks. Region capabilities remain incomparable. Region identities form a
distinct kind; there is no separate normalized Region carrier. The slice
type/storage API has no normalized slice-value carrier or language array
expressions. No slice-value equality or source Region completion is claimed.

Each walk consumes a finite occurrence or advances a finite cursor. A shape
mismatch schedules same-shape self checks; map cursors switch once from left keys
to right-only keys. Bytes/Text checkpoints advance through positive chunks of at
most 4096 bytes. These are termination arguments and cancellation intervals, not
logical quotas. Frontier reservations are fallible and contain only borrowed
references or iterator state, so failure/cancellation releases scratch without
moving input ownership. Record/List/Option/Result/Variant unary descent needs no
continuation when it leaves no siblings. The reference walker can retain a unary
Map cursor for its right-only pass and accumulate a fallible heap frontier.

Existing modeled policy charges remain their documented metrics, distinct from
physical owner-header sizes. A `None` policy setting imposes no cumulative quota;
every explicit allowance, including zero and `u64::MAX`, remains finite and exact.
This decision makes no new modeled charge for comparison-frontier storage.
Actual scratch allocation, passive collection traversal and cancellation-check
counts may change with the iterative algorithms; no new allocator-traffic or
peak-memory measurement is supplied.
No measured performance, copy-volume, peak-live-byte, mapped-capacity or RSS
improvement is attributed to these runtime ownership/comparison changes. The
separately measured source-copy refinements are recorded below.

### Canonical reference continuations

A closure relay such as `add(invoke(f), 1)` retains work after its nested call.
The reference candidate stores that work in checked, fallible heap continuations
and flat graph activations. The driver reads canonical expression and owner
records; no normalized instruction or VM execution decision participates.
A finite whole-body classification visits exact expression identities and selects
the complete supported expression closure: primitive leaves and Local,
If/ordinary Let/Sequence, Call/FunctionValue/Bind/Invoke, Record/Field/List.
Each selected child position has a continuation. Eligible pure activations have
ordinary value results and no effect, requirement, implementation or read-scope
parameters/sources. Broader functions or body forms retain their previous
evaluator as a whole, with their existing semantics and stack limitations.

Activation entry, result validation and exit share the existing canonical
admission and custody checks. The driver retains invocation proof before child
evaluation, preserves consume/borrow argument handling and the Local fast path,
and admits ByteBuffer locals and direct consuming/borrowing intrinsics. Tail
transfer retains its already-admitted outgoing target, exact modeled target
charge and live-loan restrictions; it replaces an activation only after cleanup.
Non-tail calls preserve the exact call-depth limit and observed frame/tail counts.
This change does not remove default call-depth or value-stack policies.

Pending argument/binding states and activations have no recursively owning
continuation links. Destination storage is reserved before publication. Errors,
cancellation and host unwind drain younger operands before reverse lexical
owners and restore scope, active package and transient call-depth ownership.
Cleanup allocates no storage. Body classification advances a finite worklist;
evaluation advances canonical operations or explicit continuations with
cancellation checks. Recursive programs may still run indefinitely according to
their semantics; the driver introduces no logical work or depth quota.

Existing modeled expression, call, external-call, allocation-byte/charge and
collection accounting stays at its original operation points. Classification
adds honestly observed canonical-owner reads; actual fallible frontier storage
and cancellation-check counts are separate from modeled charges. A test-only
scoped legacy-route selector permits shallow same-binary accounting comparison;
it is not a production switch or an independent program oracle.

Four driver controls passed: non-tail closure and record environments through
depth 2048 on 512 KiB workers; tail reuse and exact/one-short call depth;
frontier reservation refusal at zero/eight reservations with cleanup/retry; and
22 shallow legacy-versus-driver cases with legacy-derived exact expression and
allocation allowances. The fifth, cancellation with outer calls pending and clean
retry, passed after the test-host correction in its separate invocation. The tests use
independent VM behavior, retain the existing
positive workload and distinguish actual reservation refusal from modeled policy.
Canonical schema preparation occurs outside the small-stack worker, while the
reference invocation retains its ordinary per-invocation schema charges.
Independent static review found no unresolved issue in the guarded runtime
bundle `a8078d125f718380d31849b4272def90e04cef0455f1c569a3da869d68584bc8`.
The integrated candidate passed formatting and release all-target Clippy as
recorded below. The reference/fallback/policy selection recorded 30 passed,
one failed and zero ignored in 538.568042671s, including 8m46s release compilation.
The only unresolved control was the cancellation harness: its raw host lacked
the checked byte-word primitive used before the selected relay checkpoint. The
test-only correction delegates that decode through a host-free invocation of
the same engine, preserving the original count 257, depth 260, cancellation point,
cleanup and retry assertions. No product error-class correction was required.
Its independent review found no concrete issue. The corrected control passed
one test with zero failures/ignored in 395.640507465s, including Cargo's reported
6m34s release rebuild. The original receipt is `post-adoption-7/receipt.json`,
SHA-256 `96bb83ee2c44846bf83b2ad4a3339e97033119950768485e9d98462b00978352`.
Its 30/1/0 outcome is unchanged; there was no later 31/0 rerun of that suite.
The successful separate receipt is `post-adoption-9/receipt.json`, SHA-256
`c8ce08f1cc7637381aebae05a576989c0ab901231de0fd615aa28c1c5bca631a`.
It binds source inventory
`4ca00a27718032f3fa586ab8ad50460f7541649b6f30ee5f761eabeecb7f9548`,
root libtest `d3c882499d23ddee083a2d2156326d1907f25db78fdd16088a58940ddb43f7df`
and locked-Cargo proof `a87fcee34842d82c0196184bd85c4b3474b75d5510800e2e8e5af58f8364ef4e`.
Product runtime and compiler sources are identical to the preceding source run;
only the reviewed cancellation test host changed.

## Focused evidence before graph adoption

The successful prerequisite run used the shared target lease and
`cargo test --release --locked --workspace --lib`. Its receipt retains every
filter and exact command; runtime tests used eight test threads and the
producer/native group used one. Both groups recorded an unchanged source binding.

| Selection | Passed / failed / ignored | Command elapsed |
| --- | ---: | ---: |
| Runtime prerequisites, including all five semantic lifecycle controls | 47 / 0 / 0 | 381.836098606s |
| Profile-8 producer and native prerequisites | 14 / 0 / 0 | 1.296727113s |
| C0-only 8193-prefix extent probe, explicitly selected with `--ignored` | 1 / 0 / 0 | 6.035847214s |

The runtime duration includes the release compilation reported by Cargo as
6m21s; these command durations are not isolated compiler-phase measurements.
The five semantic controls cover deep nominal Node/List equality and late
inequality, complete incomparable-suffix scans, comparison-only cancellation
after ingress, frontier refusal and exact-policy recovery, F64 observation
distinctions, and real pointer/capability behavior with custody preserved.
The wider runtime group also covers post-effect result-owner failure and cleanup.

The 47- and 14-test groups bind source inventory SHA-256
`ffca52ab8ede2d8c41047a6b52033f95094ada8faf7cc77c1a9b4cf55be0bcdb`
and release libtest SHA-256
`7b5e0a1edd3ba0d94c3dad4abe7dedcd0f506859131401e069c7ac0b2ab986f9`.
`cargo fmt --all` completed in 4.677199305s, and
`cargo clippy --release --locked --workspace --all-targets -- -D warnings`
passed in 34.104376965s.

Successful receipts are under
`.artifacts/capacity-unbounding/next-closures-20261101/`:

| Receipt | SHA-256 |
| --- | --- |
| `pre-adoption-4/receipt.json` — 47 runtime and 14 producer/native controls | `19ac1ad1be18f4536d4f615afd30849d32c7eff3ffd07da3069f4e648b4d5478` |
| `feedback-6/receipt.json` — formatting and Clippy | `b9e565ba58bcfc94aeb32213207664b7f1da9935997da33f45fbbbf16abc6111` |
| `pre-adoption-3/receipt.json` — C0 extent probe | `9b9991c75153ef3fda4ccaf74d2b2eeec01460e9ceabb1c84d7c3262d9bf1795` |

The C0 probe separately binds source inventory
`bb668c1b2f659bfe4421e877863e4f2072c621a34ae6a3fe520955ce5e1be4c0`
and libtest
`1379e4bd564b6336470eaa7d84e04e9666954e08c9939c642761190a6a0cecb9`.
Its source binding stayed unchanged during that run. It measured this authored
8193-capture fixture without executing a self-produced source stage:

| Extent | Bytes |
| --- | ---: |
| Captured scalar prefix | 65,544 |
| Complete source carrier | 2,957,173 |
| Private profile-8 carrier | 16,600,050 |
| C0 canonical artifact, six units | 4,710,463 |
| Largest canonical unit | 212,455 |

The source carrier SHA-256 is
`d464402d7f029c810289465031db689eca43a2017f56daf0d1caea7f7296e38b`,
the private carrier SHA-256 is
`bf3af204480466fa6b4a9895cbc06f01f4da52d1468f4a12cb4e3c06ed0af73d`,
and the C0 artifact SHA-256 is
`1702a174e484aa390d6023bb25db18bec8c6b268c46beb0cce3cf1f4a5ada265`.
The prefix exceeds the 64 KiB physical target by eight bytes. The largest unit
remains below both the 4 MiB storage envelope and 8 MiB codec ceiling; the larger
aggregate artifact does not establish large-unit admission. This early C0-only
probe did not establish source-stage execution; the subsequent complete crossing
is recorded below.

These receipts explicitly record `full_seal=false`. They establish the named
focused controls and C0 extents, not public compiler-graph adoption, a maintained
source fixed point, or final staged-tree acceptance.

## Public adoption, replay and active profile

The compiler proposals were accepted through public GraphRepository operations.
The following identities bind the completed initial callable adoption, before
the source-performance refinements. Their second public adoption/readback and
renewed replay/profile are recorded separately below.
The submission completed in 78.579464305s; public readback completed in
111.121583404s and covered all 68,233 owners in 17 pages plus five changed
signatures. Complete proposal replay and the whole-owner/type inventory each
passed, two tests total with zero failures, in a 146.884543145s command.

| Adopted input identity | Value |
| --- | --- |
| Semantic revision | `rev_5b43bf152732b9ca3da21eece306bbe2d65435d81d43d3c87c285d724d620fd0` |
| Public HEAD SHA-256 | `245761dabfb211b42b69966cc58b09505df4476a10b89aab9c368eec845ef749` |
| Semantic state | `semantic_state_0e33e9ab9717e8d9d7f5d91928063014f51db484214714e2569efa8ce8a1575d` |
| Complete source carrier | Profile 3; 21,200,959 bytes |
| Carrier SHA-256 | `6ad6f00fbdd777136a7ec49f6234b8fd0365a0016c7bb7cc402370d0a93f903f` |

The refreshed inventory contains 123 modules, 89 supplied types and 1,183
callables: 1,156 pure and 27 external. Parameter modes are 3,874 unrestricted,
319 consume and one borrow; all 36 generic application sites are inventoried.
The accepted proposal snapshot contains 123 files. These are inventory facts,
not a new inferred-type/custody certificate or native-memory measurement.

Evidence under the same artifact root:

| Record | SHA-256 |
| --- | --- |
| `adoption-1/run-receipt.json` — public submission and readback | `3e50371b677872315f2a06c31234f715cfc5f408ff1d01e6786c6b6232998b12` |
| Public readback record | `f8c948cc7ee85979c084c5e0230694e67812d68d79062d1e4a3746f438863198` |
| `adoption-1/accepted-proposal-snapshot.json` | `ebe0a00c1a8ff06b7545880f6cbfc78eaa4dbac720f049080d59b0b50cd739aa` |
| Complete active-profile JSON | `96c0629f2ff4486e61f2a42022e79ea8696a63ab75aa5c887ccd9aa34da7ea14` |
| `adoption-profile-summary.json` — compact replay/profile evidence | `2ebb7667cef2ee23d4e783dcd714b56cbce1b1b75f6d9610508eec9c7d1cd805` |

Replay and inventory bind source inventory SHA-256
`356313eb82816a210848d2dfb4305d00dcd96f0091081aacccebbea71acf41e3`.
They do not establish the renewed native compiler or artifact fixed points.

The earlier source-wide prefix control passed at 129, 257 and 1025 captured
arguments; the current graph reran them successfully in the selection below.
The subsequent Rust exporter
and fixture-only changes passed formatting and all-target release Clippy, the
latter in 34.017771836s; `feedback-7/receipt.json` has SHA-256
`a1face548db561ba05c3b06330e837033c60b874287dfd5dd1ec80890e35948e`.
These Rust/test changes do not alter the adopted compiler proposal snapshot or
format identities. Subsequent feedback-8/9 changes are test-only fixture repairs
and a new C0 continuation sensitivity control. Formatting and release all-target
Clippy passed, with Clippy taking 35.558s; `feedback-9/receipt.json` has SHA-256
`2e8470c5b48837829aaf4bc768e98455b414ab307ab3ccb7257dabbaa664d954`.
The integrated canonical reference driver and source refinements subsequently
passed `cargo fmt --all` in 4.972352250s and
`cargo clippy --release --locked --workspace --all-targets -- -D warnings` in
40.623586717s. The `feedback-10/receipt.json` SHA-256 is
`6e4679a085f16841595fbc4720176ccb1b9da9a11c768c8eb1b46f54bd97f891`.
Three direct-helper tests passed Clippy under `feedback-11/receipt.json`,
SHA-256 `ba8d5ca127537c5d97d2a0c577c859d3ace2687d1ff2d92e2fabbcc541b7e75b`.
The 31-test reference selection above and 12-test source selection below retain
their separate outcomes. The final callable inventory control supplies an unused
unsupported F64 descriptor, which is a valid canonical type: the repository host
rejects its unreachable supply with `kernel_full_type_unreachable`, while the
source profile reports diagnostic 13. All 14 callable inventory negatives passed
in the current source selection.

After that source run, the only changed source file was the reviewed cancellation
test host. Its formatting and full release all-target Clippy passed in
4.730143493s and 37.335187699s respectively; `feedback-12/receipt.json` has SHA-256
`d7fde605b4e194a889e41dfd3c2bc78ada96cc43fb4b3a0042f06a5c8f4cface`.
That static result does not change the reference selection's 30/1/0 outcome or
stand in for the separate successful cancellation execution recorded above.

### Second public adoption and readback

The source-performance follow-up changed six modules and 30 members with no
deletions and one changed signature. Public transaction and complete readback
passed for 69,147 owners across 17 pages, including that parameter-order change.

| Current adopted identity or record | Value / SHA-256 |
| --- | --- |
| Graph revision | `rev_5f7d680c46880ff835b94175662205d39c94a81fe152c3cc7c3ed5e8bef2fe92` |
| Public HEAD | `46f8619ded773cd96e2552962589f1ca2e14e66cfdf6744d2f2e8a45ea930845` |
| Transaction record | `b8e870dfce8149033a45950c473a5bd0a215e618fa0fc1f2756d8de12c9a9f2e` |
| Preparation record | `803b4b5e6f6fa981c3a214e68c5fc2065b2246fe06062833d46d9d2d99c0cbfe` |
| Public readback record | `900d2a898c18b8ac1e562c01efc0ea865bfa596f487370a8e357caa7957931a2` |
| Public run receipt | `db8937b1854a6853ecf10d8515d03f3f027388bc2dde0f902ac2ac167426463f` |
| Semantic state | `semantic_state_6d1cb97f28be15c08ec4dfde375992e7255ef9b39872e7c134f806e8b6ce83a2` |
| Complete source carrier | Profile 3; 21,541,870 bytes; SHA-256 `45604f6c429431e2632b9963503879dae2291b48470d07678e4a7a67a9fdf8e3` |
| Accepted proposal snapshot | `c9016fcac48441d3fddc665f0165abe7d70e4b2052eaba8a651bb84a25d0fe51` |
| Complete current active-profile JSON | `e5cba41ebfcc510bc43bec95b3328c2a36d99e6d3c78e1377a74fb54656a151e` |
| `adoption2-profile-summary.json` | `bf706ced1dbc49cbbff57694bcdf0d08a7da07c2279609e8deb2b0bb45ead528` |

Complete proposal replay and the refreshed owner/type profile passed. The current
inventory has 123 modules, 69,147 owners, 89 supplied types and 1,199 callables:
1,172 pure and 27 external. It contains 58,717 expression owners and 4,828 lexical
bindings; parameter modes are 3,937 unrestricted, 329 consume and one borrow,
with 36 generic application sites. The current C0 artifact has 1,206 canonical
units and 19,343,144 bytes, SHA-256
`6c541915db4c16812d2f47e3fa0d541bc75e31de494c7db543b0690f757c2141`.
Inventory is not an inferred-type/custody certificate or native-memory measure.

### Completed current source controls

All four source groups ran under the shared target lease with release, locked
workspace library tests. Their receipt records exact filters, thread counts,
binary digests and `full_seal=false`. The last group explicitly selected ignored
tests with `--ignored` under `prlimit --as=34359738368`; neither test was skipped.

| Source selection | Passed / failed / ignored | Command elapsed |
| --- | ---: | ---: |
| Complete proposal replay and active owner/type profile | 2 / 0 / 0 | 135.589699128s |
| Direct source-allocation helper projections | 3 / 0 / 0 | 32.663452554s |
| Callable behaviors, complete negatives, execution shapes and wide prefixes | 5 / 0 / 0 | 240.652751883s |
| 8193-prefix complete crossing and retained native profile export | 2 / 0 / 0 | 212.943431784s |

The terminal receipt is `post-adoption-8/receipt.json`, SHA-256
`2ec62232b4e02eac1f447e4cff39a3c3dfa014a0ab0aa800465d9cb55579f1ec`.
It binds unchanged source inventory
`c3c33a6cb83f17f4f0a72c90e321a1631a8ccbd1d8c01156dea6802d71a7fc4d`
and release libtest `4a15c8d23793f9134f1c6c3bbb373635ebd9d6b037f3a05503ee6c9a6e9280f4`.
The original reference selection uses that same source/binary binding. Its
test-host correction is a subsequent test-only change with its own passing
invocation, source binding and binary digest recorded above.

The current 129/257/1025 and 8193 cases cover both native output modes, cold/repeat
determinism, exact Rust canonical artifact agreement, strict loading and four
independent engines. The 8193 case returns checksum `119833`. Its 65,544-byte
captured prefix crosses the 64 KiB physical target; the 4,710,463-byte artifact
still has six units, largest 212,455 bytes, so neither unit ceiling is retired.

The retained `post-adoption-8/native-profile/compiler` is 4,982,082 bytes,
SHA-256 `9135d6f87a32b0c44ac89307d237906c80e242f37da895892bc3c92be5d04e20`.
That exact compiler digest appears in both 8193 cold/repeat logs. They emit the
same 15,680,346-byte ELF, SHA-256
`5b64bfdd827502a4719614e004279981d3eaaa1fc8aa00adff99662c6f38b597`,
and the exact Rust artifact, SHA-256
`1702a174e484aa390d6023bb25db18bec8c6b268c46beb0cce3cf1f4a5ada265`.
Uninstrumented native elapsed times were 31.250/31.162s for Mode 0 and
16.551/16.553s for Mode 1. These timings have no memory sampler and remain
separate from the observations below. Repeated C1 execution does not establish
the maintained `C2 = C3` or artifact fixed-point equations.

## Source projection and copying changes

Six publicly adopted modules remove repeated work without changing any current
format or adding a new intrinsic. `source-scalar-profile` and
`source-custody-signature` retain callable data/cursors across parameter walks.
`source-unit-tables-signature` emits parameter rows and local slots into two
consumed ByteBuffers, freezes each once on success and discards them on failure;
existing prefixes and order remain exact. `source-interface-record` reads a
checked visibility window while retaining full name parsing. The artifact-owner
and closure modules enumerate parent child arrays once, independently validate
namespaces/endpoints/parent identity/uniqueness, sort membership rows and merge
them into classification flags without repeated whole-array scans.

The classification is derived inside the compiler from the complete raw graph;
no request accepts a precomputed membership certificate or filtered owner set.
Closure emission still visits all source owners in their original order. Cursor
advancement over finite owner/parameter arrays and existing interval-halving
merge sort supply termination; affine builder custody is preserved. The three
direct-helper tests and complete source controls above validate these paths.
Any copying quantities inferred from the old loops are source-derived cost
counts, not measured allocator traffic or copied-byte telemetry.

## Pinned native before/after observations

The direct generated-C1 sampler used an empty native environment and an exact
32 GiB process allowance on the same 8193-prefix requests, machine and observer.
The before run began at 2026-11-02T03:15:47.213Z and the after run at
2026-11-02T21:55:17.756Z. Their separation matters when interpreting elapsed time.
Both use observer SHA-256
`b412c6c9a1b7f6172f55e3d6487d33e2ef80848ff0de6a147446c19d9d1d5ad0`.

| Pinned input | Identity |
| --- | --- |
| Before C1 SHA-256 | `c6fb2397e8b978d44989e67c5526b69796b328e3ad59ff9d61b7a8505ce67564` |
| After C1 SHA-256 | `9135d6f87a32b0c44ac89307d237906c80e242f37da895892bc3c92be5d04e20` |
| 8193-fixture source carrier | 2,957,173 bytes; SHA-256 `d464402d7f029c810289465031db689eca43a2017f56daf0d1caea7f7296e38b` |
| Mode-0 request | 2,957,205 bytes; SHA-256 `cce24fc6c6f05ca8b5931c04ddb4508a4e791a16f0b493685e3e77d36e0465f9` |
| Mode-1 request | 2,957,205 bytes; SHA-256 `a9bd99b55c7b03645ab8d70d462218cd4ec4f36c097a23d4a7ce22e598f83ce3` |

| Observation | Mode 0 before | Mode 0 after | Mode 1 before | Mode 1 after |
| --- | ---: | ---: | ---: | ---: |
| Terminal outcome | Success | Success | Exit 70, `native_allocation` | Success |
| Native process elapsed | 36.471380668s | 31.504901248s | 32.314081302s | 16.550420510s |
| Highest sampled `VmSize` / `VmPeak`, KiB | 29,401,140 | 9,117,068 | 33,522,628 | 5,503,472 |
| Highest sampled `VmRSS` / `VmHWM`, KiB | 27,042,744 | 8,725,032 | 30,443,300 | 5,085,464 |
| Highest sampled rollup RSS, KiB | 26,994,436 | 8,487,464 | 30,035,996 | 4,748,640 |
| Status / rollup samples | 354 / 8 | 312 / 7 | 310 / 7 | 163 / 4 |
| Missed samples | 7 | 2 | 9 | 2 |
| Observer CPU user/system, microseconds | 81,945 / 1,007,578 | 73,271 / 222,947 | 68,753 / 896,118 | 39,844 / 84,189 |

The sampler reads process status every 100 ms and rollup every 5 s. These
observed high-water fields and RSS samples may miss the terminal interval; they
are not exact process peaks, allocator traffic, live payload, mapped-slab
ownership or a phase-internal breakdown. Observer CPU is reported separately.
Mode 0 emits the same ELF and records 13.617% lower elapsed time, 68.991% lower
highest sampled virtual peak and 67.736% lower highest sampled RSS. These are
observations from separated fixed-input runs, not isolated phase causality or a
whole-suite speedup claim. Mode 1 moves from refusal to complete oracle-matching
output under the unchanged allowance. It has no successful baseline and no
equivalent-completed-run elapsed speedup ratio. Both successful after outputs
match the four-engine/strict-loader controls above.

| Observation record under the same artifact root | SHA-256 |
| --- | --- |
| `pinned-native-before-8193/receipt.json` | `71ce2246daef651189f23213f895cc0ff9c633d70d1679155691e3fa57aadc39` |
| `pinned-native-after-8193/receipt.json` | `60ad658ad7a23169c0e0e702a79e76c71b69f85044b0c7075f0921476c07a97f` |
| `pinned-native-before-after-8193.json` — compact comparison | `a954e8328953dca57b0873130b464b413ff538e2d93d2365cb50a77d9e5395dd` |
| After measurement plan | `27ce61ada7663a63d8fabccc043f4519ab917edc8e3c06d8ff286255e4154b0c` |

The receipts explicitly state `full_seal=false`; sampled memory does not prove
arena reclamation, and these C1 fixture compilations do not establish maintained
source self-compilation by C2 or C3.

## Current CLI, discovery and public compaction

The current candidate CLI was selected from Cargo's machine-readable build
output, independently bound to its source inventory and checked again for the
public operations. It is `target/release/lkjscript`, 80,695,256 bytes, SHA-256
`1a449412dba5c735f97ad061d6accee09d437af9dfa7941ed498a4cf0d55f995`.
This is the current candidate executable, distinct from the historical accepted
Milestone C CLI. Its source commit remains
`298a5556c7f3b80bf576b10af4f2ff39a95694b1`; the preparation receipt binds the
actual changed source inventory rather than treating that commit alone as the
candidate identity.

| Preparation operation | Result | Command elapsed |
| --- | --- | ---: |
| Release locked CLI build | Passed; Cargo-selected executable | 165.140180445s |
| Public discovery generation | Passed; zero generated changes | 0.903048104s |
| Public generated-discovery verification | Passed | 0.859698455s |
| Public complete package export before GC | Passed | 11.935143310s |
| Public GC | Passed; physical compaction | 89.028394746s |
| Public complete package export after GC | Passed | 11.654693864s |

The CLI/discovery receipt is `prefreeze-cli-1/receipt.json`, SHA-256
`0fd86822974f8a01534c5b6764dbb5d2f1a1d21b722eaaf992214bff6d15f8fc`.
The subsequent `prefreeze-gc-1/receipt.json`, SHA-256
`608fd1be8395f8b5003f03a64a0958706e09fdd409a70e14774688876c2fbe86`,
binds that preparation receipt, the same executable and the exact adopted graph
revision/HEAD above. Both receipts explicitly report `full_seal=false`.

Public GC preserved the exact HEAD bytes and all 425,333 retained objects;
zero objects were reclaimed. Physical packs changed from 264 files /
336,692,641 bytes to 12 files / 327,384,179 bytes. The complete before/after
transports were 26,517,565 bytes, SHA-256
`612494c7ae1fc32b1bd8d28280dc7447eb1a91c5d33abe3d9b27f56ac05c60b6`;
a complete streaming byte comparison independently confirmed equality. These
physical storage observations do not establish logical quota removal or native
memory reclamation. The remaining development-tool 256 MiB candidate / 384 MiB
verifier limits apply to individual executable files, not the aggregate source
snapshot; this batch does not remove those limits.

## First staged-tree attempt and repaired public graph

The first closed-callable seal was run against source commit
`298a5556c7f3b80bf576b10af4f2ff39a95694b1` and exact staged tree
`041c6193173d17e6d55dc9e97b05fee531d1791e`. Bootstrap passed in
204.857190303s. The first selfhost selection then ended with **219 passed,
6 failed and 9 ignored**, after 4413.363182513s including compilation; Cargo's
test process reported 4027.71s. The final source check retained the exact same
commit, tree, 2,331 files and 398,300,241 source bytes. The runner stopped before
the later named controls, fresh full gates or release target. This attempt is
a failed seal, not accepted evidence for the expanded candidate.

The six failures had separate causes. Five corrections were confined to
verification code; the dispatcher correction changed one compiler function
through public adoption:

| Control | Cause and correction |
| --- | --- |
| Complete admission above 1,024 functions | Opcode 17 now denotes FunctionValue. Use reserved opcode 0 for the unknown-form control and retain diagnostic 7. |
| Malformed Mode-1 complete bundles | The common dispatcher compared eight bytes at 112, so nonzero reserved bytes at 116 were reported as an unsupported selector. Read the selector as u32 and leave reserved-word rejection to artifact framing, retaining diagnostic 10. Mode-0's complete zero mode word and unsupported selector handling remain checked. |
| Ordinary-call machine-stack alignment | The former probe became a TailCall and skipped the ordinary entry patch. Force an ordinary Call with a following addition and assert the emitted Call/operand-depth route before checking all 16 stack positions. |
| Complete function-body exporter profile | A captured callable is now supported. Retain unsupported cases and add literal 42 checks for the captured program across independent evaluation and emitted execution. Unexpected support now reports a concise failure rather than dumping the entire accepted input. |
| Raw owner framing and unsupported expression forms | Bare FunctionValue 11, Invoke 12 and Bind 21 are truncated supported forms and must report framing 12. Keep genuinely unsupported tags at 13. |
| Complete-compiler nominal-flow native reader | The ordinary binary-filter harness's 10-second watchdog killed the whole-graph reader after its independent VM/reference checks. Source-reader tests now explicitly select 600 seconds for both native producers, retaining the 127-byte pipe input and exact exit, digest and diagnostic assertions. Ordinary/control helper calls still select 10 seconds. This is a verification policy, not a compiler admission allowance or a claimed speedup. |

The first repair preflight exposed one omitted argument at a second caller of
the consolidated syntax-reader helper. That failed preflight remains recorded.
After the one-line caller correction, the next preflight passed format checking
in 4.883445471s and release workspace/all-target Clippy in 35.464038711s. The
shared source-reader helper records the producer, complete input/executable
SHA-256 and total filter-call elapsed time. Watchdog failures also report their
elapsed time. The focused native reader control checks both Rust-native and
self-produced readers for syntax and nominal admission of the complete current
graph. Existing complete VM/reference controls remain in full acceptance.

Only `ci::validate` required a program-meaning change. Its 686-byte public
transaction used existing `ci::eq`/`ci::u32` capabilities; no intrinsic or active
semantic family was introduced. Public submission passed in 27.395406665s and
complete public readback in 88.484629642s. The accepted revision is
`rev_73e5ce5ee1420bcf6a84ac1237d5874447d4affa7d007e4e294e29a1e306c981`,
public HEAD SHA-256
`34cacbea820335d87be8e019ef83b309452bbf425abe3b9e977dd7744706bcf9`.
All 69,148 owners were read across 17 pages; no parameter order changed. That
readback is an owner-inventory proof; complete body replay is recorded
separately with the focused controls.

| Retained evidence | SHA-256 |
| --- | --- |
| Failed staged-tree terminal receipt | `1981ad0493cde414debd5a69e84014c884ed42473b4fd80ea5c282cf75b3f9bf` |
| Failed selfhost stdout | `cb88039b798f8ce60f762df3b388fae4fe9759b5cbbf1a3814c678444fbea706` |
| Failed selfhost stderr | `51156983250f83a0b1b6f11f0e9e4c6dd8ea0f7c3391fae5860d155924f8e079` |
| Initial repair installation, ending in failed preflight | `832814deca686f5cfb06755e65d779c5754367ccee91eabb5ebe6fc457ece330` |
| First repair preflight, failed | `db39755e6f6e5e7b0e28779b265481d57f1414dc43f217d79e07b28ee1646cc2` |
| Corrected repair preflight, passed | `a7eb0a86829b050c5105115d5e046213fed37de2acbed5260ef30eb9274979d8` |
| Third public adoption orchestration | `240069bb4c3405dd0d843c53fbfaed55fb4d066092c0778bd88e51aa47abbea6` |
| Third public readback | `86fdda03f7905229d576ae084cad4b00a7bc39d607cc200e7d5106175d0443f1` |

The failed-seal files are under
`.artifacts/capacity-unbounding/seal-closed-callables-20261102-round1/`.
The repair records are under
`.artifacts/capacity-unbounding/next-closures-20261101/seal-repair-1/`, and
the third adoption/readback under the adjacent `adoption-3/` and
`adoption-3-orchestration/` directories. Historical source/measurement identities
above remain historical; this dispatch change requires fresh current profile,
fixed-point and full staged-tree acceptance evidence.

### Repaired focused acceptance and refreshed inventory

The seven repair controls passed 7/0/0 in 721.829205976s of command time
(723.254340110s through source/binary binding). Cargo reported a 6m44s rebuild.
Complete proposal replay and the refreshed profile passed 2/0/0 in 125.306748592s
of command time (126.721528231s through binding). Both groups used the same
lease-bound root test binary SHA-256
`fab6140c93f0e0bc3abcc75c70daaae9b663de86c0006d160a47c842f4b7d072`.
Focused terminal receipt: `cb1a76de2986573868c7ff3577293143f84131808e6e00bbb76f56d1b848f3d3`.

The new complete carrier is profile 3, 21,549,935 bytes, SHA-256
`774d9816aa93c3d09b5a58b25a67798e96492f5d0b1b37f39415f15da1465503`; semantic state
`semantic_state_9f562cab7acf5ac819314f95f61e9ff27b88921449af18490bd9be3181a71361`, revision-core SHA-256
`3e2cdd82173fb55a078ba39c15b3db2a25c99f6eb10ca6ce3c5c7e31fe546bd9`. The inventory retains 123 modules, 89 supplied
types and 1,199 callables (1,172 pure/27 external), with 58,718 expression owners,
4,828 lexical bindings, 47,263 retirements and 36 closed generic application sites.
Parameter modes remain 3,937 unrestricted/329 consume/one borrow. No new intrinsic
or active semantic family was added by the dispatcher repair. Full profile JSON:
`962ef0d73acd66b94d452dd044023e1293f47e4102ee9ed7b0a97a1c472a6c15` (25,772,646 bytes). This inventory does not infer
custody obligations or measure native allocation/RSS.

The focused whole-graph reader control exercised both native producers:

| Producer / reader | Executable SHA-256 | Total filter-call elapsed |
| --- | --- | ---: |
| rust-native / read-syntax | 60116934fd24b5f1483fee1e6ec370c9c9778f3962d48d8a2ba692f5439d2a51 | 10.279s |
| self-produced-native / read-syntax | c9a92c246f19866403fe7825e8ba05a18109ff3378ee6a51e28178b9fb0a0531 | 9.882s |
| rust-native / read-nominal | d9dcdcd9763790a5a153ea6bac09fd756a4f9873782a4ee656ab381e30493ca8 | 10.508s |
| self-produced-native / read-nominal | 78a526af710e3393c1cdf7ef69aca923d31c4d645ceb74d341ba299465f58483 | 10.689s |

Each run consumed the complete carrier above, with an empty environment and
127-byte pipe writes, and returned exit 0, the exact 32-byte semantic digest and
empty stderr. These times include executable publication, launch, pipe transfer
and output collection. They do not isolate validation phases, allocator traffic,
copied bytes or RSS and do not establish a before/after speedup. Three observed
calls exceed 10 seconds in total; the failed first seal independently records the
actual old watchdog refusal. The new 600-second allowance is explicitly selected
by this verification helper, not by the compiler's ordinary resource policy.

The CLI/discovery receipt before the second attempt, `8dbc3c880d385e7cc9c7909f9aae60ce972d0b7b19078f1b7dd452aa20fb863f`, binds the unchanged
80,695,256-byte executable SHA-256
`1a449412dba5c735f97ad061d6accee09d437af9dfa7941ed498a4cf0d55f995` to the repaired candidate. Its build was
0.120880485s; generation/verification were 0.870691344s/0.869615379s, with zero
generated-file changes. These focused/preparation records do not replace a fresh
full staged-tree seal of the repaired graph.

## Second staged-tree attempt and test-only profile correction

Round 2 ran from 2026-11-03T02:38:19.874Z to 04:10:05.490Z against
source commit `298a5556c7f3b80bf576b10af4f2ff39a95694b1` and staged
tree `d17316f77c9319e142763c9df414151cd788b482`. Its terminal status is
**failed**. The final source admission retained that same commit and tree,
2,349 files and 402,705,539 tracked bytes. Public graph HEAD remained
`34cacbea820335d87be8e019ef83b309452bbf425abe3b9e977dd7744706bcf9`.

| Operation | Result | Recorded elapsed scope |
| --- | --- | --- |
| Staged-tree bootstrap | Passed; original checker retained | 11.403298022s command interval |
| Bootstrap/checker identity bridge | Passed under the shared target lease | 0.187968776s worker interval |
| Literal selfhost script | 226/0/9, then 12/0/0, then 26/0/0: **264 passed, 0 failed, 9 ignored** | 5435.580719306s whole command; libtest reported 4935.51s, 0.00s and 119.31s |
| Separate maintained-native-closure control | **0 passed, 1 failed, 0 ignored** | 56.121063887s Cargo; 55.98s libtest; 56.280060696s enclosing worker |

The failure is the explicit `assert_eq!(word(&mode0, 8), 6)` in
`selfhost_source_measure_tests.rs:111`. The observed value was 8, matching
the current producer and specification; the test retained the former profile.
The recorded exit was 101 with no signal or resource-failure diagnostic. This
is a stale verification expectation, not an observed compiler rejection or
orchestration failure. The locked proof still records the Cargo-selected image
digest after the failure. Its success-only `named_test` inventory is null
because the runner executes `--list` only after a passing Cargo command.

Before the failing assertion, this invocation had reached the backend Mode-0
`C2 = C3` check, equality of Mode-1 outputs across C1/C2/C3, and strict
artifact loading. Those checks do not turn the enclosing failed control into
a pass: its later combined source-compiler native closure and independent
raw-source behavior/rejection checks, final measurement output and stage
identity report were not reached. No renewed raw-source fixed point is claimed
from this partial execution.

| Round-2 retained evidence | SHA-256 |
| --- | --- |
| Failed terminal receipt | `a34b6c64d1782d2170ed3502fb6c39839aed258c560f8a34c0f545240b520492` |
| Bootstrap receipt | `028e73a7e33581cbd223142cc5d65cdd5f8e46db9137c6bac194309a8b540c74` |
| Leased bootstrap/checker bridge | `cabcfef85b893904db5f9ddf48eb97e64c7d3bcc480d9233ea8817e010634615` |
| Successful literal selfhost stdout | `7c01ed9ba04949e157caadd4c84cad47f9e5394dd23f827e213e437269cd38c8` |
| Failed maintained-closure locked Cargo proof | `0574e371b88f10c16676ccd276d4a2c582ec361e7e204cdb7ecbf1359b649f5d` |
| Failed maintained-closure stdout | `a03a4b46517eee7a38b7f2a38132fe0aaec5a820ecc5c2f23b14c6f0e20b30d1` |
| Failed maintained-closure stderr | `44242f67f8f9ee804089f14c653358431b37f7b2bfb98c12dbab200aa529080c` |
| Selected libtest executable | `d9ab9a74f10cb6bfd19981c58e175f675ab0fe32faa27220ef70748b99645d4f` |

These files are under
`.artifacts/capacity-unbounding/seal-closed-callables-20261102-round2/`.
The literal script's existing evidence scope is unchanged: it holds its own
target lease and checks input hashes, but does not emit a separate selected
executable digest. The named control's later executable proof belongs to that
named invocation. The fresh full checker was not invoked and supplies no
round-2 harness or gate receipt.

The subsequent correction changes only the literal expectation from 6 to 8.
The measurement owner moves from SHA-256
`2fe43798ed8fa11d8f2a65aadc776446ace8b6f244fbce87384142e6904adbd0`
to `449edfff00b4459f9dd45c1cfd4ff91a7be00ad8010263186a2a07c34a6b6790`.
Compiler proposals, graph meaning, format contracts and resource policies are
unchanged by that edit. Its focused retry under
`.artifacts/capacity-unbounding/next-closures-20261101/seal-repair-2/focused-1/`
subsequently passed, as bound below. The next staged-tree attempt has no completed
seal or assigned tested-tree claim here.

The separately named owner/type inventory, explicit 8193 crossing, raw-source
fixed-point control, all 29 fresh full gates and `release target` remained
unexecuted by the round-2 driver. Final acceptance requires a newly frozen tree
including the correction and completed documentation; the historical failed
`d17316f7` tree and earlier focused passes cannot supply that seal.

## Renewed fixed point and normalized-runtime preflight

The measurement expectation correction completed its focused retry from
2026-11-03T04:13:36.462Z to 2026-11-03T04:58:33.258Z. All four
explicit controls passed (4/0/0), after formatting and release all-target Clippy.
The receipt is `3347a6c87e5526e0075763cb52e0a745a8e4d387ac86afa19362fcc2312fd57e`; its original Cargo-selected
library-test executable is `3e67d885019658c1cd7f419477826406f8252b29d7164ce7dee36f17f6736ff2`.
Before/after source snapshots match at `24d899f29e09110c45dc054bfa3d64c85c37942a404f40440bd0f9550ef9ff39`.

| Focused selection | Passed / failed / ignored | Recorded command interval | Locked proof SHA-256 |
| --- | ---: | ---: | --- |
| Maintained native closure | 1 / 0 / 0 | 453.108s | `731f46378984126a21c35dd80c839bafdb198058ad8d69fbcf566c65f9d6bfd7` |
| Complete owner/type inventory | 1 / 0 / 0 | 41.930s | `c06b49ebadb0bba09ade61e5c03b73c7d90a271eaa38b4490dde6691380f974d` |
| Explicit 8193-prefix crossing | 1 / 0 / 0 | 141.128s | `152397e2763a988ff82f352b6774bf8d6e4b8a9aa47890af13047e7dd57cfa20` |
| Whole maintained raw-source fixed point | 1 / 0 / 0 | 2017.832s | `5620b8666fed41330c5e59d646445fc248f12c4c10222a22825f79462fe51f11` |

The explicit 8193 and raw-source controls retain the selected 32 GiB process
allowance. Their command intervals include orchestration/Cargo costs and are
not isolated compiler phases. Formatting and Clippy took
4.787s and
36.467s respectively. The renewed
8193 result does not replace the earlier pinned before/after observations.

The whole raw-source test reached both equations, independent behaviors 42/43,
the source-compiler mutation diagnostic 77, strict artifact loading and its
final unchanged-original-repository check. The intermediate log's
`original_recheck=pending` is followed by the terminal
`fixed_point_component_passed=true original_repository_unchanged=true`.
The complete test passed; no partial footer is used as terminal evidence.

| Renewed raw-source binding | SHA-256 |
| --- | --- |
| Complete source carrier | `774d9816aa93c3d09b5a58b25a67798e96492f5d0b1b37f39415f15da1465503` |
| ELF request | `dab546aa7f1647111c317104e7227d636d7c7c10419ec3c3ba3063bacb916dbe` |
| Artifact request | `3657595d3db1aa37acc28c42f548e71e9ef5cc3b136581122a53906a563467e4` |
| C1 | `0a362e7650e36f9b0f59a5b0d0ed082365b16406bb7cc6310eb04a700687004e` |
| C2 = C3 | `83c5b0eccbd43edda63afee90baf5da62228a86a7feea2e371b9773435a5d03f` |
| Ahost = A1 = A2 = A3 | `27a0c93c346b893033b43c29ea073ea312051a88860c4d2e4324d1fc660a5d34` |
| Mutated source carrier S' | `5872a606dd7096cac7ae5c8de843fda62178197e0d7911285ba86e141ba4850e` |
| Mutated ELF from C2/C3 | `ce8f0e1949715b14b9428b07d5d7cdd032c976c4ea6bd5821bfe659bd9f45e63` |
| Mutated canonical artifact from C2/C3 | `2d0e75120c88b4980953d37ebdac4364451cef6634d7d9477da9b85b0950b512` |
| Complete raw-source stdout | `f65a01049210e3957b299111d98b173f867d680b37331d230706df885fe6c8ad` |
| Complete raw-source stderr, including terminal recheck | `a0f1e3575fa140b0abe5b88606d2310c4f78d792370508d1b72aa04e00404f58` |

### Seven-path evidence and specification reconciliation

The subsequent normalized nonselfhost runtime preflight retained its original
930 passed, 4 failed and 2 ignored result in receipt
`fc0d28b0d124b9c9136e41f2cf4b1ccfa8117564a0128294aa96524297509745`. The four failures were stale evidence
expectations: the old recursive route reads the complete independent owner
inventory plus eleven records, and the canonical continuation route performs four
additional pinned selection reads beyond that baseline; opaque Map/List edges
no longer share the removed total raw-value-depth guard. The unchanged
explicit allocation/item policies were not relaxed.

The seven-path repair keeps the old owner-read count under the test-only legacy
route and checks the current route separately. Existing Option spines of 255/256
and additional opaque Map shells are positive admission/comparison controls.
A new canonical recursive Named Variant fixture reaches the remaining Box-spine
guard independently of structural type depth: 256 payload edges admit, 257
refuse at input node 258, with exact tier diagnostics, cancellation at node 37,
cleanup and healthy retry. The pure-tail probe and strict receipt consumer now
agree on a different deep malformed-input path: a List<I64> containing 100,000
Option wrappers fails typed admission at node 2 as Infrastructure, before effects,
and retains the stack-safe disposal obligation. Full exact admission-message
prefixes are checked. The language specification now distinguishes these paths.

| Repaired owner | Before SHA-256 | Formatted verified SHA-256 |
| --- | --- | --- |
| `src/platform/execution/normalized/vm_checked_tests.rs` | `44a83bc06fa8b4ca1e4262e94d080afb1201888ecbcde943c925f0610e6a39a7` | `215d2bb56297b4e8501b757ae45d007edb427ebf3ff8b8261f5c197665323a78` |
| `src/platform/execution/normalized/tests.rs` | `cc45b3794799aca9d8f12fcf5b7c932aebe25510094f79b4ab03edc0ccc79f42` | `d458855efc117896090be31e6a5df5f769b314939c5007e64fe66cf818b3d0e5` |
| `src/platform/execution/normalized/vm_map_tests.rs` | `990b38efc57b288e217902542dba19bc10dbadc881e75f99765332826a51d7be` | `b6dec336716f984a8d548bbc7735e93c2f8048c59c581de9ac52c2838ece20f2` |
| `src/platform/execution/normalized/recursive_tests.rs` | `346b062fe1104168bb406ae64b704a07f05fe3db9aaa8ae23defc5b3114606c5` | `7253fb2ecd690b941f13d511f6df66c1b87bd0382ae58b3535c1addbf2ad64fb` |
| `src/platform/execution/normalized/pure_tail_probe.rs` | `b2af2038de8a798f2fa7add2a12961e968f4cb2014f2e5157c285c377aa114d7` | `af1a0f6fc65ffe40d30e44806bdaae91fbfad9eba02e1442179f8d4c92daa853` |
| `tools/lkjscript-dev/src/pure_tail.rs` | `f867a7289050b069fffd640c4f0ac98db6e8a8e09fcf68fd8e0aaeb71b7a6138` | `0870ab300e7734febeaf206b3fae9e558024453bdbbffd16cfdf0efc6fda801c` |
| `docs/spec/language.md` | `3bbc250126a4cd3d0f583f67aff25526bfd88e843922957cc073019be06b3893` | `b83f27311bd7960fbd3534696ad91b9cc3a2a535e9c9f05ac1bff0a46eab81c8` |

The repair manifest is `b8d7d46b72af693193cc3e06143dcc625da451b204defc2ab49ddc071056185e`. These are
test/probe/specification changes; no compiler proposal, product runtime admission
algorithm, canonical format, caller policy or authoritative graph changed.

The repaired runtime preflight completed from 2026-11-03T05:20:29.968Z to
2026-11-03T05:30:35.410Z. Receipt `f30ead3cc099615f43c6b66aa9fbd1c6f9221c900e3e1ea78c83a681f6c6e6e8`
records **935 passed, 0 failed and 2 ignored**, with the exact pre-Cargo inventory
and all five selected repaired controls present. Its runtime command interval is
153.355s; library-test build,
formatting and Clippy are separate recorded operations, not compiler-phase timings.
The original and terminal repaired library-test executable are both
`dbc7e430da10e54958b031110ce0868eb1e101fe87af520f3d2bbcb3ae3e0060`.
This differs from the focused executable `3e67d885019658c1cd7f419477826406f8252b29d7164ce7dee36f17f6736ff2`;
the two runs must not be presented as a single executable proof.

| Runtime preflight binding | SHA-256 |
| --- | --- |
| Before/after source snapshot | `d3ecf503d7c3db9a2043a0b1d8995562f17fe3bf9ece194f6eea3e5b9c51a738` |
| Source input identity | `e58935d6b08e8ab54cdb34661cca00089480ffc46e5d64b2a2804629277da210` |
| Runtime inventory | `1f06fb783e31b8f49ec093566870afbab8c835871f9f1d4a43cb7f1b74a1e981` |
| Locked Cargo proof | `f4693dbb2cef757e3a9d4c68fe1aa06dae505535ba8f2b594c3b94474dad1e08` |
| Runtime stdout | `f1d710db5fe673377f2152a943d8c599b1baac218348e4700193d1e686d4c3c9` |
| Runtime stderr | `ad75c1ecc51927f55bc537c6807ebd1c8b72b50aaaa331e744f760b4253b4985` |

At runtime preflight the staged index remained `d17316f77c9319e142763c9df414151cd788b482` under
source HEAD `298a5556c7f3b80bf576b10af4f2ff39a95694b1`; that historical tree did not contain the
unstaged repairs. The measurement owner is separately guarded at
`449edfff00b4459f9dd45c1cfd4ff91a7be00ad8010263186a2a07c34a6b6790`, and public graph HEAD remains
`34cacbea820335d87be8e019ef83b309452bbf425abe3b9e977dd7744706bcf9`. Receipt source snapshots bind the exact changed
working files. A new final staged tree will be assigned only after documentation
and all candidate inputs are complete.

The repaired pure-tail proof has **not yet executed its declared preflight**.
Complete the final documentation and freeze, then bootstrap that exact tree for
the preflight. The unchanged full seal, including its own bootstrap, must still
run the literal selfhost script, all four named controls, all 29 fresh full gates
and release target on the same quiescent tree. Neither this
935/0/2 selection nor the renewed fixed point supplies that full seal or mainline
integration. Full P_self support and repository-wide unbounding remain open.

## Remaining work and acceptance

Nominal owned records/choices and Match, explicit owned witnesses and source-tied
read views, lexical Regions, arbitrary lists and general generic/effectful
callables remain outside this source subset. Canonical unit/object envelopes,
source-carrier/IR representation bounds, structural type/AST lifecycle, other
codec guards and general polymorphic allocation remain separately tracked.
Structural type admission and identity derivation, JSON/data codecs, boxed-spine
guards and recursive owned-storage escape checks are not retired. The VM's
unchanged default and explicit call/value-stack policies, broader reference
evaluation routes and ordinary native CALL stack growth remain distinct
boundaries. The selected reference heap driver does not remove those policies or
make the entire reference evaluator stack safe. Closure values do not escape
the phase ABI's Bytes output boundary; arbitrary-value promotion is not added.

Future nominal owned-record layout preparation also retains a 64 MiB cumulative
metadata ceiling: `kernel/owned_record.rs::LayoutBudget::default` selects
`MAXIMUM_LAYOUT_METADATA_BYTES`, and `reserve_bytes` enforces it. Its
`validate_type_depth` rejects depth greater than
`contract::MAXIMUM_TYPE_DEPTH` (256), including derived owned layouts. These
limits remain in the capacity ledger for a coupled layout/lifecycle batch;
ordinary representation-cycle rejection remains a separate semantic requirement.

Acceptance must establish the following on the assembled candidate:

1. Preserve the focused prerequisites recorded above in final acceptance.
   Their finite-policy and failure controls distinguish injected storage refusal
   from physical OOM; they do not substitute for the complete fresh gate set.
2. Exercise raw-source callables across VM, canonical reference, Rust native and
   source native, with exact canonical artifacts and strict loader provenance.
   Include deep shallow-type environments, retained aliases, exact semantic
   mismatches, complete-inventory negatives, source mutations, empty-Bind
   allocation observations, overlapping tail arguments and same-process recovery.
   Pass all five reference continuation controls, including shallow legacy-derived
   accounting allowances and cancellation/refusal cleanup, on the actual candidate.
3. Preserve the completed 8193-prefix source-stage crossing, exact oracle output
   and behavior under the explicit policy. Its scalar prefix crosses a physical
   target; the largest unit remains below the independent storage/codec ceilings.
   Keep the measured source result distinct from future canonical-unit unbounding.
4. Preserve the adopted graph, public readback/replay and refreshed `F_active`
   bindings above; refresh them if compiler proposals change. Repeat the renewed
   whole maintained raw-source closure on the final frozen tree: `C2 = C3` and
   `Ahost = A1 = A2 = A3`, with independent behavior and rejection controls.
5. Finish specifications and status, retain the completed current discovery
   verification, then freeze the index and working tree. Bind the exact source commit and staged tree through
   verifier bootstrap, the selfhost suite and named inventory/closure controls,
   every fresh full gate and `release target`. Commit and integrate that exact
   verified tree, recording terminal receipt digests without post-seal edits.

Renewed maintained-source fixed points have passed for the exact focused inputs.
The paired pure-tail preflight and expanded candidate's full seal/mainline
integration remain pending. Final acceptance must bind the adopted graph above to complete requests,
executable/artifact digests, exact tested Git tree and terminal receipts. Earlier
A/C seals, public adoption and the focused receipts cannot supply that expanded
final acceptance.
