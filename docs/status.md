# Current status

Updated 2026-10-31 (Asia/Tokyo). Accepted typed meaning is the sole editable authority.
Production bytecode is checked against an independent canonical interpreter.

## Accepted implementation and evidence

Profile 7 is accepted at commit `5e7e8d37831b33a9e44c3e50a0b733cfd049b64d`,
Git tree `06ad6feaa9df96a12f1e1ecab039855bff6072e1`. Its complete staged-tree run
finished on 2026-10-30 at 11:42:23 UTC, before that same tree was committed.
The run used base HEAD `932f197ebcd9c17e2702372300ab73996bb20754`; this is the
recorded precommit source identity, not a claim that its committed tree contains
Profile 7. The owning process subsequently joined with exit zero. Recovery checks
bind the unchanged raw indexed files and committed tree to the terminal receipts.
The acceptance record was integrated in documentation-only commit
`bcf455816c11f99ebb60e76f786a1c2baf1e41e7`, separate from that tested tree.

The accepted implementation follows Phase 1 at `e7a44d289ddd90daf81125a9f37c66fa17cd3c21`
and the list slice at `932f197ebcd9c17e2702372300ab73996bb20754`.
The [capacity ledger](decisions/20261025-capacity-unbounding.md) owns the remaining
boundary inventory. The [Profile-7 decision](decisions/20261030-artifact-profile7.md)
records the format cut, public adoption, earlier focused controls and final seal.

| Acceptance operation on the exact Profile-7 tree | Terminal result | Execution / command time |
| --- | --- | --- |
| Exact staged-tree verifier bootstrap | Passed; source and output digests bound | 197.031s |
| Literal selfhost selection | 149 passed, 0 failed, 2 ignored, 2,705 filtered | 3,414.29s test execution |
| Native ELF selection | 11 passed, 0 failed, 0 ignored, 2,845 filtered | 117.66s test execution |
| Combined selfhost script with input-hash readback | Passed | 3,532.520s |
| Maintained native closure measurement | 1 passed, 0 failed, 0 ignored, 2,855 filtered | 23.30s test execution |
| Complete owner/type/profile inventory | 1 passed, 0 failed, 0 ignored, 2,855 filtered | 13.03s test execution |
| Full gate registry, fresh, four checker workers | 29 passed fresh; 0 reused; 0 unrun | 3,849.927s checker execution |
| Release target policy | Passed | 0.006s |

Full receipt SHA-256: `36ec247b40817f8bbc6f835fb516677ebcff2f1d67c23303600c8ccd2bbe4400`.
Bootstrap receipt: `ec0b1604523b0fcc7d1b3ab9bf14883167e875ab4d8cd9f6b8e11cbd6675fd92`.
Terminal checkpoint: `34865dff49d1f2294090100e105bb327da0ed6a09faa4d10c6c3284ab4d24988`.
The unchanged verifier executable is
`2c798059ce30fdb6e4e42d63061d8ddb4a81efb1bd6eaf3e1c7f924c97929747`.
Bootstrap observed one reused Cargo artifact; that is distinct from the zero reused
full-check gates. No release archive or deployment was published by this seal.

## Accepted capacity and policy capabilities

The 46-dimensional change-policy API defaults to `None` and preserves exact
finite allowances, including zero and `u64::MAX`, without engine clamping.
Store, map, kernel and publication observations carry saturation separately from
permission. Ordinary memory-project storage, compiler reconstruction and compiler
proof admission use optional exact policies with unmetered defaults. The two
literal/test-retention ownership helpers charge through the same exact saturation
owner; that repair does not assert an exhaustive audit of every accumulator.

Compact authoring and logical-plan I/O have no aggregate byte or record defaults.
Transaction and semantic-diff codecs decode sequences progressively without the
former 4 MiB payload or 100,000-edit defaults. Their logical storage domains are
unbounded, while exact explicit read policies and checked pack framing remain.
The 32 MiB pack target is a physical grouping choice. Receipt contract 2 and
LKJABDG2 are current-only; all six maintained repositories were publicly
republished without changing canonical meaning or source inventories.

Complete callable edits preserve retained parameter identities and explicitly
order the final parameter list. Public transactions can delete parameters with
their corresponding caller/body repairs. Normalized queries have no default
output-byte allowance, exact finite fitting, resumable pagination and independent
read telemetry. All four cumulative execution policies preserve exact zero and
finite allowances and independent saturation flags across VM/reference, child
execution and receipts. Discovery emits each scalar once with units and exactness.

Lists have no default one-million-element or height-3 ceiling. Their 32-way radix
and 32-element tail are physical units. Construction, traversal and disposal use
worklists sized from the representable index width. Ordinary raw-value admission
has no separate one-million-item subtotal or visit-fuel cap; exact selected
collection/allocation policies, cancellation, identity and type checks remain.

Native layout and Mode-0 profile 6 remove former function, instruction, image,
local, stack and descriptor defaults through wide checked framing and emission.
The public 4,097-function witness crosses 65,536 instructions and agrees across
native ELF, VM and reference. LKJSG002 and LKJREGN2 profile 2 are current; source
carrier aggregate-byte and graph/map defaults were removed, but source carrier
u32 extents and independent object/depth boundaries remain.

Mode-1 profile 7 widens projected unit, object, metadata and pool fields, checks
complete extents before allocation and retains exact optional u64 estimated-proof
policies. Tag zero bypasses the estimate. Native emission, canonical map/pack
construction, strict loading, independent behavior and malformed controls pass.
This cut removes projection-only ceilings; it does not remove the canonical unit
codec's independent 8 MiB cap or the outer 4 MiB CompilerUnit storage allowance.

## Current artifact-ordering candidate

The stable index merge sort is publicly adopted and has passed focused checks,
complete proposal replay and a pinned complete-compiler artifact comparison.
Fresh full acceptance and mainline integration of this candidate are pending.
The [sorting decision](decisions/20261031-artifact-index-sort.md) owns its algorithm,
source bindings, measurements and evidence limits.

On the identical 650-unit pinned input, VM instructions decreased from 826,835,225
to 739,945,099 and canonical reference expressions from 438,158,052 to 387,022,989.
Observed VM/reference times decreased from 103.671s / 526.016s to
93.557s / 482.161s. These are single-run timings. Modeled cumulative allocations
decreased by 8.26% / 8.30%; allocator traffic, copied bytes, peak live bytes, mapped
capacity and RSS were not measured. All four evaluators emitted the identical
10,338,849-byte artifact before and after; strict loading and 37,712 independent
canonical object comparisons passed. This input is a projected artifact request,
not the raw-source self-compilation required by Milestone A.

## Authoritative compiler closure

The current graph revision is
`rev_71fade8bd535b16925f51aa8c0e00260bf375288febad1654a602f7308f01d83`.
Its public HEAD SHA-256 is
`34b5ed697976321ad1c26b5554ee9fb82911a2389c5398c591c1ae7fb12048bc`;
its semantic state is
`semantic_state_4afafbe96907620b4e04d62319faa3c1253ce8c8ee56a012fd834a084883d7fc`.

The refreshed inventory contains 73 modules, 651 callables (627 pure and 24
external), 36,295 owners, 41 types, 29 structural record types and 29,438
retirements. It covers 30,873 expression owners, 7,768 call expressions,
2,527 bindings, 16 generic application sites and all supplied/uncalled/untaken
source. Parameter modes are 1,980 unrestricted, 182 consume and one borrow.
Complete proposal/body replay agrees with 655 canonical units. Public readback
covered all owners in nine pages. The sort uses existing semantic families and
intrinsics.

The complete source carrier is 12,312,663 bytes, SHA-256
`0f8b486925e1482fbb482204c3952313448635d868dca42156d83682707cbc60`.
Physical storage is four packs / 87,934,112 bytes after public GC. These are
physical storage observations, separate from Git and semantic identities.

The following fixed points belong to the accepted Profile-7 graph before the
sorting change; they are not fixed-point claims for the current candidate.

| Reproduction on the accepted Profile-7 tree | Bytes | SHA-256 |
| --- | ---: | --- |
| Mode-0 profile-6 native compiler, C2 = C3 | 1,249,170 | `c613d7c604322d42f4cec6bd65d83520828d901fa7f620c792707aeb1caeedb7` |
| Mode-1 profile-7 input, policy tag zero, 650 units | 10,806,949 | `c50451cd991e04fe64e38f5a7bae5a4d34497cbfd54e614ad2c9c137bde19791` |
| Strictly loaded artifact, C1 = C2 = C3 | 10,338,816 | `41dd8141cb12a9501e69c8092e0b8e5d7c020765967511ec051c50bacac18f23` |

That Profile-7 source-compiler backend closure exported 624 functions / 36,561
nodes in 3,984,067 normalized-input bytes. Both backend generations emitted the
same 2,366,697-byte source-compiler executable and compiled independent scalar
requests. This establishes Tier 1 reproduction of the source compiler, not
raw-source self-compilation of the maintained compiler.

## Next dependency batches and unproven claims

Complete the artifact-ordering candidate's exact staged-tree acceptance, then
integrate the verified tree. The next semantic batch admits and lowers the
copyable compound source closure, including closed List<I64> substitutions,
structural records, active copyable intrinsics and binary-filter targets.
Its drafts are not accepted compiler capability. Affine ByteBuffer custody and
source-derived canonical artifacts remain explicit dependent batches.

Milestone A requires source-driven compound types and all active expressions,
complete intrinsic and closed generic admission, affine ByteBuffer custody and
borrow checking, binary-filter targets, and both ELF and canonical artifact
emission. The scalar source route currently supports Unit/Bool/I64 and its eight
closed intrinsics. It cannot compile the complete active closure. Refresh the
whole-owner/type inventory after each compiler change and establish capability
before adopting new feature families into maintained compiler source.

Remaining coupled capacity families are canonical unit/manifest/cache storage and
codecs; source carrier/object envelopes; recursive validation, lowering, codecs
and destruction; map key/value scale; catalog and GC topology/accounting; and
package/data/queue/HTTP/tooling policies. Each requires an end-to-end workload and
all its coupled readers, writers and lifetime paths. Increasing a constant or
removing only an outer guard is insufficient.

Milestone C requires separate native phase allocation contexts, sound survivor
promotion and repeated same-process success/failure/cancellation evidence. Native
backing allocations still live until process exit. Milestone B adds the remaining
P_self families: nominal owned records/choices and Match, closures and Invoke,
explicit owned implementation witnesses/read views, and lexical regions.
Complete Tier 2, repository-wide capacity liberation, bounded same-process RSS,
phase reclamation and Tier 3 sovereignty remain unproved. Projected-input fixed
points do not establish source-derived `A_host = A1 = A2 = A3`.
