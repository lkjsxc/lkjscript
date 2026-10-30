# Current status

Updated 2026-10-30 (Asia/Tokyo). Accepted typed meaning is the sole editable authority.
Production bytecode is checked against an independent canonical interpreter.

The Phase 1 capacity-unbounding cutover is accepted and integrated on `main` at
`e7a44d289ddd90daf81125a9f37c66fa17cd3c21`, tree
`bc733630e4850e1cc50e91607029a24ca520c6e1`. The next list-capacity slice is applied
on `capacity/20261030-unbounded-lists`. At source freeze, its focused runtime,
proof and public-discovery checks have passed; its complete staged-tree seal and
integration remain pending and separate from the accepted Phase 1 evidence.

## Accepted Phase 1 capabilities

All 46 authored change-policy dimensions default to `None`. Explicit finite
policies, including zero and `u64::MAX`, retain exact admission without engine
clamping. Store, map, kernel and publication observations carry saturation separately
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
hierarchical Mode-1 profile 5 unit map remains the artifact producer; profile 7 remains
pending.

Complete callable edits preserve retained parameter identities and can declare the
exact final parameter order after additions and deletions. Public transactions can
delete value parameters alongside caller/body repairs. Normalized query output defaults
to no byte allowance, with exact finite fitting and preserved read telemetry; pagination
is resumable. All four cumulative runtime policies admit exact zero allowances and
carry independent saturation flags through VM/reference, child and receipt observations.
Execution discovery emits each scalar once with its units and linked exactness flag.

The compiler graph contains 35,952 live owners, 41 types, 24,864 retirements and
29 structural record shapes. Maintained proposal replay covers 641 canonical units.
The accepted graph revision is
`rev_e5fa56ece633987cb2f62a3e7cdda4aec58fa74aa9cc7008f71682ca289b2f51`; its semantic
state is `semantic_state_66d01c7280e302fa390e1449e148893c23b0aea46b1fb7c7dc9033cc81a318ec`.
Public readback verified all 2,832 expected namespace entries and 23 changed parameter
orders. These graph facts are distinct from the Rust repository commit.

## Implemented list candidate

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

The applied control inventory includes 22 core list test functions and VM/reference
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
whose separate terminal evidence follows; they do not accept the complete staged tree.

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

## Source-freeze acceptance boundary and remaining work

The list candidate has been applied and formatted under the canonical target lease.
Its application receipt SHA-256 is
`2469dabd5b802ade4363d8990c820b524b7d7b660d77a7f84abfa81b1a315844`.
The map-policy repair has receipt SHA-256
`c2754e5088f8b8d2a58f30715cca255b1fa276e795e038ca10413378543c97d6`.
The supplemental proof/public-discovery repair has receipt SHA-256
`693fcb2710f7de2379ca0fc05341d8e323893cb8ec79fc8bbb8ea89da2bf6dd4`.
At source freeze, only the new complete staged-tree seal and integration remain
pending for this implemented slice. The focused runtime and supplemental observations
do not replace that seal. Before integration, freeze the intended list tree, bootstrap its
independent verifier, run literal `bash tools/selfhost/verify.sh` and the two named
controls, all 29 fresh full gates with zero reuse, and release target. Bind the exact
source/tree and terminal receipts in the acceptance commit and external record,
without editing this frozen status to add the later outcome.

Expression/type/evaluator and value-lifecycle recursion, structural call/operand limits,
map overflow values, wide Mode-1 units, canonical owner/type envelopes, catalog hierarchy
and application data/queue policies remain separate cuts. Physical GC byte observations
and catalog pack/leftover guards remain storage obligations. Source-derived active-profile
E2/E3, full affine custody, canonical artifact emission and native lexical regions remain
on the capability-before-adoption path.

## Unproven claims

Complete repository-wide unbounding, Tier 2 semantic self-hosting, Tier 3 platform
sovereignty, native phase reclamation and bounded same-process RSS remain unproved.
Projected-input backend fixed points do not establish raw-source compiler self-hosting
or `A_host = A1 = A2 = A3`. Proposal files alone never change the authoritative graph.
