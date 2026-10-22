# GC preparation without default quotas

Date: 2026-10-22. Baseline: 9eaaad620b047e1b9170efb5bb00e93a4fc9fc86.
Scope: the first WS1 slice; GC preparation and verification receipt capacity.

## Decision

Remove default aggregate GC gates on 512 MiB modeled retention, 1,000,000
retained objects/map entries/queued references, 16,000,000 canonical reads and
4 GiB read bytes. Ordinary reads no longer construct an inner preparation
allowance. Explicit caller-supplied StoreReadAdmission remains a pre-consumption
policy and is forwarded unchanged to the owning store.

No maximum-integer sentinel replaces these quotas. Retention buffers use
try_reserve_exact on actual payload lengths. Checked object framing, domain and
digest verification, reference-binding consistency, complete map consumption,
reader exclusion and atomic catalog publication are unchanged.

The reference queue records each object identity before enqueueing it. Repeated
identical references do not enqueue again; conflicting bindings reject. Each
queued reference is removed once and complete map iteration checks the declared
entry count. Cancellation checkpoints remain between references, required object
reads and map entries. These concrete structures justify removal of synthetic
preparation limits; no blanket complexity claim is made for unrelated passes.

## Observation contract

GC receipts now include preparation.retained_modeled_bytes, catalog_lookups,
objects_read, bytes_read and references_visited. Modeled retention is canonical
payload plus 256 bytes per retained object. All counters saturate at u64::MAX;
a saturated value is a lower bound. None controls acceptance, sizes buffers,
computes offsets or enters a canonical object digest.

These are cumulative work and modeled-retention observations. They are not peak
live allocations, retained arena capacity, mapped OS bytes or sampled RSS. This
slice makes no native-memory reclamation or bounded-RSS claim.

GC remains physical compaction. Deterministic 32 MiB target partitions remain
physical output policy, with larger individual objects in singleton partitions.
It preserves complete immutable ancestry, receipts, transactions, witness maps,
current retirements, HEAD and canonical source identity. No project reseed or
history/tombstone pruning was performed. Existing object-domain, pack and catalog
encoding limits require separate representation changes.

## Complete verification receipts

Remove the 64 KiB aggregate compact-receipt ceiling, its size-triggered excerpt
deletion, the shared evidence writer's 128 MiB encoding/publication ceiling, and
the matching archived-evidence reader ceiling. A larger gate registry can publish
and read every outcome and identity without
failing receipt creation. Per-excerpt presentation truncation remains; full
originals still require explicit archive retention. Joined scratch cleanup and
one replaceable latest receipt are unchanged.

Cargo inventory reads are bounded by their observed, verified file length and
rehash the exact consumed bytes. This preserves file integrity without a fixed
aggregate size cap. Status guidance now requires concise, complete reporting
without hard line-count, 8,000-byte or 1,000-word editorial rules. The product-surface
audit still checks forbidden text, expected product commands and exact output
protocols; a long-guidance regression admits all three guidance owners and rejects
a forbidden suffix after the former threshold. General audit file, directory and
command-output bounds remain separate verification-infrastructure work.

## Independent controls and acceptance

The focused GC suite passed 14 tests. The generated immutable-object source
retains 33 distinct 16 MiB payloads (528 MiB actual payload) without duplicating
the whole source fixture. A separate repeated-read case consumes 257 times
16 MiB, beyond the former 4 GiB cumulative cap, while retaining one object.
A queue case inserts 1,000,001 distinct references twice and checks one queued
visit per identity. Saturated observation counters still permit valid work and
do not mask cancellation.

Explicit exact-fit/read-refusal tests, canonical corruption, missing references,
reader leases, historical receipts/idempotency, inferred compiler types and every
physical durability checkpoint remain independent negative/atomicity controls.
The receipt matrix contains 120 mixed terminal outcomes and exceeds 64 KiB;
published statuses and excerpts remain complete and latest-receipt replacement
still works. A canonical JSON encode/publish/streamed-digest/read round trip crosses
128 MiB; appended noncanonical whitespace still rejects. Archived original reads
use the actual regular-file extent and retain complete canonical re-encoding checks.
Focused logs and the source-bound full receipt are under the paths
recorded in docs/status.md. Publication is gated by the final staged tree,
changed selection, fresh full registry and release-target policy inspection.

No compiler proposal or accepted project object changed. The existing Tier 1
fixed points remain distinct from the unfinished source-driven Tier 2 contract.
Next work removes trusted kernel/runtime policy defaults and then repairs the
coupled carrier encodings and recursive traversals as atomic verified slices.
