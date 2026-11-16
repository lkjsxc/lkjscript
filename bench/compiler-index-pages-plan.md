# Lazy semantic-index page experiment: declared controls

The baseline is source 71e3c9637aab74fdb860dbe1d5ff2ab9631b2f31,
producer 71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7.
The candidate changes only semantic_store::Entry, node and publish_emitted
through request 3f2f690873776d7658892d46a690f4a826cf50ebacb8e03f4e205a8a36aeb6cd:
producer 40464dda053795b93dbba1af93e41633b915b2b625b0007a7b95ec4d22f0f200.
No source-format, authentication, query-output, checker or admission change is proposed.

The old algorithm consumes each fact vector while accumulating serialized page
values. The candidate instead retains every sorted, deduplicated fact vector and
keeps only page range descriptors; it serializes logical values at radix leaves.
This trades allocations/lifetimes. It does not remove two simultaneously live
complete collections, and no peak-RSS improvement is assumed. Global facts,
descriptors, catalog and checked IR remain. The candidate's live *encoded logical
value* is at most 16,384 bytes, alongside a radix-leaf buffer at most 32,768 bytes;
these are payload bounds, not allocator/RSS bounds or a bound on all index state.

## Correctness before timing

A test-only copy of the old eager publisher runs in the same native executable
as the candidate publisher. It compares exact roots, snapshot identity/provenance
and complete query replies including authenticated continuation cursors. It covers
empty logical records, empty strings, a maximum admitted single fact, sixteen
multibyte lines, radix branching and long UTF-8 signature/text pagination.
There is no old-algorithm branch in production.

## Fixed measurement sequence

One worker; external 4 GiB address-space ceiling and 900-second per-command
watchdog; 2 GiB authenticated-store budget per producer. Check free space before
materializing snapshots or stores. Source shape is frozen at
0b30ca8987bd4ba343ab45ab01751b6721c7151a; generator seed 20261116.
The expected shape SHA-256 is
0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c.
Generated 10,000- and 100,000-line trees are byte-identical inputs for both producers.
An explicit shared frozen library/runtime is supplied. Ordinary emitted code uses
--inline-limit 0. No 1M rung or extrapolated allocation experiment is included.

For each rung, in increasing order, execute exactly:

1. Baseline ordinary cold build in its empty store.
2. Baseline indexed build in that warmed ordinary store.
3. Candidate ordinary cold build in its separate empty store.
4. Candidate indexed build in that warmed ordinary store.
5. Compare emitted image bytes and authenticate/compare every rooted semantic-page
   payload under the two producers, including logical records, counts and bounds.
6. Query ten declared questions once with each producer at the explicit resulting
   snapshot: outline edit; signature, text, slice, uses, callers, instances,
   authority and type edit::target; layout u64. Depth 1, limit 96, byte limit
   32,768. Require no continuation for this small named subject. Normalize only
   each independently verified producer and snapshot field in protocol headers;
   every other stdout byte must match. The same-producer oracle separately
   requires exact unnormalized continuation replies.

Record wall, user/kernel CPU, peak RSS and load beside each timed command, source,
producer and image hashes, all stdout/stderr, stats, store bytes, authenticated
rooted page payload/envelope totals, exact logical-value/fact bytes by kind,
pagination counts and full comparison outcomes. After each indexed build, also census
all semantic-page/root/latest/locator files and the .budget-index by file count,
stored/envelope payload bytes and first-90-byte header reads. This static census
is outside timed commands, makes no authentication claim, and does not change
the declared build/query order. Authenticated byte totals are
retained-payload measurements from the post-build oracle, not a claim about how
many bytes the build dynamically authenticated. Query checks are correctness work,
not a new latency campaign.

Keep every setup failure, refusal and regression. Do not replace observations
with a retry or choose a best run. A setup repair is a separately named attempt.
Retain the owned fixture/store/key until the report is accepted, then remove them;
keep intentional text evidence and operation hash chains. These measurements are
a distinct current-producer experiment and do not replace the original 54db01f5
100k observation or diagnose its older 1M refusal.
