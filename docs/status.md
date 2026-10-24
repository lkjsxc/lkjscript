# Current status

Updated 2026-10-24 (UTC). Accepted typed meaning is the sole editable authority.
Production bytecode is checked against an independent canonical interpreter.
Verification has no three-minute requirement; elapsed time is an observation.

## Capabilities

One checkout and one leased Cargo target serve admitted packages/programs and
one current schema epoch. Public tx preserves symbolic identities, runs
impact-scoped differential tests and publishes atomically. Safe Rust, denied
lint invariants and the no-Python policy remain intact.

The lkjscript-written Tier 1 backend consumes Rust-admitted LKJCI1 mode 0/profile 4
for standalone Linux x86_64 ELF and mode 1/profile 5 for current .lkja artifacts.
The additional compile-source-elf route consumes complete raw LKJSG001 and performs
restricted scalar E2 admission and E3 lowering inside lkjscript. The
[self-hosting specification](spec/selfhosting.md) owns the complete capability
matrix. Native heap allocations remain process-owned until exit; same-process
and phase reclamation still require implementation.

## Current milestone

The source-driven scalar path checks all supplied owners/types, including
uncalled functions, untaken branches and unused descriptors. It admits pure
monomorphic Unit/Bool/I64 functions, locals, If, Let, Sequence and direct calls.
It resolves the command Target and component Port internally, produces its own
normalized instructions and flow certificates, and emits ELF through the existing
backend. Empty-row task callables remain unsupported tasks.

Five scalar tests cover eight independently executed small positive programs, eight
semantic failures, two unsupported-form controls and two independently constructed
hash-consistent structural failures. Bool/Unit parameter and result calls,
I64 boundaries, nested lexical scopes, operand prefixes, direct/mutual recursion
and alternate component selection are exercised. Missing links and a disconnected
cycle pass commitment checks before rejecting at their intended structural layer.
The additional balanced source witness lowers 32,770 normalized instructions
and independently executes to 42. The [scalar decision](decisions/20261023-source-scalar-e2-e3.md)
and [capacity decision](decisions/20261023-mode0-node-capacity.md) record exact
contracts, complete-carrier identities, measurements and evidence boundaries.
The [carrier-capacity decision](decisions/20261024-source-carrier-byte-capacity.md)
extends complete raw admission beyond the former 8 MiB quota: a native checker
accepts an 8,433,806-byte carrier and rejects a hash-consistent unused missing
type at endpoint admission. This extends E1 capacity, not Text/List E2 permission.

Public adoption exposed a composed callable-work policy blocker. The unchanged
planner transaction now succeeds after replacing the ordinary composed-work quota
with explicit None. Some(limit) preserves exact checked quota enforcement.
Cancellation, finite specialization, metadata reservations and depth checks remain.
The independent associated/read-view/Region preparation chain and package-source
collectors retain their existing bounds pending coherent downstream audits.

The previous aggregate mode-1 byte quota remains removed: the maintained input
now exceeds 8 MiB and succeeds. Native input transport uses checked growable
mappings. The 700-unit artifact guard still protects flat map emission. Ordinary
Rust execution retains explicit unmetered cumulative policies; GC preparation and
evidence retain their earlier aggregate-quota removals. None of those changes
removes unrelated representation, recursion or physical limits.

## Maintained compiler baseline

The authoritative project has 72 modules, 554 pure functions and 23 externals,
33,655 owners, 35 types and 11,064 retirements at
rev_2ea581ffa2b517be5b5decabc899000f11f46d54f543374c47ff283a844de098.
Semantic state:
semantic_state_26e8c812bea771560ad512c8f83febbe2da12dc63fc927fa480a9814e1e04972.
The complete LKJSG001 carrier is 7,941,194 bytes, SHA-256
a2dbf5a6604c4c3cbbce3df4fe3b003256eea3e4af454832181da1c5002759bf.
Its aggregate 8 MiB quota and the matching index-backing quota are removed.
The retained positive signed-32-bit proof domain still requires
L <= 16,777,183 - 8*N, with independent layout, count and object restrictions.

Mode-1 input occupies 9,086,555 bytes for 581 units and emits a 9,603,621-byte
artifact. The Tier 1 native backend closure has 244 functions and 16,456 nodes
in 832,576 mode-0 bytes. Direct local full-kernel validation consumes 3,094,141
work units. Composed callable analysis separately reports 7,995,020 work units
and 11,401,876 modeled metadata bytes. These measures are not interchangeable
with an aggregate public validation total. The maintained closure does not
select associated preparation.

The complete inventory records exact intrinsic signatures/substitutions, parameter
use modes, lexical scopes, recursion and syntactic tail calls. It remains a derived
observation, not E2 permission or an E3 custody certificate. Physical packs occupy
406,592,267 bytes; versioned historical reachability retains 485,420,559 canonical
payload bytes across 227,308 objects. Isolated public GC measures reachability
while preserving the authoritative project. Native requested allocation, live
retention, mapped capacity and RSS have not been measured.

The carrier-capacity transaction updates two existing bodies through public
authoring while preserving declaration and parameter identities. Live owners fall
by 11, while 84 added retirements increase complete carrier bytes by 16,139.
Historical retirement records remain part of the complete carrier. No history
truncation, retirement deletion or project reseed occurs.

## Verification and next work

Native maintained-closure measurement reproduces Tier 1 C2=C3 and identical
C1/C2/C3 artifact bytes, bound to the complete source carrier. Mandatory selfhost
gates additionally own VM/reference equality and behavioral/compiler-source
controls. These fixed points still consume Rust-normalized input. Scalar
source-to-ELF execution does not establish maintained compiler-source fixed points
or equality with independently packaged Rust canonical artifacts.

The combined source frontend/backend now exports and consumes 1,661,372 mode-0
bytes containing 561 functions, 34,201 nodes and 31 type entries. The independent
32,768-node guard is removed across exporter, flow admission and scalar planning/
packing. Checked complete-layout arithmetic and the retained byte/function/local/
type domains bound native addresses and displacements. Explicit proof allowance
still rejects at one less than required; a hash-consistent u32-max extent rejects
before a table walk.

Backend C1 and C2 emit identical 5,272,820-byte frontend images. Both images and
the Rust-bootstrap frontend compile a complete raw scalar carrier to an
independently executed program and reject a hash-consistent unused Bytes type.
This closes the aggregate-node capacity blocker while retaining the narrower
scalar source profile.

Next, admit and lower exact I64/Bool intrinsic declarations and introduce explicit
Target selection. Immutable Bytes, structural records, Text, closed List
substitutions and complete ByteBuffer cleanup then extend E2/E3 toward the
maintained compiler profile. Reassess complete source-carrier headroom before
adopting each slice; retirement accumulation makes live-source size insufficient.
Preserve whole-carrier admission and complete-carrier identity binding. The
remaining source proof representation and artifact-unit guard remain concrete
dependencies. Native memory reclamation drafts have no runtime acceptance yet.

Publication requires exact staged-tree bootstrap, tools/selfhost/verify.sh,
every gate in full --fresh and release-target inspection. The current checker
entrypoint is tools/bootstrap-verifier.sh and lkjscript-dev; there is no root
./verify executable. Source/test changes select the full matrix for changed,
so the matrix runs once. Terminal evidence is under
.artifacts/selfhost/source-carrier-byte-capacity, its frozen acceptance directory and the
official latest receipt.
Git owns integration records. No public product release or deployment is implied.

WS1 and full Tier 2 remain incomplete. Remaining obligations include
authoring/inspection quotas, recursive depth limits, custody/type encodings,
full active-profile E2/E3, nominal/closure/witness/read-view/Region expansion,
source-driven C2=C3 and A1=A2=A3, independent full-profile controls and measured
same-process native reclamation. [Verification](spec/verification.md) owns final
acceptance.
