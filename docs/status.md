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
restricted scalar E2/E3 and eight exact pure I64/Bool intrinsics inside lkjscript.
The [self-hosting specification](spec/selfhosting.md) owns the complete capability
matrix. Native heap allocations remain process-owned until exit; same-process
and phase reclamation still require implementation.

## Current milestone

The source-driven scalar path checks all supplied owners/types, including
uncalled functions, untaken branches and unused descriptors. It admits pure
monomorphic Unit/Bool/I64 functions, locals, If, Let, Sequence, direct calls and
eight exact I64/Bool External implementations. Permission checks complete
implementation identifiers, signatures, unrestricted parameter modes and empty
generic binders. The compiler requires one command Target, resolves its Component
Port internally, derives normalized instructions and flow certificates, and emits
ELF through the unchanged backend. Empty-row task callables remain unsupported
tasks.

The joined scalar tests add 26 intrinsic cases: six exact values, nine exact
runtime traps and eleven semantic/profile rejections. The complete-carrier E2
matrix compares VM, reference and native results; native E3 compiles raw carriers
and executes independent value/trap expectations. Eager Boolean arguments,
left-to-right evaluation and integer trap ordering are preserved. The five
existing scalar tests also retain the balanced 32,770-instruction witness and
independently constructed hash-consistent missing-endpoint/disconnected-cycle
controls. No Rust target lowering or source prefilter is supplied.
The [intrinsic decision](decisions/20261024-source-scalar-intrinsics.md) records
the exact eight signatures, added cases, canonical adoption and evidence scope.

The earlier [scalar decision](decisions/20261023-source-scalar-e2-e3.md) and
[node-capacity decision](decisions/20261023-mode0-node-capacity.md) retain their
contracts. The [carrier-capacity decision](decisions/20261024-source-carrier-byte-capacity.md)
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

Accepted main baseline is commit 01c805a202b258a1a9241f1edafd5feaa7655ce1.
The intrinsic canonical transaction and targeted controls have succeeded;
intrinsic publication still requires the exact-tree gates below.

The authoritative project has 72 modules, 559 pure functions and 23 externals,
33,919 owners, 35 types and 11,651 retirements at
rev_dd94b401eb30277e5289bbc9d651230337edcd6b1391e46fabcd92c0fe38d427.
Semantic state:
semantic_state_706b6db4ae98c6adb7511c56da0e893974a45585b91fbaf48e8e38593870fa19.
The complete LKJSG001 carrier is 8,110,321 bytes, SHA-256
b800b61e8d90d6c507fa9918130bc629bdbad491b9d9ec38f79b28beeaa64177.
Its aggregate 8 MiB quota and the matching index-backing quota are removed.
The retained positive signed-32-bit proof domain still requires
L <= 16,777,183 - 8*N, with independent layout, count and object restrictions.

Mode-1 input occupies 9,155,057 bytes for 586 units and emits a 9,675,994-byte
artifact. The Tier 1 native backend closure has 244 functions and 16,456 nodes
in 832,576 mode-0 bytes. Direct local full-kernel validation consumes 3,120,526
work units. Composed callable analysis separately reports 8,076,365 work units
and 11,499,321 modeled metadata bytes. The composed count is passive telemetry
under explicit unmetered policy, not a 10-million quota headroom measure.
These observations are not interchangeable with an aggregate public validation
total. The maintained closure does not select associated preparation.

The complete inventory records exact intrinsic signatures/substitutions, parameter
use modes, lexical scopes, recursion and syntactic tail calls. Its 24 structural
record types, 146 Text expressions, 91 List expressions, 14 generic sites,
one borrow parameter and 159 consume parameters identify further source-profile
dependencies. The inventory is a derived observation, not E2 permission or an E3
custody certificate.

Physical packs occupy 419,166,252 bytes across 346 packs, preserving all previous
history. Isolated public GC measures 497,593,841 reachable canonical bytes across
231,731 historical objects, retaining 397,154,190 pack bytes in the copy and
modeling 556,916,977 preparation-retention bytes. The run verifies the original
project/source/index/branch/history unchanged and successful copy cleanup. These
are storage observations. Native requested allocation, live retention, mapped
capacity and RSS remain unmeasured.

The intrinsic transaction updates nine existing bodies and adds five callable
declarations through public authoring, preserving retained declaration/parameter
identities. It creates 851 owners; after replacement, live owners increase by
264 and retirement records by 587. Complete carrier bytes increase by 169,127.
Historical retirements remain part of the carrier. No history truncation,
retirement deletion or project reseed occurs.

## Verification and next work

The joined targeted run passed all five scalar tests, one explicitly enabled
maintained native measurement and one complete-profile test, with no failures.
Their test-reported times were 321.97, 23.72 and 11.96 seconds; the corresponding
whole-command times were 516,360.533, 23,873.855 and 12,077.030 milliseconds.
These intervals are distinct and are not a same-input performance comparison.

Native maintained-closure measurement reproduces Tier 1 C2=C3 and identical
C1/C2/C3 artifact bytes, bound to the complete source carrier. The native digest
is 55eaea3fcc2604a386f68a5dffd5bb9c2ca8e936ca0d2bf94235e1770c2487d5;
the artifact digest is 6dab0ce912baa2306325d2f74c5e341f12264797213fd9bf8f67fe0cdd62840a.
These fixed points still consume Rust-normalized input. Scalar source-to-ELF
execution does not establish maintained compiler-source fixed points, canonical
Rust A1 parity or same-process native reclamation.

The combined source frontend/backend exports and consumes 1,672,927 mode-0
bytes containing 566 functions, 34,469 nodes and 31 normalized type entries.
Backend C1 and C2 reproduce identical 5,312,463-byte frontend images. Both images
and the Rust-bootstrap frontend compile a complete raw scalar carrier to a
program returning 42 and reject a hash-consistent unused Bytes type. Checked
complete-layout arithmetic and the retained byte/function/local/type domains
still bound native addresses and displacements.

Next implement explicit Target selection, then Bytes/flat records, Text, closed
List substitutions and complete ByteBuffer custody/cleanup. Reconcile and measure
each actual successor; static local drafts are not adoption or runtime evidence.
At 586 measured mode-1 units, replace flat map emission before a measured
successor reaches the retained 700-unit blocker. Reassess complete carrier size,
retirement count and proof-domain constraints separately. Preserve whole-carrier
admission and complete-carrier identity binding.

No intrinsic full publication gates have run yet. Required acceptance is the
exact staged tree through tools/bootstrap-verifier.sh, literal
tools/selfhost/verify.sh (127 expected selfhost passes plus two ignored and ten
native passes), the stdout-selected lkjscript-dev check full --fresh --machine
--jobs 4 (29 fresh gates, zero reuse), and that same verifier's release target.
There is no root ./verify executable. One checkout and the canonical target stay
under the appropriate exclusive gate lease. The matrix runs once for the frozen
candidate. Targeted evidence is under .artifacts/selfhost/source-scalar-intrinsics;
future frozen acceptance and terminal publication receipts must bind its actual
source, revision and tree. Git owns final integration and remote identities.
No public product release or deployment is implied.

WS1 and full Tier 2 remain incomplete. Remaining obligations include
authoring/inspection quotas, recursive depth limits, custody/type encodings,
full active-profile E2/E3, nominal/closure/witness/read-view/Region expansion,
source-driven C2=C3 and A1=A2=A3, independent full-profile controls and measured
same-process native reclamation. [Verification](spec/verification.md) owns final
acceptance.
