# Unbounded persistent lists and ordinary value admission

Updated 2026-10-30 (Asia/Tokyo).
Status at source freeze: core/admission/map-policy focused verification and
supplemental stored-proof/discovery verification passed; the complete staged-tree
seal and integration remain pending.
Accepted parent: `e7a44d289ddd90daf81125a9f37c66fa17cd3c21`, tree
`bc733630e4850e1cc50e91607029a24ca520c6e1`.
Working branch: `capacity/20261030-unbounded-lists`.

## Decision and implemented changes

Keep the immutable 32-way list radix and its tail of at most 32 elements. Remove the
one-million-element ceiling and height-3 admission guard. The carrier represents a
flat logical sequence; tree shape and sharing remain private execution details.
No persisted graph, type, artifact or data encoding changes with this runtime cut.
GraphRepository remains the sole editable meaning authority.

`List::append` and `List::from_items` accept an optional u64 length policy. `None`
selects unmetered logical growth. `Some(0)` permits only an empty list; every other
finite value, including `Some(u64::MAX)`, retains that exact allowance. The common
length check validates u64 representation and the selected policy before any new
list storage reservation. Append also checks `usize` addition. No largest-value
sentinel, default clamp or implicit replacement allowance remains in this path.

The VM and independent reference path propagate `maximum_collection_items` unchanged.
Bulk checked-value wrappers check logical length before classifying children or
reserving the conversion buffer. Physical node and element reservations continue
through the evaluator's separate exact cumulative allocation/collection ledgers.
The length check is not a replacement for those cumulative ledgers: a new leaf or
branch reserves all 32 slots, including unused capacity, and previous work is not
refunded after a later failure.

Ordinary raw list construction selects `None`. The unused raw-construction byte
subtotal was removed. VM/reference ordinary raw-value admission no longer has a
separate one-million-item subtotal or outer single-container/external-value ceiling.
The preliminary view-escape inspection also no longer uses a visit counter as fuel.
Its cancellation, authority checks, existing nesting guard and nominal ancestor
handling remain. All actual occurrences are still validated; sharing is not an
admission certificate. Exact invocation and shared policies remain authoritative.

This cut also changes the list result construction in the raw `map.entries` helpers
to the optional list API. Remaining map construction/insertion policies are unchanged.
Checked allocation extents and exact reservation totals remain distinct from passive
observations; no saturated count is used to size a buffer or prove an index.

## Structural depth and termination

Let `W = usize::BITS`, let radix width `b = log2(32) = 5`, and define
`INDEX_LEVELS = ceil(W / b)`. Every radix descent consumes five index bits. The leaf
uses one level, so a reachable root needs at most `INDEX_LEVELS - 1` branch layers.
The implementation uses explicit inline worklists derived from this representation
bound for insertion, bottom-up bulk construction, bidirectional iteration and disposal.
There is no recursive radix construction and no independently selected height quota.

For logical length `L > 0`, the completed prefix has
`P = floor((L - 1) / 32)` leaves and the tail has between one and 32 elements. The
prefix root height is the least `H` with `P <= 32^H`. Since the count and index fit
`usize`, every reachable leaf capacity and child coordinate fits its checked machine
representation. On a 64-bit host the root can reach 12 branch layers. Its leaf
capacity fits `usize` even though a hypothetical completely full value capacity at
that level would not; the latter is neither needed nor used as an allocation size.
The observed execution host is x86_64. For 32-bit indices, the same source/model
argument gives seven worklist levels and at most six branch layers; no 32-bit
compilation or execution result is claimed.

These worklist sizes are a proof about every representable index, not a resource
policy. They also preserve allocation-free iterator/disposal bookkeeping and the
existing exact node-reservation schedule. Input value nesting is a different graph
property and is not bounded by the list radix calculation.

Insertion walks one finite radix path and settles the copied frames in reverse.
Bulk construction settles each branch after its finite children; remaining leaf
counts decrease before entering a child. Iterators advance finite child cursors.
Disposal removes each uniquely owned node and adds at most 31 pending siblings per
branch level, in addition to the two roots. Shared subtrees are released through
ordinary immutable handles. None of these algorithms uses an execution-fuel counter
to establish termination.

Length and shallow clone remain O(1); indexing is O(log32(N + 1)); ordered traversal
is O(N). Append copies at most one bounded tail and its changed branch spine, without
copying old payloads or traversing a retained prefix. Logical equality, ordering,
typed bytes and canonical commitments do not depend on shape or reference counts.
Reservation refusal or cancellation installs no partial result and preserves old
aliases. Rejected raw payloads retain iterative destruction through their existing
owning wrappers.

## Passive work and public discovery

List work now has an aggregate `saturated: bool`. Each numeric observation saturates
without changing admission or execution; overflow sets the flag. Child joins preserve
incoming saturation, observation intervals retain uncertainty, and worker scopes
restore their caller's complete work state on exit or unwind. `false` means the
recorded counters are exact; `true` marks the record as containing lower-bound
observations, including intervals beginning after a counter already saturated.

Public `production-list-saturated` and `reference-list-saturated` Boolean fields are
linked from all 18 production/reference numeric list metrics through
`saturation-field`. The pure-tail checker requires unsaturated list observations
before using numeric inequalities to establish work bounds. Saturation never grants
finite capacity, changes a buffer extent, establishes an index or proves an upper
bound merely because a reported number is small.

The supplemental checker repair applies this requirement to live and stored matrix,
mapping, boundary and cancellation proofs. It decodes complete typed `ListWork`
records and requires unsaturated observations before testing bounds. A stored result
cannot bypass that check by omitting the saturation field, providing a malformed
value or setting it to true. Implemented serialized negative fixtures exercise each
of those failures. Three active library proof controls passed in the separately
recorded supplemental verification below.

The execution-observation descriptor inventory has 72 distinct records. Pure-tail
selects 38 fields, including both list flags; the public capability control checks
all 18 numeric list links and both correctly typed Boolean flag records. These are
implemented inventory/control expectations, not a new terminal discovery proof.

List discovery now states `maximum-logical-length=unbounded`,
`branch-height=derived-from-checked-usize-length-and-32-way-radix`, and
`length-policy=none-or-exact-u64-before-reservation`. Ordinary value discovery states
`aggregate-items=unbounded-or-exact-caller-policy`. These describe the implemented
list/ordinary admission path; other owners retain their separately disclosed limits.

## Implemented verification controls

The focused inventory contains 22 core list test functions. Controls cover exact
`None`/zero/exact/one-short/maximum policies before reservation; the independent
allocation schedule; retained-root and payload sharing; ordering across tail and
root growth; forward, reverse and mixed iteration; every selected reservation and
cancellation failure; bounded-stack destruction of rejected deep raw payloads; and
passive saturation with finite refusal unchanged.

A bulk fixture crosses both former boundaries and the next radix transition.
Separate compact shared-node and synthetic-overflow fixtures exercise the highest
representable branch depth and checked length overflow without attempting an
impossible contiguous allocation. These structural fixtures do not claim that a
host physically materialized every element of a machine-sized sequence.

The VM/reference control uses 1,048,609 ordinary I64 elements. Its implemented matrix
covers unmetered execution, exact measured finite collection admission, one-short and
zero refusal, the largest finite u64 allowance, and a wrong final Bool that must reach
the type-admission diagnostic after all preceding elements. Existing 255/256 nesting
controls continue to delimit the separate pending lifecycle work.

The corrected map/list control keeps both the 999- and 1,000-item child-list cases
unmetered by default. It separately measures each evaluator's cumulative baseline
and checks zero, one-short, exact and largest-u64 policies against that evaluator's
ledger. It preserves the existing depth-255/256, cancellation and deep-drop controls.
The correction updates the policy expectation rather than restoring a default cap.

The large-list VM/reference positive and wrong-final-type controls have observed
passes in the initial focused selection below. The corrected core/admission/map-policy
selection subsequently passed all 108 selected tests. That evidence precedes the later
stored-proof/public-discovery repair and does not substitute for its verification or
the complete staged-tree seal.

## Observed evidence and current acceptance boundary

The 21 Rust/tool changes were applied under the canonical target lease and formatted.
The application receipt SHA-256 is
`2469dabd5b802ade4363d8990c820b524b7d7b660d77a7f84abfa81b1a315844`.
The initial focused process joined with exit 1; Cargo exited 101. Its terminal
summary was 107 passed, one failed, zero ignored and 2,735 filtered, with 6.80s test
execution and 391,702,033,744 ns total driver time. The log SHA-256 is
`ba3b2018257f7979f3148837532d68a2f92e130a879ff84563e5a117261be110`.

The sole failure was
`vm_map_tests::map_admission_keeps_depth_and_aggregate_item_bounds_in_foreground`.
That existing fixture expected unmetered admission of 1,000 map entries, each holding
1,000 list items, to fail at the removed aggregate item ceiling. The actual result
was `Ok(I64(1000))`. The large-list VM/reference positive and wrong-final-type controls
passed. The map-policy fixture correction was subsequently applied with receipt SHA-256
`c2754e5088f8b8d2a58f30715cca255b1fa276e795e038ca10413378543c97d6`.
The initial failed selection supplies no full-gate or integrated-list acceptance claim.

The corrected focused process joined with exit 0 at request `capacity-resume-1030-045`;
Cargo exited 0. Its summary was 108 passed, zero failed, zero ignored and 2,735
filtered, with 7.43s test execution and 376,388,950,567 ns total driver time. The
38,365-byte log SHA-256 is
`4cf89958aec4f7fa0adff9753f60bc0ef6bb150699a514d2dba677826c9caef5`.
This execution used the candidate with 22 changed source paths, including the
map-policy repair. It precedes the later supplemental stored-proof/public-discovery
repair, whose separate terminal verification follows. No full staged-tree acceptance
or integrated list commit is established by this focused result.

The supplemental stored-proof/public-discovery repair has application receipt SHA-256
`693fcb2710f7de2379ca0fc05341d8e323893cb8ec79fc8bbb8ea89da2bf6dd4`.
Its first verification driver joined with exit 1 at `capacity-resume-1030-053`.
Clippy passed for that repaired 23-path candidate in 39.51s, with log SHA-256
`5c15f77215c5055fc1689d6658aff0f7baaacd88bf5db85deab3b89c7ce8d20d`.
The proof invocation selected the binary target, whose entry delegates to the
library. Cargo exited 0 after running zero tests; the driver rejected the missing
three-test inventory. The zero-test log SHA-256 is
`885469bc3fd38643568d0d56958afe114ee2052101534a7e4368002c58958337`,
and the driver receipt SHA-256 is
`d4a7b35de7b76b872a5129ed2fc08ab88e630b9c9bb45b22111e8f0494ed74e1`.

This was an incomplete test selection, not an observed test pass or a source defect.
No product source edit followed that refusal. The second attempt selected the actual
library target and explicitly reused only the same-source Clippy observation. Its
owning session joined with exit 0 at `capacity-resume-1030-064`. The 11,970-byte
source-bound receipt records all 23 source hashes and has SHA-256
`3045f15a8b44023fab176458ea6ecc2be2baacc14f9e09777152a0f348603d78`.

| Supplemental operation | Observed terminal outcome |
| --- | --- |
| Three active library proof controls | 3 passed, 0 failed, 0 ignored, 352 filtered; 0.00s test execution |
| Public capability/discovery control | 1 passed, 0 failed, 0 ignored, 351 filtered; 0.10s test execution |
| Current executable selection | Cargo exited 0; both selected executable records contain `fresh: false` and exact SHA-256 identities |
| Pure-tail proof run | Passed; 38 discovered fields, 82 commands and complete cleanup |
| Public discovery generation and verification | Both exited 0; all eight generated files were updated and verified |

The three library controls cover restored-guard/raw-readmission cost rejection,
serialized list exactness rejection, and each mapping tier's exact list capture and
scope slots. Their log SHA-256 is
`ebbaf69fb98b232a08ed428730497615aa2dd43e99d41b15285e6fef94d49502`.
The public-control log SHA-256 is
`b84a300761ff7a21510dab6408efd39f6cafb3acd57dca53eb8a14064a8be201`.
Pure-tail's receipt SHA-256 is
`9e08a4a7907586468036188c1b67fb06816f0d5efd65899f4f66d008c9d746ac`.
Its internal elapsed time was 48,092,616,427 ns; the enclosing driver operation took
48,564,108,076 ns including process overhead. Source-batch authority before and after
was identical: 14 files, 2,399,591 bytes, and HEAD SHA-256
`aaefd72a3dbeb386c241711c10a4a8a1882cd06e5bba26deb5a0523b873da0df`.
The generated files share the updated registry header identity; the operations
document also carries the changed capability content.

These are source-bound focused observations, with the Clippy reuse explicitly
disclosed. They establish no fresh-full gate acceptance. At source freeze, only the
complete staged-tree seal and integration remain pending for this implemented slice;
their eventual outcome belongs in the acceptance commit and external terminal record.

The parent Phase 1 cutover is independently accepted. Its source-bound seal passed
all 29 full gates fresh with zero reuse or unrun gates and stable inputs, literal
selfhost 138 passed / 2 ignored, native 11 passed, both named closure/inventory
controls, and the static-musl release target policy check. The full receipt SHA-256
is `668c3548809c8abf9a063f017a6a0ed9a618754bcbb91749edc6065611955503`.
Normal fast-forward integration and remote commit/tree readback delivered the accepted
parent. The integration record SHA-256 is
`fcde1e7da229ffec013952191454a306317e250c6181692a9d33669190b4504b`.
Those results prove that parent tree and do not accept this later list candidate.

Before list integration, freeze the intended tree and bootstrap the independent
verifier against its exact source/tree. Run literal `bash tools/selfhost/verify.sh`,
the separately selected maintained-native-closure and whole-owner/type controls,
all 29 fresh full gates with zero reuse, and release target. Join the owning terminal
results, verify unchanged source and cleanup, and bind acceptance in the commit and
external record. Do not edit the frozen source merely to add a passed claim.

## Pending design and unproven claims

The remaining 256-depth guards and recursive value/type/lifecycle consumers still
require their coupled stack-safe redesign. Structural call/operand policies, normalized
map and schema admission limits, owned/view-storage limits, and JSON/typed-data codec
boundaries are separate work. Persistent-map overflow values, wide Mode-1 profile 7,
canonical object envelopes, storage catalog hierarchy and application data/queue
policies remain in the capacity ledger.

This runtime representation slice does not establish repository-wide unbounding,
Tier 2 source-driven semantic self-hosting, source-derived `A_host = A1 = A2 = A3`,
Tier 3 sovereignty, native phase reclamation or bounded same-process RSS. Existing
Tier 1 backend equations retain their disclosed Rust-normalized/projected-input scope.
