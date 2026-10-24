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
The additional compile-source-elf route now consumes LKJSRQ01/profile1: a 32-byte
header carrying the caller's canonical Target key followed by the complete raw
LKJSG001 carrier. It performs restricted scalar E2/E3 and eight exact pure
I64/Bool intrinsics inside lkjscript, then selects the admitted live Target.
The [self-hosting specification](spec/selfhosting.md) owns the complete capability
matrix. Native heap allocations remain process-owned until exit; same-process
and phase reclamation still require implementation.

## Current milestone

The source-driven scalar path checks all supplied owners/types, including
uncalled functions, untaken branches, unused descriptors and unselected targets.
It admits pure monomorphic Unit/Bool/I64 functions, locals, If, Let, Sequence,
direct calls and the eight exact I64/Bool External implementations. Permission
checks complete implementation identifiers, signatures, unrestricted parameter
modes and empty generic binders. Empty-row task callables remain unsupported tasks.

The canonical selector transaction permits one or more command Targets. After
request-header validation and complete-carrier E1/E2, lkjscript resolves the
selected Target, its Component and pure `() -> I64` function Port, derives the
normalized entry index, and emits ELF through the unchanged backend. The host
supplies only the typed Target identity and raw carrier. Missing/retired selectors
reject after whole-inventory admission; unframed legacy source input rejects.
The [selector decision](decisions/20261024-source-target-selector.md) records the
current-format transition, bootstrap, single adoption and evidence boundaries.

The five existing scalar tests retain the 26 intrinsic cases, eager left-to-right
evaluation and exact value/trap expectations, the balanced 32,770-instruction
witness, and independently constructed hash-consistent missing-endpoint and
disconnected-cycle controls. New request controls cover selection from the same
carrier, malformed framing and invalid unused targets. The joined selector suite
passed all five tests, with zero failures or ignored tests. The accepted
[intrinsic decision](decisions/20261024-source-scalar-intrinsics.md) retains its
exact signatures and earlier evidence scope.

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

Accepted main predecessor is commit
4165156d4d4c9726af61199d34f9b1b2053b9ef3, tree
a08a988e92c8500102856f6d05fff8f31cdf86c4. Selector public adoption succeeded once,
with differential equality, seven updates and 445 created canonical owners, at
rev_22b1d6d49b74893bf402af9461931f8c372f820ada378524eb50ef892a9f03d9.
It adds one callable and preserves retained declaration/parameter identities.
The joined maintained measurement, complete profile and isolated historical
reachability receipt provide the counts below.

| Current selector observation | Value |
| --- | --- |
| Modules / pure functions / externals | 72 / 560 / 23 |
| Live owners / supplied types / retirements | 34,006 / 35 / 12,009 |
| Complete LKJSG001 bytes / SHA-256 | 8,201,410 / db0ae8fd2a2c1d529bc43297bd29fa5308224e1ed5c379bc62c31d0a4d335ee4 |
| Semantic state | semantic_state_0626b9fe9d87ae871e734eafa2e45f60533f163388f1ee7e9a705b0921ba8599 |
| Mode-1 input bytes / units | 9,177,461 / 587 |
| .lkja output bytes / units | 9,699,624 / 587 |
| Direct local full-kernel work | 3,130,736 |
| Composed callable work / modeled metadata bytes | 8,105,847 / 11,531,962 |
| Physical packs / bytes | 390 / 430,408,188 |
| Reachable canonical history / retained pack bytes | 508,557,991 / 405,487,133 |

The accepted intrinsic before inventory is 72 modules, 559 pure functions and
23 externals; 33,919 live owners, 35 supplied types and 11,651 retirements. Its
complete carrier is 8,110,321 bytes, mode-1 input is 9,155,057 bytes for 586 units,
and artifact output is 9,675,994 bytes. The selector decision preserves the
separated before/after table. Measured deltas are +87 live owners, +358
retirements and +91,089 carrier bytes. Mode-1 input grows by 22,404 bytes and one
unit; the artifact grows by 23,630 bytes. These compare maintained closures, not
speed on one pinned input. Historical retirements remain in the complete carrier.

The source carrier's aggregate 8 MiB quota and index-backing quota remain removed.
The retained positive signed-32-bit proof domain requires
L <= 16,777,183 - 8*N, with independent layout, count and object restrictions.
Composed work remains passive telemetry under explicit unmetered policy, not
10-million quota headroom. Other validation/preparation bounds remain distinct.
The inventory identifies remaining record/Text/List/borrow/consume dependencies;
it grants neither source-profile permission nor cleanup certificates.

Current isolated GC retained 234,781 objects with 508,557,991 canonical bytes and
405,487,133 pack bytes, using 568,661,927 modeled preparation bytes. It reclaimed
784 objects and 24,921,055 pack bytes in the copy, reducing 390 packs to 16 in
180,613.176497 ms. The original project, sources, index, branch and stashes were
preserved; the copy was removed successfully. The raw report digest is
`8f1a844b8e3c782d0f848ce56cf591db825d6ff2ace8cae01151de8b36cbbcd1`. Native
requested allocation, live retention, mapped capacity and RSS remain unmeasured;
historical storage and modeled GC retention do not establish native reclamation.

## Verification and next work

The joined selector run passed five scalar tests, one explicitly enabled
maintained-native measurement and one complete-profile test, all with zero
failures or ignored tests. Test times were 365.19 / 24.42 / 11.46 seconds; whole
command times were 559,348.815185 / 24,566.869172 / 11,579.212302 ms. The continuation
is `f2d8a6839299314c1068ba319b4b46b0b7d2f1ddc5df76973ea10b4d5559d837`. The root's
separate recovery proof passed and binds the initial compile failure, exact
three-prefix/three-line-join correction, retained single adoption and source
inventory; its SHA-256 is
`5196441bfb8ddee34e9e3d36729760b286433a9001fd6ae59349d2d882767f80`.

Current normalized-input Tier 1 backend ELF C2=C3 has digest
`55eaea3fcc2604a386f68a5dffd5bb9c2ca8e936ca0d2bf94235e1770c2487d5`; the artifact
from C1=C2=C3 has digest
`2f2ce9aedd41988db53042561c4ea1b3a08af8a4927ab849694875204a4f22ca`. Source
compiler executables emitted by backend C1/C2 are identical. All three native
source compilers accept the same framed scalar request, emit identical programs
printing 42 and reject a hash-consistent unused Bytes descriptor with diagnostic
13. The decision binds the complete carriers/requests, selected Target, compiler
identities and execution context. These observations do not establish maintained
compiler-source C2=C3, canonical Rust A1=A2=A3 or same-process native reclamation.

Next extend Bytes/flat records, Text, closed List substitutions and complete
ByteBuffer custody/cleanup. Reconcile and measure each actual successor; static
local drafts are not adoption or runtime evidence. The accepted before inventory
has 586 mode-1 units; the selector inventory has 587. Replace flat map emission
before a measured successor would exceed the retained 700-unit limit. Reassess
complete carrier size, retirement count and proof-domain constraints separately.
Preserve whole-carrier admission and complete-carrier identity binding.

No selector full publication result is claimed. Required acceptance is the exact
staged tree through tools/bootstrap-verifier.sh, literal tools/selfhost/verify.sh
(127 expected selfhost passes plus two ignored and ten native passes), the
stdout-selected lkjscript-dev check full --fresh --machine --jobs 4 (29 fresh
gates, zero reuse), and that same verifier's release target. There is no root
./verify executable. One checkout and the canonical target stay under the
appropriate exclusive gate lease. The matrix runs once for the frozen candidate.
Targeted evidence is under .artifacts/selfhost/source-target-selector; future
acceptance and terminal publication receipts must bind its actual source,
revision and tree. Git owns final integration and remote identities. No public
product release or deployment is implied.

WS1 and full Tier 2 remain incomplete. Remaining obligations include
authoring/inspection quotas, recursive depth limits, custody/type encodings,
full active-profile E2/E3, nominal/closure/witness/read-view/Region expansion,
source-driven C2=C3 and A1=A2=A3, independent full-profile controls and measured
same-process native reclamation. [Verification](spec/verification.md) owns final
acceptance.
