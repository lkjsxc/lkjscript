# Expandable canonical unit maps

Date: 2026-10-24
Status: Implementation candidate; verification and integration identities are recorded below.

## Decision and scope

Remove the 700-unit artifact admission guard through the complete Tier 1 chain:
Rust projection, profile-5 input admission, lkjscript map construction, generated
object inventory, pack emission, and the unchanged strict artifact loader.
Keep the canonical 64 KiB page bound. Emit a deterministic radix hierarchy whose
aggregate size grows with its binding inventory. The current canonical schema
remains epoch 1; no unit/map/pack/artifact wire encoding changes.

This advances capacity architecture. It does not extend source-driven scalar
E2/E3, derive canonical units from raw source, establish Tier 2 compiler-source
fixed points, or implement native phase reclamation. The maintained compiler
closure, its observed active features and the complete Tier 2 acceptance profile
remain distinct.

## Representation and termination

Unit keys are fixed 17-byte typed identities; values are 65-byte compilation
bindings. Admission establishes strict key order and uniqueness across every
supplied unit. For U entries and common-prefix length p, a leaf's exact encoded
size is `107+p+U*(88-p)`. Compare U with `(65429-p)/(88-p)` before deciding whether
to split. Overflowing ranges partition at the next differing key byte.

A branch carries prefix, exact entry/logical-byte summaries, logical commitment,
absent terminal tag, and sorted child descriptors. Each child descriptor is
82 bytes: edge, physical digest, entry count, logical bytes, tagged content
commitment. A branch occupies `106+p+82*C` bytes and C is at most 256.
Every descent increases the common prefix, so non-tail recursion is bounded by
the 17-byte key width. Child-loop cursors advance across nonempty groups; the
existing affine tail-call mechanism threads two consumed buffers. Returned
records contain ordinary Bytes, so no new owned-record or cleanup capability
is assumed. Failure branches explicitly consume their still-live buffer with
core.buffer.discard before returning. This existing primitive avoids copying
unneeded bytes and preserves exact profile-4 branch-state agreement; it does not
introduce general asymmetric-branch cleanup support. Empty input produces one
leaf. Every nonempty branch has at least two children, yielding at most `2U-1` pages for U>0.

Logical content derivation is retained independently of physical partitioning.
The compiler returns all physical pages in deterministic postorder as internal
u32-length-prefixed frames. The root is last, included once. The artifact
assembler includes every page as a domain-5 object and uses existing canonical
ObjectKey sorting, manifest derivation, pack framing and strict loading.

The generated pool includes framed units, framed pages and the compilation
manifest. Each unit append checks header and payload extent before writing its
u32 length. Aggregate generated offsets are proved representable before object
rows are serialized. The frame parser rejects missing length bytes, zero or
oversize pages, truncated payloads and trailing fragments. Finite extent checks
preserve exact-fit and reject one-over with compiler diagnostic 3. Final pack
counts and offsets remain u64. Arithmetic boundary probes avoid allocating
multi-gigabyte values.

## Allowance and remaining capacity boundaries

Profile 5 remains the current deterministic input contract. Wire zero means
unmetered admission; the host API uses Option<NonZeroU64>. Positive allowances
retain their exact unsigned value and minimum estimate
`4096+128*L+64*(U*U+O)`. This is an optional admission estimate, not a decremented
execution-work counter. Its existing layout proof remains valid without the
700-unit guard. Termination uses finite cursors and increasing key prefixes.

Retained boundaries are reported separately: u32 input/generated-pool offsets,
100,000 source objects and metadata owners, 8 MiB projected/canonical units,
per-table/instruction counts, 128 locals/operands, the independent canonical
compilation manifest's 10,000,000-unit guard, and native 256 MiB per-value policy.
Mode-0 8 MiB/callable/local/fuel domains and LKJSG001 proof/object limits remain.
Kernel validation still defaults/clamps to MAXIMUM_VALIDATION_WORK, and change
budgets still validate maxima against implementation ceilings. This change does
not remove those WS1 paths, recursion limits or historical identity growth.

## Verification obligations

Compare emitted pages byte-for-byte with the independent Rust map builder using
the same 64 KiB leaf target, and compare logical content with the ordinary
smaller-page builder. Cases cover empty, singleton, small, prefix-1 752/753,
prefix-15 896/897, and two 897-entry groups producing an 11-page, three-level
tree. Verify complete reachability and absence of duplicate pages. Repeated
emission checks deterministic order. Remove a child page; separately alter a
parent/child logical-byte summary while retaining the local total, or alter
logical content. Recompute checksums and physical digests before requiring the
exact independent structural diagnostic.

The complete capacity package has 800 units: one entry returning 42 and 799
unused functions with distinct 6,144-byte Text values. Require input >8 MiB,
strictly loaded output with a multi-page unit map, exact source/compiler-unit
sets and bytes, preserved Text associations, and entry execution on VM and
canonical reference. Native C1/C2/C3 consume the same input and emit identical
artifacts. Each stage must reject a hash-rebound malformed unused unit past the
former 700-unit boundary with diagnostic 10. Existing compiler-source behavior
mutation and independent program/trap controls remain required.

## Adoption, observations and acceptance

Accepted predecessor is commit `7d928f74ad1c47bc625cd55fd9675f61e3a21ceb`,
tree `dd549d04af552cdbbe859cae5785d7d121e637fe`. Its selector gates and remote
integration are complete. This capacity candidate is a separate successor.

The first revision-bound public transaction
`transaction_c52bf90c94f31ab6415502404e918c66d03c33472d6edc13b9a35e6f5a5d5311`
created 966 owners, added nine functions and updated four. The subsequent public
correction `transaction_93ae303ff3172eaf29a28852679712b9b3e7c761d188fd432b9ef09b61d97f6b`
created 181 owners, added the existing discard intrinsic declaration and updated
two functions. Both published once with differential equality. Unchanged proposal
functions were omitted; existing declarations and parameters used identity-preserving
upserts. Fine-grained body identity/retirement economy remains separate work.

Current revision: `rev_1c34f3f6a0fd193b5077c8ceaf6c10f078e16416b990e796542790e649910472`.
Semantic state: `semantic_state_50c336e1c41ed4515a9ccb5c9db482c48f84513296d0dd22189e8dbc1e4b8cd5`.
Complete LKJSG001: 8,422,624 bytes, SHA-256
`0e8c5cd9644f5309f9c77cfad80a15c9a944af87c9328687ff194f493857d501`.
The complete inventory has 72 modules, 569 pure functions, 24
externals, 34,550 live owners, 40 supplied types and 12,612 retirements.

Current-revision focused commands share
`cargo test --release --locked --lib FILTER -- --test-threads=1 --nocapture`;
the final measurement explicitly adds `--ignored`.

| Filter | Passed / failed / ignored | Whole command seconds |
| --- | ---: | ---: |
| selfhost_byte_backend_reproduces_its_complete_native_closure | 1 / 0 / 0 | 2110.076 |
| selfhost_artifact_generated_pools_check_exact_extents | 1 / 0 / 0 | 36.795 |
| selfhost_artifact_input_ | 7 / 0 / 0 | 77.708 |
| selfhost_source_profile_inventories_every_owner_and_type | 1 / 0 / 0 | 12.228 |
| selfhost_source_measure_maintained_native_closure | 1 / 0 / 0 | 24.959 |

Mode 0 has 254 functions, 17,033 nodes and 858,136
input bytes. C2=C3 is 2,676,552 ELF bytes, SHA-256
`e8a1d9eb93e29a7aa9cb49d03abc07fae85950e0f8281e5dc9f040a703d901f4`.
Mode 1 has 597 units in 9,323,887 input bytes and emits 9,853,415
artifact bytes. The Tier 1 .lkja outputs produced by native C1, C2 and C3 from
that same Rust-projected input are byte-identical, with SHA-256:
`d7c4aa68b5d12a07f90514d287cede7427e5e75a499edb149cf8970b492b9bfc`.
These equations use Rust-normalized Tier 1 inputs. They do not establish the
specification's raw-source Tier 2 equations.

The separate 800-unit fixture, containing 799 unused Text functions, requires a
hierarchical map and preserves all source/unit identities and bytes. Each native
stage rejects a freshly rebound malformed unused unit beyond the old boundary
with diagnostic 10. Independent map fixtures compare every page with the host
builder and preserve the logical root under different physical partitioning.
The three-level witness has 1,794 entries and 11 pages. Error controls reach
`persistent_map_child_summary` and `persistent_map_page_content` after rehashing,
and reject missing children. Arithmetic probes admit exact u32 extent and refuse
one-over without multi-gigabyte allocation. The new maximum-count case sets both
unit counts to u32::MAX and reaches layout diagnostic 10 before allowance arithmetic.

Physical storage changed from 390 packs / 430,408,188 bytes to 472 packs /
451,334,442 bytes. Carrier size changed from 8,201,410 to 8,422,624 bytes.
No GC or reachability measurement was repeated, and no native requested/live/mapped/RSS
reclamation measurement was made. Work counters and modeled metadata are recorded
separately in [status](../status.md); they do not size storage or authorize quotas.

The final candidate is frozen after documentation and whitespace preparation.
Its exact staged tree is bound externally in the frozen manifest and the
`.artifacts/selfhost/hierarchical-unit-map-acceptance/tree-<candidate-tree>`
receipts. Required publication gates are bootstrap, literal selfhost verification
(128 passes plus two ignored; ten native passes), changed check, all 29 full fresh
gates with zero reuse, and release target. Commit must equal that staged tree;
remote integration must preserve it and pass normal ancestry checks. Targeted
measurements are not a claim that those later gates have already run. Completed
outcomes and final commit/remote identities belong to their terminal receipts.

## Next concrete dependency

Compiler unit growth no longer requires increasing a flat-map constant. The next
bounded source-lowering slice is scalar direct self-tail transfer with verified
native frame reuse. The live audit of elf_x86_64.rs and machine.lkjc found that
mutual tail calls still emit call plus epilogue; general tail-transfer ABI and
non-tail stack safety must remain explicit follow-up. Bytes/structural-record
typing, source-derived type rows and binary-filter signatures then lead into
complete generic-template admission, closed specialization and affine ByteBuffer
cleanup. Source-derived canonical unit construction remains distinct from native
lowering. The next WS1 work-policy cutover must include kernel, repair, witness,
oracle/associated preparation and
actual request/plan identity bindings; changing one top-level default is insufficient.
