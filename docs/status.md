# Current status

Updated 2026-10-30 (Asia/Tokyo). Accepted typed meaning is the sole editable authority.
Production bytecode is checked against an independent canonical interpreter.

The list-capacity slice is accepted and integrated on `main` at
`932f197ebcd9c17e2702372300ab73996bb20754`, tree
`6717d8249c1db319169b2b32be0e088ef09dea21`, following the accepted Phase 1
cutover at `e7a44d289ddd90daf81125a9f37c66fa17cd3c21`, tree
`bc733630e4850e1cc50e91607029a24ca520c6e1`. Profile-7 artifact projection is
applied on `capacity/20261030-artifact-profile7`; its Rust producer controls and
public graph adoption/readback, focused four-way artifact controls, complete
proposal replay and public GC have passed. Native compiler/artifact reproduction
passed within the first frozen-tree run. Its stale fixture and two separately
audited ownership-charge helpers are now corrected; 20 focused tests, formatting
and strict Clippy passed. The repaired tree awaits its full acceptance retry.
Prior accepted receipts do not accept this changed compiler.

## Accepted Phase 1 capabilities

The 46-dimensional change-policy API defaults to `None` and preserves exact finite
allowances, including zero and `u64::MAX`, without engine clamping. Dimension-level
API controls do not prove that every caller uses the exact charging path; the
current ownership-helper audit is recorded below. Store, map, kernel and publication observations carry saturation separately
from permission. Compact authoring and logical-plan I/O have no aggregate byte or
record defaults. Transaction and semantic-diff codecs decode sequences progressively
without the former 4 MiB payload or 100,000-edit defaults. Immutable storage for those
domains is unbounded; exact optional store-read policies and checked pack framing
remain. The 32 MiB pack target is a physical grouping choice.

Receipt contract 2 and LKJABDG2 are current-only. All six maintained repositories
were publicly republished through authenticated code-complete transport, preserving
canonical identities, inventories and semantic-state commitments. Current-only export
reopens each repository, and stale pre-cut HEAD expectations are rejected atomically.
The maintained selfhost proposals were publicly adopted through GraphRepository.

Ordinary memory-project storage, compiler reconstruction and compiler proof admission
use optional exact policies with unmetered defaults and explicit observation exactness.
The 4,097-function fixture publishes and compiles beyond 65,536 instructions; native
ELF, production VM and independent interpreter agree on its result. Wide Rust native
layout, Mode-0 profile 6, LKJSG002 and wide LKJREGN2 profile 2 are accepted. The
hierarchical Mode-1 profile 5 unit map was the artifact producer in that accepted
cut. The subsequent Profile-7 changes and their separate evidence are recorded below.

Complete callable edits preserve retained parameter identities and can declare the
exact final parameter order after additions and deletions. Public transactions can
delete value parameters alongside caller/body repairs. Normalized query output defaults
to no byte allowance, with exact finite fitting and preserved read telemetry; pagination
is resumable. All four cumulative runtime policies admit exact zero allowances and
carry independent saturation flags through VM/reference, child and receipt observations.
Execution discovery emits each scalar once with its units and linked exactness flag.

At Phase 1 acceptance the compiler graph contained 35,952 live owners, 41 types,
24,864 retirements and 29 structural record shapes; proposal replay covered 641
canonical units. That accepted graph revision is
`rev_e5fa56ece633987cb2f62a3e7cdda4aec58fa74aa9cc7008f71682ca289b2f51`; its semantic
state is `semantic_state_66d01c7280e302fa390e1449e148893c23b0aea46b1fb7c7dc9033cc81a318ec`.
Public readback verified all 2,832 expected namespace entries and 23 changed parameter
orders. These graph facts are distinct from the Rust repository commit.

## Accepted list capabilities

The list carrier removes its one-million-element and height-3 ceilings. Logical length
and all extents remain checked; `None` admits unmetered growth, while every explicit
finite allowance retains its exact value. The 32-way radix and 32-element tail remain
physical units. Explicit construction, traversal and disposal worklists derive their
capacity from `ceil(usize::BITS / 5)`, covering every representable list index without
a separate logical height policy or recursive radix construction.

VM and reference construction pass optional policies unchanged. Ordinary raw-value
admission no longer applies a separate one-million-item subtotal or visit-fuel limit.
Exact invocation/shared collection and allocation policies remain authoritative.
Cancellation, full type/identity/authority admission, atomic refusal and alias preservation
remain requirements. Existing 256-depth, map, owned-storage, schema and codec boundaries
are not removed by this slice.

List work carries an aggregate `saturated` flag. Public `production-list-saturated`
and `reference-list-saturated` fields are linked to the corresponding numeric metrics.
Saturated observations supply lower bounds; they cannot grant capacity, size storage,
establish an index or prove a work upper bound. Live and stored pure-tail matrix,
mapping, boundary and cancellation proofs require complete typed, unsaturated `ListWork`
before comparing work with independent bounds. The execution-observation descriptor
inventory is 72 distinct records, with 38 selected by pure-tail; the public capability
control checks all 18 numeric list links and both Boolean flags. Supplemental negative
controls cover serialized missing, malformed and true saturation flags.

The accepted control inventory includes 22 core list test functions and VM/reference
controls using 1,048,609 elements. These cover exact optional policies, growth across
both former boundaries, ordering, aliases, cancellation, reservation failure, a wrong
final element and representability/saturation behavior. The large-list VM/reference
positive and wrong-final-type controls have passed in a focused selection. A corrected
map/list admission control covers both 999- and 1,000-item children under unmetered and
zero/one-short/exact/maximum policies, using each evaluator's own measured cumulative
baseline; depth, cancellation and deep-disposal checks remain. Supplemental stored-proof
and public-discovery controls have passed in their separate scope. The observed host
is x86_64; 32-bit index coverage is a source/model argument, with no 32-bit execution claim. The
[list decision](decisions/20261030-unbounded-lists.md) records the proof and scope.

## Observed Phase 1 acceptance

The seal used precommit HEAD `dad67f97c6b9b008300d59d83e3e4e7540acfa4f` and accepted
tree `bc733630e4850e1cc50e91607029a24ca520c6e1`. The seal driver, collector and
independent audit each joined with exit 0. Normal fast-forward integration and GitHub
readback confirmed commit `e7a44d289ddd90daf81125a9f37c66fa17cd3c21` with that exact tree.

| Required check | Observed outcome |
| --- | --- |
| Literal selfhost script | 138 passed, 0 failed, 2 ignored; 3,379.49s |
| Native filter in that script | 11 passed, 0 failed, 0 ignored; 118.25s |
| Maintained native closure and whole-owner/type inventory | Each separately passed; 22.64s and 13.01s |
| Full `--fresh --machine --jobs 4` check | All 29 gates passed fresh, zero reuse, zero unrun, stable inputs; 3,790,577,053,911 ns |
| Public CLI harness within full verification | 350 passed, 0 failed, 1 ignored, 1 filtered; 366.03s |
| Release target | Passed for `x86_64-unknown-linux-musl`, static-musl; target policy check |

All eight repaired regressions were explicitly observed passing in the full run.
The full receipt SHA-256 is
`668c3548809c8abf9a063f017a6a0ed9a618754bcbb91749edc6065611955503`.
The terminal integration record SHA-256 is
`fcde1e7da229ffec013952191454a306317e250c6181692a9d33669190b4504b`.
No release archive publication or deployment is implied by the target policy check.

Tier 1 reproduction reaffirmed Mode-0 profile-6 C2 = C3: 1,246,043 bytes, SHA-256
`342ec8bcf60d548225d5818c3d51677d08f5fe987d59c601307c17be82202ed6`.
Mode-1 profile-5 artifact C1 = C2 = C3: 10,292,518 bytes, SHA-256
`a441d5740bf81e9b292e0963ae9cae9917cc61473ac12b52eed3f768e74d598c`.
These equations use the disclosed Rust bootstrap and normalized/projected inputs.

Public GC preserved all six maintained HEADs and complete source transports while
reclaiming 419,417,250 physical bytes. Five maintained-asset comparisons matched without
replacement. Package checks passed 116 standard, 44 application and 176 package-plan
tests with zero failures and equal differential results. The [capacity ledger](decisions/20261025-capacity-unbounding.md),
[publication evidence](decisions/20261027-policy-publication-evidence.json) and
[source adoption evidence](decisions/20261028-selfhost-adoption-evidence.json) retain the
scoped identities, observations and earlier verification history.

## Observed list focused scope

The corrected core/admission/map-policy selection joined with exit 0 and Cargo exit 0:
108 passed, zero failures or ignored tests and 2,735 filtered; test execution took
7.43s and the driver took 376,388,950,567 ns. The 38,365-byte log SHA-256 is
`4cf89958aec4f7fa0adff9753f60bc0ef6bb150699a514d2dba677826c9caef5`.
These results apply to the candidate with 22 changed source paths, including the
map-policy correction. They precede the later stored-proof/public-discovery repair,
whose separate terminal evidence follows. These focused observations alone do not
accept the complete staged tree; its later full seal is recorded below.

Clippy passed for the supplemental repaired candidate with 23 changed source paths
in 39.51s. Its log SHA-256 is
`5c15f77215c5055fc1689d6658aff0f7baaacd88bf5db85deab3b89c7ce8d20d`.
The supplemental driver joined with exit 0 and binds all 23 source hashes in its
11,970-byte receipt, SHA-256
`3045f15a8b44023fab176458ea6ecc2be2baacc14f9e09777152a0f348603d78`.
Three library proof controls passed with zero failures or ignored tests, 352 filtered
and 0.00s test execution; the public capability control passed with zero failures or
ignored tests, 351 filtered and 0.10s test execution. Only the same-source Clippy
observation was explicitly reused. Pure-tail passed with 38 discovered fields and
complete cleanup; its receipt SHA-256 is
`9e08a4a7907586468036188c1b67fb06816f0d5efd65899f4f66d008c9d746ac`.
All eight discovery-owned documents were publicly regenerated and verified.

## Observed list acceptance and normal integration

The final list tree was sealed on precommit HEAD
`e7a44d289ddd90daf81125a9f37c66fa17cd3c21`, tree
`6717d8249c1db319169b2b32be0e088ef09dea21`. The complete sealing owner, postrun
collector and independent audit each joined with exit 0. All raw bytes and modes
remained stable across 35 changed paths, 2,193 indexed files and 133,847,296 bytes.

| Required check | Observed outcome |
| --- | --- |
| Literal selfhost script | 138 passed, 0 failed, 2 ignored, 2,703 filtered; 3,367.50s |
| Native filter in that script | 11 passed, 0 failed, 0 ignored, 2,832 filtered; 119.22s |
| Maintained native closure | 1 passed, 0 failed, 0 ignored, 2,842 filtered; 22.78s |
| Whole-owner/type inventory | 1 passed, 0 failed, 0 ignored, 2,842 filtered; 12.92s |
| Full `--fresh --machine --jobs 4` | All 29 gates passed fresh, zero reuse or unrun, stable inputs; 3,797,629,862,566 ns |
| Release target | Passed static-musl target policy; no archive publication or deployment |

The full compact receipt SHA-256 is
`4c701f56772b537dc3bcbfac758861e17c4b46c609d5e06df342cc7747a081b8`.
The terminal integration record SHA-256 is
`9a705b8413d125eb584d74e92bae285fb46d0cec41be94164c70b7ab75ede10b`.
Normal fast-forward delivery and independent Git/GitHub readback confirmed main
commit `932f197ebcd9c17e2702372300ab73996bb20754` and the exact accepted tree.
Gate freshness does not claim that Cargo rebuilt every unit. Exact workspace
harness test totals were not captured before cleanup and are not inferred.

The bootstrap artifact directory retained only its receipt, checker transient
storage was absent, and the repository-artifact scan preserved exactly the three
historical ELFs with no new executable retained. OS temporary files were not
inventoried by that collector. The list seal reaffirmed the Mode-0 profile-6 and
Mode-1 profile-5 Tier 1 equations above; it supplies no Profile-7 fixed point.
Evidence is under `.artifacts/capacity-unbounding/seal-list-20261030/`.

Initial list application, map-policy repair and supplemental proof/discovery
application remain separately bound by receipts
`2469dabd5b802ade4363d8990c820b524b7d7b660d77a7f84abfa81b1a315844`,
`c2754e5088f8b8d2a58f30715cca255b1fa276e795e038ca10413378543c97d6` and
`693fcb2710f7de2379ca0fc05341d8e323893cb8ec79fc8bbb8ea89da2bf6dd4`.
Their focused scope remains distinct from the later complete seal.

## Profile-7 implementation and observed progress

Mode 1 now has coordinated 64-bit private projection/pool framing and explicit
`None` versus exact finite u64 policy. The producer and selfhost modules remove
projection-only 4 GiB, 8 MiB, 100,000-entry, 32,768-table/instruction and
128-parameter/local/operand guards. Finite estimated proof admission precedes
hashing/deep validation; output measurement precedes its fallible reservation.
Canonical unit/packed codecs, semantic u32 indices and manifest/cache/query
limits remain independent. The artifact probes compare VM, reference,
Rust-bootstrap ELF and selfhost-emitted ELF; their focused selection passed.

The producer owner joined with exit 0: 12 passed, zero failed or ignored,
2,842 filtered, 0.01s execution; focused compile/test operation 366,873,597,353 ns.
Receipt SHA-256:
`abab2fe578d207b57d3606296619e268a672700d4df163f0310504cf97e2c0ee`.
Cargo selected an 80,050,464-byte product with `fresh: false`, SHA-256
`c25a6d576f62bfbe273ea26f1985bd5feaf2d5e45933fd51791c10c2b10ff6d2`.

Public adoption accepted nine modules and 69 changed members, with zero explicit
delete operations, through a 79,847-byte transaction, SHA-256
`c3f19cd392712801b4c011662f183e8cce614923851c974c2688fb3e1b7593f4`.
It reported 4,626 created and 60 updated identities and equal authoring differential
results. It selected zero runtime tests. Current accepted graph revision:
`rev_b14f1d9465fd9d67cad9cc5946607a7028d2a2d26a2a2883ad953422f23a1b05`;
public HEAD SHA-256:
`f66f443af7f9e46179c56fc9600113fa72dc65b2284ceb6e4450a83494eccc79`.
Public readback covered 36,080 owners in nine pages, with zero changed parameter-order
signatures; the readback receipt SHA-256 is
`5155d7b8f5f991a15c8971c9c3daeb2086f6b12aec3a44ccb765013d92332e8e`.
Complete proposal/body replay passed in its separate focused check. The unit
emission controls observed 650 canonical units; current type and retirement counts
are not inferred from the preceding graph.

The focused owner joined with exit 0: the artifact selection passed 28 tests,
zero failed or ignored, 2,826 filtered, in 658.49s; the complete proposal replay
passed one test, zero failed or ignored, 2,853 filtered, in 46.63s. Receipt SHA-256:
`81eecca6777b57239d65c3781770f57244826a5f9bf20849d2de2b1f8eb3c3b9`.
Initial/final source inventory matched at 2,269 files / 154,423,333 bytes, digest
`ea226ed609fadb3f8476e3dafdb166a0e3552d97d400ea8362599d2260f5ef21`, with unchanged
public HEAD. The complete compiler artifact case reported 646 functions,
10,806,949 input bytes and a 10,338,816-byte artifact, SHA-256
`41dd8141cb12a9501e69c8092e0b8e5d7c020765967511ec051c50bacac18f23`.
Reported VM/reference work was 826,834,103 steps / 438,157,491 expressions,
cumulative modeled allocation 86,155,453,261 / 97,782,508,693 bytes, and maximum
call depth 21/22. These observations are not native RSS or compiler fixed points.

Public GC passed, reducing 79 packs / 85,802,127 bytes to three packs /
81,763,516 bytes and reclaiming 4,038,611 physical bytes. Receipt SHA-256:
`947e765ec799733810bca182f26d8ce80be01a557786162ecf0702a834779de3`.
Both preparation and canonical-retained-byte saturation flags were false. HEAD
and graph revision were unchanged, and complete before/after transport matched:
15,330,787 bytes, SHA-256
`8774b278e0859f0c092af119248fff73ad57d5ca27607a802d1e18e8a3cfc7a1`, digest
`package_transport_6866de46a96c9839de546be697ff2ee8fbcae78b1cb8882c85b07b5161cf85f9`.
This reclaimed physical packs; it did not remove retirement tombstones. The
focused source inventory precedes the physical pack rewrite.

A later equivalent 138-byte-row iterator cleanup followed the focused artifact
run. Final formatting and workspace/all-target/all-feature release Clippy passed;
preflight receipt SHA-256
`0dd326f7895d9650c57740acbadfc2139f1d561b3713a7e5a915658a08a9c4b5`.
Discovery generation and verification passed without generated-file changes.

On first-seal tree `13b09cc3e4be24802a6fcbd0433b8a96d45eb33d`, the native closure
control passed with 294 functions, 18,261 nodes and 2,021,454 normalized-input
bytes. Mode-0 profile-6 C2 = C3 was 1,249,170 bytes, SHA-256
`c613d7c604322d42f4cec6bd65d83520828d901fa7f620c792707aeb1caeedb7`; both native
stages consumed the same 10,806,949-byte Profile-7 artifact input and emitted the
10,338,816-byte artifact with digest `41dd8141cb12a9501e69c8092e0b8e5d7c020765967511ec051c50bacac18f23`.
These native values are observations of the earlier tree. The corrected source
and documentation require their own full acceptance. The applied hash-consistent
control separates current Profile-7 framing diagnostic 10 from retired
Profile-5 selector diagnostic 2 and passed in focused execution.

The literal-edit and test-retention helpers now charge ownership work through
exact `WorkSaturation::charge` admission. Earlier dimension-level API controls
did not cover those callers. All three Rust corrections were applied under the
canonical lease, with application receipt SHA-256
`f35a1517fa9ea8c53811c9bb6222de152ccb280a680a40c6227b0e04e21226bd`.
The focused owner joined with exit 0: two direct charge regressions, 16 literal-edit
tests, one retention control and one malformed-input control all passed, with
zero failures or ignored tests. Formatting/check and strict release Clippy passed.
Focused receipt SHA-256:
`3fff28de686971dda8212a20f5a0458d51de1a2ac7d37a5e9fdd8024c2cbeedb`.
Its initial/final source inventory matched at 2,194 files / 150,435,825 bytes,
SHA-256 `1aed7e766c19e6e36558d0853964c1878354179a63efb411e09e0f8919dc25de`,
with unchanged public HEAD. The native fixed-point values above remain bound to
the first tree; these focused repairs do not establish a new full seal.

Freeze this corrected source and documentation. Run its exact bootstrap, literal
selfhost harness, both complete compiler controls and native reproduction
checks, then all 29 fresh full gates and release target before normal integration.
Those checks must preserve the repaired source and documentation. The
[Profile-7 decision](decisions/20261030-artifact-profile7.md) separates implemented
code, public adoption, observed reproduction and outstanding acceptance.

Expression/type/evaluator and value-lifecycle recursion, structural call/operand
limits, map overflow values, canonical unit/owner/type envelopes, catalog hierarchy
and application data/queue policies remain separate cuts. The outer 4 MiB
CompilerUnit and 64 KiB CompilationManifest storage allowances and nested recursive
decode/drop obligations are part of the next canonical-unit audit. Physical GC observations
and catalog pack/leftover guards remain storage obligations. Source-derived
active-profile E2/E3, full affine custody, canonical artifact emission and native
lexical regions remain on the capability-before-adoption path.

## Unproven claims

Complete repository-wide unbounding, Tier 2 semantic self-hosting, Tier 3 platform
sovereignty, native phase reclamation and bounded same-process RSS remain unproved.
Projected-input backend fixed points do not establish raw-source compiler self-hosting
or `A_host = A1 = A2 = A3`. Proposal files alone never change the authoritative graph.
