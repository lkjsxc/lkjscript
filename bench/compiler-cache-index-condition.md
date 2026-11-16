# Indexed-store bookkeeping attribution

The 100k ordinary body build against the index-populated store spends
**7,936.027 ms** in the driver Store phase, with **6,903.465554 ms**
assigned to budget metadata: **87.04%** of all accounted Store time
(load **8.67→9.38**). The same body using the copied store's ordinary-only
predecessor spends **975.983 ms** in that phase, including **0.687305 ms**
of metadata (load **8.79→8.67**).

Both conditions reuse the same checked/native bodies, read byte-identical
ordinary payloads and emit the exact same image as the no-store control.
The populated store makes the writer inspect **70,173 complete headers**
instead of **five**, and authenticate/reseal a **16.21 MB budget payload**
instead of about **1.3/1.5 KB**. This identifies a global bookkeeping wall
that must be addressed before ordinary-payload paging can make indexed edits
scale with the change. It does not measure a prospective paging speedup.

All **18 attempted commands** succeeded, with no retry or changed resource
limit. Every stored-row accounting check passed. The full
[timing table](compiler-cache-index-condition.tsv),
[counter table](compiler-cache-index-condition.io.tsv),
[validation rows](compiler-cache-index-condition.checks.tsv) and
[raw parsed facts](compiler-cache-index-condition.jsonl) remain separate from
the earlier [ordinary-only campaign](compiler-cache-io.md).

## Fixed condition and provenance

The [protocol](compiler-cache-index-condition.protocol.md) and
[harness](compiler-cache-index-condition.sh) were frozen before launching.
One immutable instrumented producer was used throughout:

- Instrumented producer: 82d85b1a16818c391f194f0f956a32d077a6c9639452841c86715d04c07c68f7,
  31,727,173 B.
- Common generator/edit emitter: 71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7.
- Census, generator and physical library/runtime source:
  b7e8ed035d155e75438406c7474a747220ff9537.
- Src/library trees: 68b8a295a2c8671c3a574ddd71e7c5b15ce80430 /
  25bdb8ff8567b941790d3951c78e896b23e26e8e.
- Generator/census digests, exact command arguments, snapshot/source/IR pins and
  all output identities are retained in the [metadata](compiler-cache-index-condition.meta),
  [arguments](compiler-cache-index-condition.arguments.log),
  [snapshot rows](compiler-cache-index-condition.snapshots.log) and
  [parity rows](compiler-cache-index-condition.parity.tsv).

Each rung uses one generated source path, one private key, seed 20261116 and
output inline0. The relative generated-source manifests match the earlier
ordinary campaign. The 100k pristine/changed digests are
011ab15bdd9601683ad8d942db9710e98139f4c5c94076c82cc4fb67ec67fdb9 and
032214dd903ec6727571107502412dbf6ccbf9760473d9ba5d89d961e53b9dc4.
Archive/output paths differ between campaigns; no cross-campaign emitted-byte
identity is claimed.

The sequence is ordinary cold A, closed copy A→B, index only B, a checked body
transaction through its own fresh operation store, body A, body B, then no-store
body control. Capacity is **2 GiB** in both stores, distinct from the prior
ordinary campaign's 1 GiB. All commands use one worker, external **4 GiB**
address-space and **900 s** limits. Free-space preflight passed before setup and
both rungs.

Five preexisting ordinary entries were hashed in both stores after copying and
again after indexing. All **20 entry comparisons** were exact; the
[entry table](compiler-cache-index-condition.entry-parity.tsv) preserves each.
This equally prewarms that small payload set. It does not read B's added semantic
payloads. Index population, copying, source publication and the fixed A-before-B
order still affect caches; no cache reset or randomized causal comparison is
claimed. Complete store census was taken only after the final timed control.

## Body observations

Wall is launch-inclusive; CPU is GNU user plus kernel time, RSS is KiB.
Each table row retains its paired one-minute load; the TSV has all three load
components and the separate GNU wall observation.

| Lines | Condition | Wall s | CPU s | Peak RSS KiB | Load before→after |
| --- | --- | ---: | ---: | ---: | --- |
| 10000 | body_A | 1.418736 | 1.32 | 94036 | 8.61→8.61 |
| 10000 | body_B | 1.378346 | 1.31 | 97944 | 8.61→8.61 |
| 10000 | body_control | 1.166645 | 1.15 | 102720 | 8.61→9.12 |
| 100000 | body_A | 7.315242 | 7.13 | 578728 | 8.79→8.67 |
| 100000 | body_B | 14.440263 | 14.04 | 624452 | 8.67→9.38 |
| 100000 | body_control | 13.535155 | 13.31 | 688100 | 9.38→9.49 |

The 10k B wall happens to be slightly lower than A despite its larger Store
phase: other phases differ in this fixed-order observation. No result is removed
to make total wall follow a proposed explanation.

At 100k, both A and B restore 5,064 frontend definitions, reconstruct one
definition/function, reuse 4,567 checked bodies and 4,562 native functions.
They contain 4,568 IR functions / 897,048 instructions and emit 4,637,392 B.
The body A, B and no-store output digest is
47b25313468483e0b1cf3a9843e2c6f8b9ce66559be59ad45779ba5f064fa139.

## Store attribution

| Lines | Condition | Driver Store ms | Accounted ms | Metadata ms | Complete headers | Header bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 10,000 | A | 128.709 | 127.320708 | 0.615066 | 5 | 450 |
| 10,000 | B | 469.036 | 467.939774 | 388.185257 | 6,287 | 565,830 |
| 100,000 | A | 975.983 | 970.316525 | 0.687305 | 5 | 450 |
| 100,000 | B | 7,936.027 | 7,931.469763 | 6,903.465554 | 70,173 | 6,315,570 |

The 10k rows share load 8.61→8.61. The 100k A/B rows use
8.79→8.67 and 8.67→9.38 respectively.

The complete 100k category comparison is:

| Category | A exclusive ms | B exclusive ms | B share of accounted time |
| --- | ---: | ---: | ---: |
| metadata | 0.687305 | 6903.465554 | 87.04% |
| native | 498.760636 | 593.016112 | 7.48% |
| front | 335.610608 | 305.435594 | 3.85% |
| image | 87.717493 | 62.625201 | 0.79% |
| check | 44.481797 | 62.539701 | 0.79% |
| inputs-image | 3.058686 | 4.387601 | 0.06% |

B's metadata row is disjoint:

| Metadata work | Milliseconds |
| --- | ---: |
| Authenticated budget read | 258.552211 |
| Budget sealing | 246.773970 |
| Budget publication | 7.609384 |
| Remaining metadata work | 6390.529989 |
| Total | 6903.465554 |

All B metadata durations above share load 8.67→9.38. The authenticated read is
16,210,029 payload bytes; the sealed/published payload is 16,210,252 B.
A reads 1,284 B and seals/publishes 1,507 B. The additional 223 B corresponds
to the changed image entry's budget growth in both conditions.

The remaining metadata interval includes budget decode/encode, global directory
reconciliation, entry/header validation, ordered-map/set work and other existing
budget bookkeeping. It does **not** separately attribute header syscalls versus
those in-memory operations. Header bytes are complete headers inspected before
validity checking, not authenticated payload bytes or total physical I/O.

Ordinary read/reseal payload sizes remain 10,431,971 B for native, 11,860,087 B
for front and 406,568 B for check in both A and B. The ordinary body builds do not
read semantic payloads through the semantic namespace; their collateral cost is
the shared budget/header path.

B's category sum exactly matches its summary: 7,931,469,763 ns. The driver phase
is 7,936,027,000 ns, leaving **4,557,237 ns / 0.05742%** outside these low-level
intervals. A's corresponding residual is 5,666,475 ns. All global timing failures,
overflows and pending-checkpoint flags are zero. Reports cover the explicit
finish/report point; no later failed-checkpoint retry is involved.

## Preparation, bounds and census

Preparation timings are retained independently of body A/B:

| Lines | Preparation | Wall s | CPU s | Peak RSS KiB | Load before→after |
| --- | --- | ---: | ---: | ---: | --- |
| 10000 | cold_A | 1.505901 | 1.45 | 109684 | 8.28→8.28 |
| 10000 | copy_A_to_B | 0.019262 | 0.00 | 2240 | 8.28→8.28 |
| 10000 | populate_B | 14.632455 | 14.26 | 386380 | 8.28→8.57 |
| 10000 | body_transaction | 2.779299 | 2.71 | 156896 | 8.57→8.61 |
| 100000 | cold_A | 13.818244 | 13.55 | 700656 | 9.12→9.61 |
| 100000 | copy_A_to_B | 0.017756 | 0.00 | 2240 | 9.61→9.61 |
| 100000 | populate_B | 131.053927 | 127.19 | 2736864 | 9.61→8.98 |
| 100000 | body_transaction | 21.736340 | 21.11 | 1026604 | 8.98→8.79 |

The 100k index preparation records 33,167.726 ms semantic catalog and
86,696.471 ms semantic index work, with driver Store 717.160 ms, under
load 9.61→8.98. Its Store counters total 18,299.667679 ms. They are correctly
validated against combined Store plus semantic-index coverage
(87,413.631 ms), because index publication occurs inside semantic_index.
The remaining combined time is not labelled unaccounted Store I/O.

The predeclared prediction from 10k admitted the 100k rung:

| Guard | Prediction | Limit | Actual 100k observation |
| --- | ---: | ---: | ---: |
| Peak RSS KiB | 3,346,267 | 3,670,016 | 2,736,864 |
| Index launch wall ms | 219,487 | 900,000 | 131,053.927 |
| Post-index store bytes | 1,008,252,960 | 2,147,483,648 | 655,977,514 |

The prediction is an extrapolation with stated margins, not a measurement or
proof that RSS bounds virtual address space. The actual 4 GiB cap remained in
force. No guard or command refused. The [prediction row](compiler-cache-index-condition.predictions.tsv)
is unchanged.

After the body build, A is 32,098,616 B and B is 660,615,219 B in the 100k rung.
B retains 70,167 semantic-family files / 612,184,699 file bytes; A has none.
The final signed budget envelope is 16,210,342 B in B versus 1,597 B in A.
The [census](compiler-cache-index-condition.census.tsv) retains file totals rather
than inferring physical disk traffic from payload counts.

One auxiliary census label is deliberately left as observed: its filename
classifier recognizes inputs-index, while the actual namespace is
inputs-semantic (mapped to inputs-index by the production counters). Consequently
one 29,445 B / 123,159 B envelope appears as other at 10k / 100k. The raw census
is not silently relabelled. This does not affect production counters, Store
timing, total bytes or image controls.

## Next scope and limits

The first indexed-edit target is **global budget reconciliation and checkpoint
transport**, not merely the native pack that dominates the ordinary-only
condition. A bounded authenticated accounting design must retain capacity
semantics, hostile-entry validation, concurrent publication and crash recovery;
these measurements do not authorize skipping those checks. It should avoid
opening every unrelated semantic page and decoding/resealing one whole budget
for a local write. The present counters do not decide between sharded accounting,
a recoverable delta design or another bounded representation.

Native/front payload paging remains a separate ordinary-store opportunity.
It must preserve producer/graph identities and avoid replacing one pack with
one filesystem open per body. No page implementation or ≥10% optimization step
is claimed by this report.

The old 5.667465 s indexed-store row remains historical: it used another
producer, source census, paths, capacity and load. This new fixed A/B condition
exposes the same global dependency by direct counters; it does not retroactively
partition that old observation. The concurrent no-create-read repair is also
outside this frozen producer and is not credited with any result here.

All generated trees, stores, per-run private keys and images were removed by the
harness, and run-directory absence was independently verified. The copied
emitter/baseline/instrumented producers are retained temporarily for root's
evidence audit. This follow-up changes no production source.
