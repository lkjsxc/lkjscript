# Current status

Updated 2026-10-24 (UTC). Accepted typed meaning is the sole editable authority.
Production bytecode is checked against an independent canonical interpreter.
Elapsed time is an observation; complete verification has no three-minute requirement.

## Implemented capability

The Tier 1 lkjscript backend now emits an expandable canonical unit-map hierarchy.
The 700-unit guard is removed from Rust projection and profile-5 admission. A leaf
retains the 65,536-byte physical bound; larger inventories produce deterministic
radix branches and include every child page in the artifact object inventory and
pack. Generated-pool offsets and frame lengths remain checked. Capacity failure
paths explicitly consume their ByteBuffer with the already supported discard
intrinsic before returning diagnostic 3. Exact profile-4 custody checks remain.

The [hierarchy decision](decisions/20261024-hierarchical-unit-map.md) and
[artifact input contract](spec/selfhosting-artifact-input.md) own the representation,
termination, allowance and remaining-boundary details. The canonical schema stays
at epoch 1. No canonical unit, map, pack or artifact wire encoding changes; the
changed compiler graph binds new identities through public transactions.

The previously integrated source selector accepts LKJSRQ01/profile 1: an explicit
canonical Target key followed by the complete LKJSG001 carrier. Restricted scalar
E2/E3 validates all supplied owners and types, including unused/untaken definitions
and unselected targets, then selects a pure `() -> I64` command port. Its admitted
operations and eight scalar intrinsics remain unchanged. Source-driven Bytes/record
command ports and maintained-compiler raw-source self-compilation remain outstanding.
The [self-hosting specification](spec/selfhosting.md) separates the maintained
closure, observed active features and complete Tier 2 acceptance profile.

## Accepted predecessor and current canonical source

Accepted predecessor main is
`7d928f74ad1c47bc625cd55fd9675f61e3a21ceb`, tree
`dd549d04af552cdbbe859cae5785d7d121e637fe`. Selector acceptance completed:
127 selfhost passes plus two ignored tests, ten native passes, all 29 fresh full
gates with zero reuse, and release target. The supplemental changed check also
passed all 29 gates freshly. Remote main was independently confirmed at that commit.

The current hierarchy candidate is based on that commit. Its authoritative project
revision is `rev_1c34f3f6a0fd193b5077c8ceaf6c10f078e16416b990e796542790e649910472`; semantic state is
`semantic_state_50c336e1c41ed4515a9ccb5c9db482c48f84513296d0dd22189e8dbc1e4b8cd5`. Public adoption and the explicit-disposal correction each published
once with differential equality. No canonical object was written by a draft helper.

| Observation | Accepted selector predecessor | Current hierarchy candidate |
| --- | ---: | ---: |
| Modules / pure functions / externals | 72 / 560 / 23 | 72 / 569 / 24 |
| Live owners / supplied types / retirements | 34,006 / 35 / 12,009 | 34,550 / 40 / 12,612 |
| Complete LKJSG001 bytes | 8,201,410 | 8,422,624 |
| Mode-1 input bytes / units | 9,177,461 / 587 | 9,323,887 / 597 |
| Emitted .lkja bytes / units | 9,699,624 / 587 | 9,853,415 / 597 |
| Direct local full-kernel work | 3,130,736 | 3,179,744 |
| Composed callable work / modeled metadata bytes | 8,105,847 / 11,531,962 | 8,258,577 / 11,725,696 |
| Physical packs / bytes | 390 / 430,408,188 | 472 / 451,334,442 |

Complete current carrier SHA-256:
`0e8c5cd9644f5309f9c77cfad80a15c9a944af87c9328687ff194f493857d501`.
These are different maintained closures, not a pinned-workload speed comparison.
Physical pack growth, live canonical owners, retirements and carrier size remain
separate observations. No GC, history reseed or identity-format cutover occurred.

The current whole-inventory profile records 163 Text expressions, 91 List
expressions, 28 structural-record descriptors, 14 generic
application sites, 1 borrow parameter and 163 consume parameters.
Its call graph has 172 recursive components and 769 syntactic
tail-call contexts. These observations do not certify inferred custody, borrow
lifetimes, source-profile permission or native phase reclamation.

## Actual focused verification

The joined current-revision checks use
`cargo test --release --locked --lib FILTER -- --test-threads=1 --nocapture`.
The maintained-native measurement additionally uses `--ignored`.

| Filter | Passed / failed / ignored | Whole command seconds |
| --- | ---: | ---: |
| selfhost_byte_backend_reproduces_its_complete_native_closure | 1 / 0 / 0 | 2110.076 |
| selfhost_artifact_generated_pools_check_exact_extents | 1 / 0 / 0 | 36.795 |
| selfhost_artifact_input_ | 7 / 0 / 0 | 77.708 |
| selfhost_source_profile_inventories_every_owner_and_type | 1 / 0 / 0 | 12.228 |
| selfhost_source_measure_maintained_native_closure | 1 / 0 / 0 | 24.959 |

The native backend closure has 254 functions and 17,033
nodes; its 858,136-byte mode-0 input emits 2,676,552-byte C2/C3 executables.
Their SHA-256 is `e8a1d9eb93e29a7aa9cb49d03abc07fae85950e0f8281e5dc9f040a703d901f4`.
For the same Rust-projected mode-1 input, the Tier 1 .lkja outputs produced by
native stages C1, C2 and C3 are byte-identical, with SHA-256 `d7c4aa68b5d12a07f90514d287cede7427e5e75a499edb149cf8970b492b9bfc`.

The independent capacity fixture supplies 800 units, including 799 unused
functions with distinct 6,144-byte Text values. It exceeds 8 MiB, requires multiple
map pages, preserves exact source/unit objects and Text associations, and evaluates
to 42 in VM and canonical reference. C1/C2/C3 artifact equality and freshly rebound
invalid unused-unit rejection beyond index 700 are checked. Map controls cover
empty/singleton inputs, 752/753 and 896/897 leaf boundaries, an 11-page three-level
tree, exact independent page bytes and logical commitments, missing children and
rebound summary/content corruption. Exact-fit, one-over, malformed frame and
maximum-count controls retain their intended diagnostics.

The combined source frontend/backend closure has 577 functions,
35,132 nodes and 35 normalized types. Backend C1/C2 reproduce
identical source-compiler executables. Their three native source-compiler controls
emit the same scalar program printing 42 and reject a hash-consistent unused Bytes
descriptor with diagnostic 13. Existing compiler-source behavioral mutation and
independent value/trap controls remain active.

These are Tier 1 normalized-input fixed points and restricted scalar source
controls. They do not establish maintained-source Tier 2 C2=C3 or source-derived
canonical A1=A2=A3. Native requested allocation, live retained bytes, mapped arena
capacity and RSS were not measured. Explicit discard ends affine custody; native
mappings still remain process-owned until exit.

## Exact-tree publication boundary

Focused receipts and whole-inventory/native measurements are under
`.artifacts/selfhost/hierarchical-unit-map`. They bind the revision and carrier
above. The final documentation/formatting preparation does not change that graph.

Publication requires the frozen staged tree to pass `tools/bootstrap-verifier.sh`
with expected base/tree, literal `tools/selfhost/verify.sh` (128 selfhost passes,
two ignored, ten native passes), `check changed --machine`, all 29 fresh
`check full --fresh --machine --jobs 4` gates with zero reuse, and `release target`.
The exact tree, completed gate outcomes and later commit/remote identities are
bound by the frozen manifest, acceptance and publication receipts under
`.artifacts/selfhost/hierarchical-unit-map-acceptance/tree-<candidate-tree>`.
Git owns the final commit identity. This file records precommit measurements;
it does not substitute targeted checks for the exact-tree publication receipts.
One checkout and the single coordinated Cargo target remain mandatory.

## Remaining boundaries and next dependency

The hierarchy retains a per-page physical invariant, not an aggregate 700-unit
quota. Other bounds remain explicit: u32 input/generated-pool offsets, 100,000
source/metadata owners, 8 MiB per-unit limits, per-table/instruction constraints,
128 locals/operands, the separate canonical manifest's 10,000,000-unit guard and
native 256 MiB per-value policy. Mode 0 retains its 8 MiB input, 1,024 callable
rows and signed-31-bit fuel domain. LKJSG001 retains its directory/object bounds
and positive proof domain `L <= 16,777,183 - 8*N`.

WS1 remains incomplete. The kernel's default/clamped 10M work policy, oracle
16M aggregate-work adapter, associated preparation, change/authoring/inspection
quotas and recursive depth consumers need coherent follow-up. Exact finite quotas
must remain distinct from passive saturating telemetry; explicit zero must not
become unmetered. Existing literal-only identity preservation does not eliminate
whole-body retirement or historical storage growth.

The next bounded source-lowering slice is direct self-tail transfer for admitted
scalar functions, with argument-order and bounded-frame controls. Both native
emitters currently reuse frames only for direct self calls; mutual tail calls
still use an ordinary call followed by an epilogue. General tail-transfer ABI
support and non-tail stack safety remain separate obligations. Bytes and structural-record
typing and binary-filter target signatures then require source-derived
type rows, followed by complete generic-template admission, closed specialization
and affine ByteBuffer branch/borrow/cleanup analysis. Source-derived canonical
compilation units remain distinct from native lowering.
Full Tier 2 also requires nominal owned families, closures/invoke, explicit
witnesses, associated types, source-tied ReadViews, native lexical Regions,
independent full-profile controls and measured same-process phase reclamation.
