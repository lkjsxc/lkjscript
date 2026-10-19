# Current status

Updated 2026-10-19. Accepted typed meaning is the sole editable authority.
Production bytecode is checked against an independent canonical interpreter.
Verification has no three-minute requirement; elapsed time is an observation.

## Capabilities

One checkout and one shared Cargo target serve bounded copy-on-write memory
projects, admitted standard packages/programs and one current schema epoch.
Public `tx` preserves symbolic identities, runs impact-scoped differential tests
and publishes atomically. Rejected or interrupted preparation cannot partly publish.
Default checks retain a bounded receipt and remove joined scratch.

Lexical regions connect allocation, reads, writes, freeze, relative offsets and
digest-bound slab persistence to both evaluators. Frozen parallel reads share an
aligned arena; unique custody can move into a joined child. Runtime generations
and compiler control-flow admission preserve lexical cleanup. Planner fixtures
retain all 176 exact IndexedData/BlockRead expectations.
The [region specification](spec/regions.md) owns layouts and limits.

The native backend performs bounded trap-preserving optimization and emits
standalone Linux x86_64 ELF. The lkjscript-written Tier 1 backend consumes
Rust-admitted LKJCI1 profile 4 input and emits ELF or current `.lkja`.
[Native ELF](spec/native-elf.md) and [self-hosting](spec/selfhosting.md) own contracts.

## Current milestone

Milestone E adds a bounded raw canonical source carrier and reference closure.
Thirty-nine functions across six maintained modules bind exact objects, recompute
logical state and resolve every supported owner/type endpoint, including unused
definitions, untaken branches and final array elements. Foreign package/blob
boundaries reject explicitly. [Source input](spec/selfhosting-source-input.md)
separates these checks from full semantic admission.

Public authoring accepted the maintained graph at
`rev_ff46acac56b71e755fbd9add4bb7f13ee2af6f6b5b6fbcb05e788d43a7f856ea`.
The complete raw carrier has 22,272 owners, 18 types, 6,271 retirements and
5,015,133 bytes. Its semantic state is
`semantic_state_cbb1ffe494e4f63006c00bb0ad83d966ce9a85aa9100fc534119cc20c632ce55`.
Type-inventory membership needs a separate complete-carrier identity.

The host callable-flow validator uses one tree entry operation per syntax visit.
Exact occurrence identity, reservation, cancellation and the 10,000,000 work limit
remain intact. Eliminating the second search admits the growing source within
that unchanged limit.

Public `gc` retains accepted history and selected package/compiler roots. Disk
compaction produces deterministic compressed partitions with a 32 MiB raw-framing
target and oversized singletons. All replacement packs precede one durable catalog
switch and obsolete-pack cleanup. [Storage](spec/semantic-storage.md) owns the
capacity, interruption and retry contract.

## Acceptance and next work

Focused graph validation passed all 11 cases on VM, independent reference and
standalone native execution. The callable-flow module passed 67 tests, including
duplicate-path exhaustion/cancellation and failed preparation without insertion.
All 47 storage tests passed, including each durable publication checkpoint and
partial multi-pack installation. Public GC retained all 134,325 reachable objects
and produced six packs, each below 32 MiB.
The verified baseline `13771737` passed all 29 fresh full gates; its exact evidence
is in `.artifacts/selfhost/accepted-e-source-bindings/receipt.json`.

Acceptance for the delivered Git source/tree belongs to the exact-source changed,
fresh full and release-target results, recorded in the bounded latest receipt and
`.artifacts/selfhost/accepted-e-source-graph/receipt.json`. A clean changed check
selects Git-status gates, not the committed diff; full acceptance requires the
complete discovered matrix and stable inputs. Git records integration separately.

Next implement parent membership, lexical scope and unique syntax ownership over
the admitted source, then cycle/type/effect/custody/witness checks and normalization.
Tier 2 still requires source-graph C2/C3 and artifact fixed points without Rust
semantic preprocessing. Native allocation remains process-owned without lexical
region reclamation or a bounded-RSS claim. Broader source types and dependencies
remain explicit limits. [Verification](spec/verification.md) owns acceptance;
[release](release.md) owns candidate and publication authority.
