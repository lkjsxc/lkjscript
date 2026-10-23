# Current status

Updated 2026-10-22 (UTC). Accepted typed meaning is the sole editable authority.
Production bytecode is checked against an independent canonical interpreter.
Verification has no three-minute requirement; elapsed time is an observation.

## Capabilities

One checkout and one leased Cargo target serve admitted packages/programs and
one current schema epoch. Public tx preserves symbolic identities, runs
impact-scoped differential tests and publishes atomically. Safe Rust, denied
lint invariants and the no-Python policy remain intact.

The lkjscript-written Tier 1 backend consumes Rust-admitted LKJCI1 mode 0/profile 4
for standalone Linux x86_64 ELF and mode 1/profile 5 for current .lkja artifacts.
Restricted E1 recognizes interfaces, operations/parameters, requirements,
function R/E binders and TaskFunction types. E1 does not replace Rust semantic
admission or normalization. The [self-hosting specification](spec/selfhosting.md)
owns the complete capability matrix. Native heap allocations remain process-owned
until exit; same-process and phase reclamation still require implementation.

## Current milestone

Mode-1 artifact input has no default aggregate 8 MiB quota. The Rust producer uses
explicit optional positive u64 allowance; zero represents unmetered admission.
The native checker compares full-width allowance halves and validates extents
before multiplication. Stale mode-1 profile-4 inputs reject deterministically.
Mode 0 remains profile 4. The [capacity decision](decisions/20261022-mode1-byte-capacity.md)
records the producer/consumer cutover, finite progress and independent controls.

The 700-unit guard remains because the current unit map emits a single canonical
leaf. u32 framing, 8 MiB individual units, finite table/local counts and native
256 MiB value construction remain separate restrictions. Both native filter
emitters already read through EOF using checked growable mappings, including
exact-fit EOF and moving remaps; the
[transport decision](decisions/20261022-native-input-growth.md) owns that invariant.

The [complete active-source inventory](decisions/20261022-active-compiler-profile.md)
now records all owners, types, intrinsic signatures/substitutions, lexical scopes,
direct/mutual recursion and syntactic tail calls. It is derived observation and
does not precompute E2 permission or E3 custody certificates.

Ordinary Rust execution retains the removal of default cumulative instruction,
modeled allocation, collection-item and capability-call quotas. Explicit quotas
still use checked precharge; passive observations grant no capacity. Live
frame/stack limits and cancellation remain enforced. GC preparation and evidence
also retain their removal of default aggregate quotas; physical compaction
preserves immutable history.

## Maintained compiler baseline

The authoritative project has 62 modules, 442 pure functions and 23 externals,
28,584 owners, 27 types and 10,586 retirements at
rev_dc8bb4c50291e16e49a520baf65b2c93dc039d1305fc0f9c0e2a961767b03c70.
Semantic state:
semantic_state_db9cdb516c961897effbf694efb3530f047cab7de3faa46dffae067760576b0a.
The complete LKJSG001 carrier is 6,985,830 bytes, SHA-256
6a29a86a8d10164f5d302c6abea284e071bbe1c426e0e3c7df088eecfb5948b0.

Mode-1 input occupies 7,754,834 bytes for 467 units. The native closure has
244 functions and 16,460 nodes in 832,748 mode-0 bytes. Direct local full-kernel
validation consumes 2,573,097 work units. Public rebuilt current-validation
separately reports 9,177,359 aggregate units across local and composed-source
operations; that sum does not measure headroom under a single admission ceiling.
Physical packs occupy 353,464,648 bytes. Reachable historical payload and native
requested/live/mapped/RSS quantities have not been remeasured.

The source transaction preserves existing module, function and parameter
identities. No retirement deletion, history truncation or project reseed occurs.

## Verification and next work

The publicly authored capacity control has 700 supplied units and a 9,567,313-byte
input, emits 9,739,154 artifact bytes, and preserves every expected source/unit
object after strict loading. The independent entry returns 42 in both evaluators.
Focused controls cover exact selected allowances, full-u64 high words,
overflow-safe extents, stale profiles, malformed complete bundles and unchanged
mode-0 custody admission.

Native maintained-closure measurement reproduces C2=C3 and identical C1/C2/C3
artifact bytes. Mandatory selfhost gates additionally own VM/reference equality
and behavioral/compiler-source controls. These remain Rust-normalized Tier 1
proofs. Complete carrier, executable, input and output digests are in the decision;
source-driven C2=C3 and A1=A2=A3 are not yet established.

Publication requires exact staged-tree bootstrap, tools/selfhost/verify.sh,
every gate in full --fresh and release-target inspection. The current checker
entrypoint is tools/bootstrap-verifier.sh and lkjscript-dev; there is no root
./verify executable. Source/test changes select the full gate set for changed,
so the complete checker matrix runs once. Terminal evidence is under
.artifacts/selfhost/mode1-byte-capacity and the official latest receipt.
Git owns integration records. No public product release or deployment is implied.

Next implement a source-driven scalar semantic/lowering path that checks every
supplied function, branch and type before emitting native code. Resolve all
port/target relationships inside lkjscript, reject unsupported task kinds
explicitly, and use raw LKJSG001 transport from Rust. Then extend Bytes/Text,
records, closed List substitutions and complete ByteBuffer cleanup to the
maintained compiler profile. Measure the combined checker/backend closure before
requiring a format redesign; LKJSG001 retains 1,402,778 bytes of current headroom.

WS1 and Tier 2 remain incomplete. Remaining obligations include authoring/inspection
quotas, recursive depth limits, custody/type encodings, source-driven E2/E3, the
declared region/closure/witness profile, complete-carrier-bound fixed points,
hash-consistent semantic mutation controls, independent full-profile programs and
measured same-process native reclamation. [Verification](spec/verification.md)
owns final acceptance.
