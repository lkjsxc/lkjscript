# Current status

Updated 2026-10-23 (UTC). Accepted typed meaning is the sole editable authority.
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

Four scalar tests cover eight independently executed positive programs, eight
semantic failures, two unsupported-form controls and two independently constructed
hash-consistent structural failures. Bool/Unit parameter and result calls,
I64 boundaries, nested lexical scopes, operand prefixes, direct/mutual recursion
and alternate component selection are exercised. Missing links and a disconnected
cycle pass commitment checks before rejecting at their intended structural layer.
The [scalar decision](decisions/20261023-source-scalar-e2-e3.md) records the exact
contract, complete-carrier identities, measurements and evidence boundaries.

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

The authoritative project has 72 modules, 553 pure functions and 23 externals,
33,640 owners, 35 types and 10,586 retirements at
rev_58c7fb8d64952a3b5f12e6637b4265657ff13b023a870af12af8b3f19153ba87.
Semantic state:
semantic_state_a82dd2ca1863cd9923f8f417570f7d4fd23bdbf7db94ab96328e67a5e866ca71.
The complete LKJSG001 carrier is 7,836,565 bytes, SHA-256
8a9a3da1fa795170436ae4ec1f3716425ef988e08a2f81901475133005846e33.
Its separate 8 MiB ceiling leaves 552,043 bytes of headroom.

Mode-1 input occupies 9,081,896 bytes for 580 units and emits a 9,598,554-byte
artifact. The unchanged Tier 1 native backend closure has 244 functions and
16,460 nodes in 832,748 mode-0 bytes. Direct local full-kernel validation consumes
3,091,132 work units. Public rebuilt current-validation separately reports
11,111,393 aggregate units; that sum is not headroom under one admission ceiling.
Composed analysis alone reports 7,985,578 work units and 11,392,635 modeled metadata
bytes. The maintained closure does not select associated preparation.

The complete inventory records exact intrinsic signatures/substitutions, parameter
use modes, lexical scopes, recursion and syntactic tail calls. It remains a derived
observation, not E2 permission or an E3 custody certificate. Physical packs occupy
390,397,862 bytes; versioned historical reachability retains 469,578,376
canonical payload bytes. These quantities are distinct. Native requested
allocation, live retention, mapped capacity and RSS have not been measured.

The public source transactions add ten modules without replacing old declarations
or adding retirements. No history truncation, retirement deletion or project
reseed occurs.

## Verification and next work

Native maintained-closure measurement reproduces Tier 1 C2=C3 and identical
C1/C2/C3 artifact bytes, bound to the complete source carrier. Mandatory selfhost
gates additionally own VM/reference equality and behavioral/compiler-source
controls. These fixed points still consume Rust-normalized input. Scalar
source-to-ELF execution does not establish maintained compiler-source fixed points
or equality with independently packaged Rust canonical artifacts.

The combined source frontend/backend builds with the Rust native backend:
560 functions and 698,850 ELF bytes. Its mode-0 export reaches the aggregate
32,768-instruction guard and rejects. The exact normalized node count remains
unmeasured. This is the next concrete capacity dependency. Audit and remove that
guard across exporter, flow, scalar planner/packing and native layout consumers
with checked finite-width addressing and an actual above-threshold control;
do not replace it with another arbitrary quota.

Then extend Bytes/Text intrinsics, structural records, closed List substitutions
and complete ByteBuffer custody cleanup to the maintained compiler profile.
Preserve whole-carrier admission and complete-carrier identity binding through
each semantic extension. The nearby source-carrier and artifact-unit bounds
must be addressed when they become actual dependencies.

Publication requires exact staged-tree bootstrap, tools/selfhost/verify.sh,
every gate in full --fresh and release-target inspection. The current checker
entrypoint is tools/bootstrap-verifier.sh and lkjscript-dev; there is no root
./verify executable. Source/test changes select the full matrix for changed,
so the matrix runs once. Terminal evidence is under
.artifacts/selfhost/source-scalar and the official latest receipt.
Git owns integration records. No public product release or deployment is implied.

WS1 and full Tier 2 remain incomplete. Remaining obligations include
authoring/inspection quotas, recursive depth limits, custody/type encodings,
full active-profile E2/E3, nominal/closure/witness/read-view/Region expansion,
source-driven C2=C3 and A1=A2=A3, independent full-profile controls and measured
same-process native reclamation. [Verification](spec/verification.md) owns final
acceptance.
