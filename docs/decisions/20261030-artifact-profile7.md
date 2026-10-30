# Mode-1 Profile 7: wide projection and generated pools

Date: 2026-10-30 UTC.
Status: Rust producer controls passed; the coordinated language changes are
publicly adopted and namespace readback completed. The focused artifact selection,
four-way probe comparisons, complete proposal replay and public GC passed.
Native compiler and artifact reproduction passed within the first frozen-tree
run, whose owner joined with exit 1 after one stale fixture failed. The selector
fixture and two ownership-charge callers are now corrected; all 20 focused tests,
formatting and strict Clippy passed. Final staged-tree acceptance remains pending.

This slice follows the accepted list milestone on main commit
`932f197ebcd9c17e2702372300ab73996bb20754`, tree
`6717d8249c1db319169b2b32be0e088ef09dea21`. That preceding tree passed all 29 full
gates fresh, with zero reuse, and was integrated by ordinary fast-forward. Its
acceptance does not transfer to this changed compiler. The working branch is
`capacity/20261030-artifact-profile7`.

## Implemented source changes and public graph adoption

The Rust producer, selfhost admission/emission modules and affected probes use
one current-only Mode-1 profile, 7. Mode 0 remains profile 6. The cut changes
private normalized projection and generated-pool framing; it retains schema
epoch 1 and the canonical unit/map/pack/container contracts for unchanged IR.
No Profile-5 reader or zero-as-unmetered compatibility path belongs in the cut.
The complete layout is specified in
[artifact input and emission](../spec/selfhosting-artifact-input.md).

The 160-byte header carries explicit policy tag 0 (`None`) or 1 (`Some(u64)`).
The finite allowance retains zero and `u64::MAX` exactly. Counts, offsets and
lengths use u64; the unit/source directories have 16/56-byte rows. Unit prefixes
are 272 bytes, table directories 160, signatures 40, parameters 48, code headers
32 and instructions 48. Metadata retains its 312-byte prefix and 56-byte owner
rows, reading all eight count bytes. Numeric arrays and generated length frames
use eight-byte words; unit facts become 138 bytes while map bindings remain 82.
Canonical semantic indices still require exact u32 representability.

Projection-only 4 GiB extents, 8 MiB units, 100,000-entry inventories,
32,768-instruction/table and 128-parameter/local/operand guards are removed.
The signed language byte-index domain and actual buffer extents remain checked
before arithmetic, indexing and modeled output allocation. Map pages remain a
64 KiB physical unit, with automatic hierarchy growth. Neither a word width nor
a successful synthetic arithmetic probe establishes a physically realizable
allocation of that size.

Finite policy admission compares
`A >= 64*(64 + 2*L + U*U + O)` exactly through division and subtraction, without
forming an overflowing square or saturating a requirement. `None` branches
before that estimate; every finite allowance rejects a requirement above
`u64::MAX`. This contract is estimated proof admission, not a decrementing work
meter. Cheap framing and policy checks precede the carrier hash and deep scans.
The Rust producer shares a measurement/fill emitter and admits its exact output
size before a single fallible byte-buffer reservation. Prior artifact admission,
map reads, decoding and inventory preparation remain outside the embedded policy.

Emission checks object-byte sums, `144 + object_bytes + 89*object_count` pack
extent and `140 + manifest_length + pack_length` container extent before their
output allocation. It rejects negative/high-bit sizes, partial rows and
nonrepresentable sums. Passive observations never size buffers or grant capacity.
Unit-relative addresses remain relative when a unit occupies a nonzero bundle
origin. Profile-7 canonical opcode 10 remains `JumpIfFalse`; Mode-0 opcode 10 has
its separate tail-call meaning.

The nine changed selfhost modules were accepted into `tools/selfhost/project`
through one ordinary public transaction. GraphRepository remains the sole editable
authority. Complete proposal/body replay passed in its separate focused check;
maintained-output and final staged-tree checks retain their own obligations.

The artifact probe harness now compares production VM, disjoint reference,
Rust-bootstrap ELF and selfhost-emitted ELF results. It constructs the latter
through the Mode-0 native backend and caches probe executables by their exact
emission-payload hash. The focused artifact selection exercised the route and
passed; this does not by itself establish repeated native compiler fixed points.

## Observed evidence at this checkpoint

The twelve coordinated replacement proposals were guarded by exact source hashes
and applied after list integration. The producer verification driver joined with
exit 0 at `capacity-resume-1030-143`: 12 tests passed, zero failed or ignored,
2,842 filtered, in 0.01s test execution. Its focused compile/test operation took
366,873,597,353 ns. The receipt SHA-256 is
`abab2fe578d207b57d3606296619e268a672700d4df163f0310504cf97e2c0ee`.
Cargo selected the current 80,050,464-byte product executable with `fresh: false`,
SHA-256 `c25a6d576f62bfbe273ea26f1985bd5feaf2d5e45933fd51791c10c2b10ff6d2`.
These are producer controls, not a four-way compiler execution or full-gate result.

The public adoption owner joined with exit 0 at
`capacity-resume-1030-148`. The 79,847-byte transaction changed 69 members in
nine modules with zero explicit deletion operations; SHA-256
`c3f19cd392712801b4c011662f183e8cce614923851c974c2688fb3e1b7593f4`.
Submission took 39,268,461,095 ns and reported 4,626 created and 60 updated
identities, with `differential: equal`. Its current 28,895-byte preparation/submission
record has SHA-256
`a97bfbe66f62f81ea09e4e16d61919bfa41ef94a4142974e078a89452ea9aa4e`;
stdout SHA-256 is
`bfef4fc710a463815b5fbedfaf4551f5fd69bc354076a0fe9cfc91ff9283da62`, with empty
stderr. The transaction selected zero runtime tests, so that authoring result establishes
no artifact execution matrix. Its accepted revision is
`rev_b14f1d9465fd9d67cad9cc5946607a7028d2a2d26a2a2883ad953422f23a1b05`;
the public HEAD SHA-256 is
`f66f443af7f9e46179c56fc9600113fa72dc65b2284ceb6e4450a83494eccc79`.

Readback joined with exit 0 at `capacity-resume-1030-150`, covering 36,080 owners
in nine pages. Its 1,065,885-byte receipt has SHA-256
`5155d7b8f5f991a15c8971c9c3daeb2086f6b12aec3a44ccb765013d92332e8e`.
No existing parameter order changed, so there were zero changed-signature pages.
This proves the recorded namespace inventory at that revision. Complete
proposal/body replay passed separately in the focused run below.

The focused owner `capacity-resume-1030-152` joined through
`capacity-resume-1030-172` with exit 0. It ran from 06:14:07.602Z to
06:31:56.355Z on 2026-10-30. Its initial/final source inventory matched exactly:
2,269 files / 154,423,333 bytes, SHA-256
`ea226ed609fadb3f8476e3dafdb166a0e3552d97d400ea8362599d2260f5ef21`, with unchanged
public HEAD `f66f443af7f9e46179c56fc9600113fa72dc65b2284ceb6e4450a83494eccc79`.
The 2,195-byte `artifact-verification-1.json` receipt has SHA-256
`81eecca6777b57239d65c3781770f57244826a5f9bf20849d2de2b1f8eb3c3b9`.

| Focused command | Terminal test result | Command duration | Log SHA-256 |
| --- | --- | --- | --- |
| Artifact selection | 28 passed, 0 failed, 0 ignored, 2,826 filtered; 658.49s execution | 1,021,724,923,634 ns | `92197843dedb6767b6ee239455489e98cc26f87815c50634518753501d189e89` |
| Complete proposal replay | 1 passed, 0 failed, 0 ignored, 2,853 filtered; 46.63s execution | 46,775,859,341 ns | `71a69545b308820dc4fd762f2bc972c296003f0d60d7d685c26ccc5de2d64c90` |

The artifact probes compare VM, reference, Rust-bootstrap ELF and selfhost-emitted
ELF diagnostics/bytes, retaining independent canonical unit, map and pack oracles.
The complete compiler artifact case reported 646 functions, a 10,806,949-byte
input and a 10,338,816-byte artifact, SHA-256
`41dd8141cb12a9501e69c8092e0b8e5d7c020765967511ec051c50bacac18f23`.
Unit emission covered 650 canonical units with kind tags 5, 6, 9 and 20.
Reported work was 826,834,103 VM steps and 438,157,491 reference expressions;
cumulative modeled allocation was 86,155,453,261 and 97,782,508,693 bytes,
respectively. Maximum call depths were 21/22. These are reported work/allocation
observations, not live retained bytes or RSS. They do not establish C2/C3
compiler identity, native phase reclamation or full-gate acceptance.

Public GC owner `capacity-resume-1030-173` joined through
`capacity-resume-1030-174` with exit 0. From 06:35:00.970Z to 06:35:23.158Z,
physical storage changed from 79 packs / 85,802,127 bytes to three packs /
81,763,516 bytes, reclaiming 4,038,611 bytes. The GC command took
10,106,528,987 ns; export commands took 6,055,138,869 and 5,958,223,363 ns.
Receipt SHA-256:
`947e765ec799733810bca182f26d8ce80be01a557786162ecf0702a834779de3`.
`preparation_saturated` and `canonical_bytes_retained_saturated` were both false.

HEAD and accepted revision were unchanged. Complete transport before/after was
identical: 15,330,787 bytes, SHA-256
`8774b278e0859f0c092af119248fff73ad57d5ca27607a802d1e18e8a3cfc7a1`, digest
`package_transport_6866de46a96c9839de546be697ff2ee8fbcae78b1cb8882c85b07b5161cf85f9`.
The redundant second container was removed; the first remains ignored evidence.
These are physical compaction results, not retirement-tombstone removal. The
focused source-byte inventory above precedes this pack rewrite; semantic/HEAD
preservation does not claim unchanged Git-tracked repository pack files through GC.

A later test-helper cleanup replaced `.chunks_exact(138)` with
`.as_chunks::<138>().0.iter()`, preserving fixed-row contents and ignored-remainder
behavior. The focused artifact result above precedes that correction. The final
format/Clippy preflight joined at `capacity-resume-1030-187` with exit 0, from
06:39:03.851Z to 06:39:39.653Z. Formatting passed in 4,613,727,314 ns; workspace,
all-target, all-feature release Clippy with `--locked -- -D warnings` passed in
31,187,303,762 ns. The 1,237-byte `preflight-2.json` receipt has SHA-256
`0dd326f7895d9650c57740acbadfc2139f1d561b3713a7e5a915658a08a9c4b5`;
Clippy log SHA-256:
`b11a1884c6cc92803bff7bea6f906f5104d91d90aa694fbb4aface6ad11862ef`.
Discovery generation and verification had passed in 876,021,424 / 857,130,798 ns,
without generated-file changes; their log SHA-256 values are
`fb6bf565ee855321f342f95731d37b21e2e77a581eafec9cfd283d2a50c20d43` and
`4b0568d49cfb4d88fd0642bd7a1a2325678325bafa0c49077d77a9d042ff36a4`.
The final frozen-tree gates must exercise the resulting source, including that
helper correction and these documentation changes.

Earlier local structural/arithmetic observations have the narrower scope below:

| Local model | Observed scope |
| --- | --- |
| Admission structure | 76 declarations; balanced forms, available call arities and local bindings |
| Finite estimated policy | 793 cases against an independent BigInt formula |
| Projected-unit layout | 22 small cases |
| Instruction/index checks | 55 cases |
| Diagnostic ordering | 24 cases with controlled hash stubs |
| Emission structure | 60 functions, 857 calls, 1,292 local references |
| Checked output arithmetic | 10 pool, 12 pack-size and 10 object-byte-sum cases |

These scratch models are not the production VM, disjoint reference interpreter,
language type/custody verifier or native runtime. Hash stubs do not establish real
checksum validation. Static comparison preserves selected canonical emission
functions, object ordering and map framing bodies apart from private length
prefixes. Those static models alone establish no canonical output equivalence;
the focused executable oracles above provide their separate observed scope.

The twelve passed Rust producer tests cover literal wire layouts, nonzero unit
origins, wide measurement without output allocation, full-u64 word serialization,
129-entry boundaries, a 180,002-instruction/8,640,128-byte code projection, exact
policy arithmetic and retained unsupported forms. Full-u64 metadata serialization must
not be relabeled as runtime acceptance of bit-63 metadata counts: the selfhost
reader uses a nonnegative I64 domain. Likewise, the oversized code projection does
not prove removal of the independent canonical-unit codec limit.

The reference-map count at metadata offset 264 is carried to the artifact
manifest without driving a local allocation or iteration. Its hash-consistent
small mismatch control targets the strict artifact root-summary check after
emission. It must not inherit the former incidental 100,000-entry rejection as
an admission expectation. Interface-count mutations instead participate in the
exact interface identity and can fail metadata admission. Each actual diagnostic
and earlier-layer commitment must be observed before claiming the control passed.

## First frozen-tree run and retry corrections

The first sealing owner, `capacity-resume-1030-195`, joined through request 231
with exit 1. On frozen tree `13b09cc3e4be24802a6fcbd0433b8a96d45eb33d`, the
literal selfhost child exited 101 after 3,770,985,268,606 ns. Its terminal summary
was 148 passed, one failed, two ignored, zero measured and 2,703 filtered;
test execution took 3,407.83s. The checkpoint completed at
2026-10-30T07:51:42.299Z with terminal status `failed`, preserving that exact
tree and its 2,194-file / 150,424,720-byte frozen inventory. No native subharness,
named complete-compiler controls, full gates or release-target command ran in
this attempt.

| First-run record | Bytes | SHA-256 |
| --- | ---: | --- |
| Literal selfhost raw log | 339,847 | `8ec6f51fcf7f230be4341bd081ca942a0856e1f015fb65a388f8e4674a54213f` |
| Failed checkpoint | 3,947 | `96a6c0958e3d876ab867ccbcc6f528e3f324bc1fcc42bde8cd1e1eaf6fbf969a` |
| Compact failed-seal observation | 3,241 | `ce5358f5c2070f5a65391fc65c79f4a10f7354b976346ab3c5b8ada0978e7e6c` |

The native closure test within the literal harness passed: 294 functions,
18,261 nodes and 2,021,454 normalized-input bytes. Its native compiler and
artifact outputs are bound below. These individual passes remain scoped to the
failed tree; they do not accept the complete harness or the forthcoming repair.

| Observed reproduction value | Bytes | SHA-256 |
| --- | ---: | --- |
| Mode-0 profile-6 native compiler, C2 = C3 | 1,249,170 | `c613d7c604322d42f4cec6bd65d83520828d901fa7f620c792707aeb1caeedb7` |
| Profile-7 artifact input supplied to both C2 and C3 | 10,806,949 | `c50451cd991e04fe64e38f5a7bae5a4d34497cbfd54e614ad2c9c137bde19791` |
| Matching artifact output | 10,338,816 | `41dd8141cb12a9501e69c8092e0b8e5d7c020765967511ec051c50bacac18f23` |
| Capacity-control input | 11,086,603 | `0e2919a816c9db5bb2c8aa17015fea250bf6ed42f90bea93be5e2f88558e1f6e` |
| Capacity-control output | 11,179,173 | `737f18a830ae343e1c7715e840a22edc13507856a8475e7eb68dec745be552b3` |

The matching artifact digest also equals the earlier focused complete-compiler
artifact result. Those byte equalities establish their reported projected-input
reproduction scope; they do not accept the complete tree or establish source-driven
Tier 2 self-hosting.

The sole failed test is a stale generic negative fixture in `selfhost_tests.rs`:
it selects Mode 1 with retired Profile 5 while expecting structural diagnostic 10.
The current-only dispatcher correctly returns profile diagnostic 2 before the
structural check. The applied correction selects Profile 7 for the malformed
layout, recomputes the carrier hash and retains expected diagnostic 10. A separate
hash-consistent Profile-5 mutation expects diagnostic 2. This tests both the
intended structural layer and rejection of the retired selector. The corrected
malformed-input control passed in the focused execution below, comparing VM,
disjoint reference and Rust-bootstrap native compiler C1. Diagnostic 10 is cheap
framing admission before carrier checksum and deep-unit validation; recomputing
the hash keeps the fixture consistent without claiming it passed those later
layers. This control does not execute through selfhost-emitted C2; the earlier
four-way artifact probes retain their separate scope.

A separate caller audit found raw `ownership_steps.saturating_add(...)` updates
in `change/request/literal.rs::charge` and
`change/request/test_retention.rs::charge`. Those former updates lost exactness
without setting `WorkSaturation`; a later scalar comparison could therefore
mistake a clamped observation for an exact finite charge. Prior controls across
all 46 budget dimensions covered the counter API, not these two callers.

The two independently reviewed corrections now use `WorkSaturation::charge` with
`BudgetDimension::ImpactOwnershipSteps`, the unchanged optional caller allowance
and checked charge-amount conversion. Direct helper regressions cover unmetered,
zero, one-short, exact and maximum policies; exact arrival at `u64::MAX`; finite refusal
without mutating work; unmetered overflow with a sticky saturation flag; and finite
refusal after saturation, including a zero increment. They also check empty edit
sets and unchanged repository HEAD/root. Both direct helper regressions passed.
This bounded caller repair does not establish an exhaustive audit of every work
accumulator.

## Applied corrections and focused verification

Request 235 applied the three Rust corrections under the canonical check lease.
Application receipt SHA-256:
`f35a1517fa9ea8c53811c9bb6222de152ccb280a680a40c6227b0e04e21226bd`.
The focused owner `capacity-resume-1030-254` joined through request 257 with
actual exit 0. All seven operations passed between 2026-10-30T09:24:05.902Z and
09:25:46.368Z. The 8,609-byte final focused receipt has SHA-256
`3fff28de686971dda8212a20f5a0458d51de1a2ac7d37a5e9fdd8024c2cbeedb`.

| Focused operation | Terminal result | Command duration | Log SHA-256 |
| --- | --- | --- | --- |
| Formatting | Passed; exit 0 | 4,556,557,865 ns | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| Two direct ownership-charge regressions | 2 passed, 0 failed, 0 ignored, 2,854 filtered; 0.01s | 115,221,581 ns | `5996b27960698eaaeaf225ec90a53f3bee52b8879b189f3b5bbcc57430541d9d` |
| Literal-edit selection | 16 passed, 0 failed, 0 ignored, 2,840 filtered; 0.82s | 922,820,772 ns | `14c4ecee3a5b60bd6f33d5ea41ae6385e7a57215559b030d84bfcdfb78341e23` |
| Existing test-retention control | 1 passed, 0 failed, 0 ignored, 2,855 filtered; 0.30s | 409,832,446 ns | `8881e28ae41964da2a162f3047a028ba207eb77b6a65161ae5c2d5f1fa11c796` |
| Malformed-input/profile control | 1 passed, 0 failed, 0 ignored, 2,855 filtered; 49.35s | 49,470,513,504 ns | `b06c8f8bede95897f0d6fdde2a221212f65ca4be4467447bd08f6298ecfaad29` |
| Formatting check | Passed; exit 0 | 4,567,832,495 ns | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| Strict release Clippy | Passed; exit 0 | 40,169,656,560 ns | `0dc8742b4c56730207ab4b39b6455a00776693aaac936c8b9195d23d2c671120` |

The four test selections contain 20 nonoverlapping tests, all passed with zero
failed, ignored or measured tests. Clippy used `--release --workspace --all-targets
--all-features --locked -- -D warnings`. The initial/final inventory matched:
2,194 files / 150,435,825 bytes, SHA-256
`1aed7e766c19e6e36558d0853964c1878354179a63efb411e09e0f8919dc25de`;
public HEAD remained
`f66f443af7f9e46179c56fc9600113fa72dc65b2284ceb6e4450a83494eccc79`.
This source binding precedes the following documentation update and final freeze.

An earlier focused wrapper joined with exit 1 because its selection assertion
expected four literal-edit tests while all 16 selected tests passed. The corrected
wrapper declared 16 and reran every operation above; that wrapper mismatch was
not a production-test failure. The passing focused result proves these selected
repairs, not acceptance of a new staged tree. Native compiler/artifact identities
remain evidence of first tree `13b09cc3e4be24802a6fcbd0433b8a96d45eb33d` until
reaffirmed on the final repaired tree.

## Pending integration and verification

1. Finish affected maintained-output and source/documentation checks after the
   applied corrections and their passing focused tests. Discovery generation/
   verification previously passed without generated changes. Preserve
   source/graph/physical storage identities separately for each tested tree.
2. Freeze the repaired staged tree and bootstrap its independent verifier against
   that exact source/tree. Run literal `bash tools/selfhost/verify.sh` and both
   named complete compiler measurement/inventory controls. Reaffirm Mode-0
   profile-6 C2 = C3 and Mode-1 profile-7 artifact C1 = C2 = C3 on the adopted
   compiler, binding actual profiles, policy tags, source identities and supplied
   observation exactness. These commands must preserve the frozen tree.
3. Run all 29 full gates fresh with zero reuse and `release target` on the same
   tree, join every owning process, audit compact receipts and cleanup, then
   integrate normally. Individual reproduction tests within the first run,
   focused artifact/replay success and public GC cannot substitute for acceptance
   of the repaired tree.

Historical Profile-5 input/artifact sizes and fixed-point hashes remain attached
to their original tested compiler. They must not be rewritten with the new profile
number or presented as evidence of executing the widened consumer.

## Remaining design and unproven claims

The canonical unit codec's 8 MiB boundary, generic packed codec's 128 MiB
configuration ceiling, canonical u32 semantic indices, unit lifecycle/flow
limits, manifest/cache/query 10-million-unit guards, map overflow values, kernel
object envelopes and storage admission remain independent cuts. The outer storage
domains still admit CompilerUnit objects under a 4 MiB allowance and
CompilationManifest objects under 64 KiB; changing the inner unit codec alone
would leave those enforcers in place. Progressive decoding must cover nested
collections and their recursive allocation/drop lifecycle, not only the outer
unit envelope. Removing an outer projection guard leaves those owners' contracts
in place; recursive value/type traversal and partial-result destruction still need
independent proofs.

This slice does not establish a real >4 GiB carrier, a valid ten-million-unit
closure, repository-wide unbounding, complete E2/E3, source-derived canonical
`A_host = A1 = A2 = A3`, Tier 2 semantic self-hosting or Tier 3 sovereignty.
Projected-input fixed points retain Tier 1 scope. The process-owned native heap
still retains its backing allocations until exit; neither widened framing nor
work counters prove bounded RSS or same-process phase reclamation. No release
archive publication or deployment is part of this decision.
