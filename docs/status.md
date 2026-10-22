# Current status

Updated 2026-10-22. Accepted typed meaning is the sole editable authority.
Production bytecode is checked against an independent canonical interpreter.
Verification has no three-minute requirement; elapsed time is an observation.

## Capabilities

One checkout and one leased Cargo target serve admitted packages/programs and
one current schema epoch. Public tx preserves symbolic identities, runs
impact-scoped differential tests and publishes atomically. Safe Rust, denied
lint invariants and the no-Python policy remain intact.

The lkjscript-written Tier 1 backend consumes Rust-admitted LKJCI1 profile 4 and
emits standalone Linux x86_64 ELF or current .lkja. Restricted E1 recognizes
interfaces, operations/parameters, requirements, function R/E binders and
TaskFunction types. Source admission does not replace Rust normalization.
The [self-hosting specification](spec/selfhosting.md) owns the capability matrix.
Native allocations remain process-owned until exit; no phase-reclamation claim.

## Current milestone

GC has no default aggregate preparation quotas for retained objects/bytes,
reference inventory, map entries, object reads or read bytes. Visited references,
complete map traversal, exact object bindings and cancellation still govern
reachability. Explicit caller-selected read allowances still refuse before payload
consumption. Saturating telemetry observes work without deciding acceptance.
Compaction keeps immutable history and uses deterministic multiple-pack partitioning.
This slice performs no retirement pruning, history truncation or project reseed.

Verification keeps complete compact receipts without a 64 KiB aggregate cap;
the evidence writer and archived-original reader also have no 128 MiB ceiling.
Gate outcomes, identities and bounded excerpts remain; joined transient work is
still cleaned up. Guidance has no hard line-count, editorial byte or word rule. The
[decision record](decisions/20261022-gc-preparation-quotas.md) records scope and controls.

## Maintained compiler baseline

The authoritative project is unchanged from 9eaaad6: 62 modules, 465 functions,
28,326 owners, 27 types and 9,881 retirements at
rev_e8682917d0eaef1ccd6dd37517ef3024b023d56cb96fcd71540f22fe37d660ea.
Semantic state:
semantic_state_d10ac03835ca364518838b7c29ad2bd8dd69b9ecb6ec269b9fd84c5b8ff54b9b.
The complete LKJSG001 carrier is 6,793,590 bytes, SHA-256
c387db4db5404258a9d6026c01f27e9c140c0002d8456ebdeb889f3ac38add7f.
These are existing carrier observations, not new runtime-memory measurements.

The preceding physical GC retained 200,012 objects in 13 packs / 334,241,707 bytes.
Its observed modeled retained bytes were 467,387,305; that metric now has no
512 MiB admission ceiling. HEAD and source identities remain unchanged here.
The [preceding decision](decisions/20261021-source-effect-owners.md) binds those
carrier, artifact and storage measurements to their exact baseline.

## Verification and next work

The focused GC suite passed all 14 tests, including actual 528 MiB retention,
more than 4 GiB of reads and 1,000,001 distinct queued references. Negative
controls retain exact object verification, explicit read limits, cancellation,
reader leases and all compaction durability checkpoints. Expanded receipt tests
retain every gate status and excerpt beyond the former 64 KiB ceiling.
All seven surface-audit tests pass, including long guidance with a late forbidden
text control beyond the former editorial byte and word ceilings.

Publication requires exact staged-tree bootstrap, changed selection, every gate
in full --fresh and release-target policy inspection. Terminal evidence is in
.artifacts/lkjscript-dev/latest-receipt.json; focused evidence is under
.artifacts/selfhost/resource-ceilings and .artifacts/selfhost. Git records normal
integration. No public product release or deployment is implied.

WS1 remains incomplete: kernel work/analysis limits, authoring/inspection quotas,
recursive depth limits, native stdin/value bounds, LKJSG001 synthetic fuel and
size/table bounds, and coupled LKJCI1/.lkja widths/layouts remain next work.
Trusted runtime frame/stack policies need an iterative reference-evaluator design
before removal. Object/pack/catalog encoding bounds remain representation work.
General verification inventory, file and command-output bounds remain separate work.

This is neither E1-final nor Tier 2. Full Tier 2 still requires source-driven E2/E3,
the declared region/closure/witness profile, source-driven C2=C3 and A1=A2=A3,
behavioral mutations, independent full-profile programs and measured same-process
native reclamation. [Verification](spec/verification.md) owns final acceptance.
