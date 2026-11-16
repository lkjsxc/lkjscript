# Ordinary-store payload attribution

The 100k one-body build spends **797.295 ms** in the existing `store_io`
phase, of which **793.740498 ms** is assigned to disjoint category intervals
(load **11.89→11.13**). The native fragment pack accounts for **54.06%**
of assigned time, followed by the frontend checkpoint at **32.88%**.
This identifies the largest current ordinary-store payload cost; it is **not**
an attribution of the older 5.667 s result after indexing.

All **24 command observations** succeeded. Emitted cold, unchanged, changed-body
and no-store control images agree exactly between the two producers. The six
instrumented stored-build reports have zero global timing/overflow flags and no
pending checkpoint, with exact category-phase sums. All rows, including setup,
transactions, the differing host loads and the faster instrumented observations,
remain in the [timing table](compiler-cache-io.tsv). There was no timing retry.

## Scope and pins

The [predeclared protocol](compiler-cache-io.protocol.md) and
[harness](compiler-cache-io.sh) use one worker, an external 4 GiB address-space
limit, 900 s per command and a fixed 1 GiB disposable-store capacity. Both
producers were constructed at inline64 by the same copied emitter; ordinary
outputs explicitly use inline0. Physical library/runtime selections and the
generated source path are identical within each producer pair. No filesystem
cache reset is claimed.

| Role | SHA-256 | Bytes |
| --- | --- | ---: |
| Common emitter | `71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7` | 31,564,397 |
| Baseline | `b3b2ab660ccf02d8892903f5323c4a8d13fcc54526c39f6d3bce71b0bc15c1d1` | 31,574,840 |
| Instrumented | `82d85b1a16818c391f194f0f956a32d077a6c9639452841c86715d04c07c68f7` | 31,727,173 |

The source census is frozen at `b7e8ed035d155e75438406c7474a747220ff9537`,
src tree `68b8a295a2c8671c3a574ddd71e7c5b15ce80430`, library tree
`25bdb8ff8567b941790d3951c78e896b23e26e8e`. Census digest:
`50e65ac7f82785b37781379e111196474a5eb2ff3faef863d1e50607ba5eadf4`.
The instrumented producer differs only in `src/store.lkj` and driver stats
reporting: final file hashes `9231ed34c66dbfe554045b520435418913a41b90280c62a42f6d4d14bebca3d4`
and `0b9a7784c025264a8b63d5101cb1089e9567a4e735a19e6c014aac4f6088c1e1`.
Construction commands and costs are separate in the
[setup evidence](compiler-cache-io.setup.log).

The actual main-root census contains 88,488 lines, 3,634,522 bytes, 171 files,
3,425 functions, 50,622 source call sites, 143 generic functions, 2,197 nominal
fields, 1,688 type edges and maximum module depth three. Seed 20261116 produces:

| Requested lines | Source bytes | Files | Functions | Generic functions | Call sites | Nominal types / fields / variants / edges |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| 10,000 | 380,657 | 20 | 335 | 17 | 4,581 | 19 / 95 / 7 / 68 |
| 100,000 | 3,921,355 | 194 | 4,081 | 177 | 48,280 | 222 / 799 / 333 / 462 |

Both bodies have eight direct signature dependents. The 100k checked program has
4,568 functions and 897,048 IR instructions. These are a new fixed cohort, not a
replacement of the earlier ladder's source/producers or observations.

## Fixed sequence and timings

For each rung, baseline runs before instrumented. Each sequence is cold,
unchanged, checked body transaction, one-body build and no-store body control.
The transaction always uses the common emitter and its own fresh **operation**
store. The ordinary build store remains unchanged during that transaction.
The `store_scope` column distinguishes ordinary, operation and no-store rows.

Wall below is launch-inclusive. CPU is GNU user plus kernel time; RSS is KiB.
The complete three-component paired loads and all preparation/transaction rows
are retained in the TSV.

| Lines | Producer | Phase | Wall s | CPU s | Peak RSS KiB | Load before→after |
| --- | --- | --- | ---: | ---: | ---: | --- |
| 10000 | baseline | cold | 2.425551 | 2.22 | 106384 | 12.77→17.76 |
| 10000 | baseline | unchanged | 0.476339 | 0.42 | 4508 | 17.76→17.76 |
| 10000 | baseline | one_body | 0.925941 | 0.89 | 93196 | 20.10→20.10 |
| 10000 | baseline | body_control | 1.094156 | 1.05 | 99988 | 20.10→20.10 |
| 10000 | instrumented | cold | 1.827755 | 1.70 | 110796 | 20.10→19.53 |
| 10000 | instrumented | unchanged | 0.340676 | 0.32 | 7232 | 19.53→19.53 |
| 10000 | instrumented | one_body | 0.910367 | 0.89 | 99596 | 19.53→19.53 |
| 10000 | instrumented | body_control | 1.103091 | 1.07 | 107500 | 19.53→18.53 |
| 100000 | baseline | cold | 14.991685 | 14.25 | 702808 | 18.53→17.43 |
| 100000 | baseline | unchanged | 0.576529 | 0.53 | 14932 | 17.43→17.43 |
| 100000 | baseline | one_body | 7.237840 | 7.12 | 572784 | 15.93→15.86 |
| 100000 | baseline | body_control | 10.774605 | 10.64 | 672908 | 15.86→14.50 |
| 100000 | instrumented | cold | 12.309282 | 12.01 | 712848 | 14.50→13.07 |
| 100000 | instrumented | unchanged | 0.437658 | 0.42 | 20868 | 13.07→13.07 |
| 100000 | instrumented | one_body | 5.836589 | 5.73 | 582948 | 11.89→11.13 |
| 100000 | instrumented | body_control | 10.351453 | 9.99 | 686444 | 11.13→10.87 |

The 100k instrumented body row restores 5,064 frontend definitions, reconstructs
one definition/function, reuses 4,567 checked bodies and 4,562 native functions.
Its phase durations include 1.444 ms changed IR construction, 768.199 ms
frontend restoration, 1,073.124 ms key/hash work and 1,670.440 ms load/parse,
beside the 797.295 ms store phase (all under load 11.89→11.13).
The large nonlocal costs remain outside this store attribution.

The instrumented executable is 152,333 B larger (0.48245%). It was constructed
in 46.38 s at load 12.87→13.94; baseline construction took 33.50 s at
4.32→5.81. Those observations use different host conditions. The paired program
rows also show variation, including lower instrumented wall/CPU values. They do
**not** isolate a causal instrumentation overhead or establish a speedup.
This method adds optional clocks/counters and changes producer bytes; no compiler
optimization or bootstrap-policy change is claimed.

## 100k one-body store breakdown

All durations in this section share load **11.89→11.13**. Bytes below are
accepted authenticated payload bytes or successful publication payload bytes,
not physical disk/syscall traffic or unique retained storage. “Sealed” combines
standalone sealing and the separately recorded packed resealing payload totals.

| Category | Exclusive ms | Share of accounted time | Authenticated read B | Sealed/resealed B | Published B |
| --- | ---: | ---: | ---: | ---: | ---: |
| native | 429.122332 | 54.06% | 10431971 | 10431971 | 10431971 |
| front | 260.966406 | 32.88% | 11859406 | 11859406 | 11859406 |
| image | 53.909510 | 6.79% | 0 | 4636714 | 4636714 |
| check | 45.421407 | 5.72% | 406568 | 406568 | 406568 |
| inputs-image | 3.764475 | 0.47% | 121035 | 121035 | 121035 |
| metadata | 0.556368 | 0.07% | 1284 | 1507 | 1507 |

The native pack's **429.122332 ms** consists of read/authentication 117.482514,
decode 98.099973, retain 61.889687, serialization/reseal 150.126029 and
publication/preparation 1.524129 ms. One 10,431,971 B payload is loaded and fully
resealed despite only a small body change.

The frontend checkpoint's **260.966406 ms** consists principally of
121.685184 ms read/authentication and 137.302099 ms sealing, with 1.979123 ms
publication/preparation. It reads and republishes one 11,859,406 B payload.
Frontend encoding itself is outside these counters and appears separately as
101.472 ms in the driver phases.

The check pack uses **45.421407 ms** for a 406,568 B payload. Of that,
14.470248 ms decodes it, 19.134244 ms retains the requested keys and
7.293153 ms serializes/reseals it. The remaining read/publication work is retained
in the [category table](compiler-cache-io.io.tsv).

Budget metadata uses only **0.556368 ms** in this condition. It authenticates
1,284 B, seals 1,507 B and scans **five complete headers / 450 B**. Complete
header inspection is not payload authentication. The ordinary store grows from
27,457,518 B after cold construction to 32,094,545 B after the body change.

The category sum is exactly **793,740,498 ns**, matching the once-only summary.
The driver reports 797,295,000 ns, leaving **3,554,502 ns (0.44582%)** outside
these timed low-level intervals. The residual includes surrounding caller work,
counter/reporting boundaries and driver phase rounding; it is not silently
assigned to a payload.

## Counter contract and verification

`Store::enable_io_stats()` opts in; `Store::io_report()` returns the bounded
machine-readable rows appended after the unchanged `LKJSCRIPT_STATS` row.
Normal output remains unchanged. Namespace families are fixed; the report
contains no raw source, key, payload, physical path or namespace digest.

The once-only summary carries `global_timing_failures`,
`global_counter_overflow`, `pending_checkpoint` and `accounted_ns`.
Per-category duration fields are disjoint: an outer completed interval subtracts
already recorded nested durations. Missing/backward/inconsistent readings flag
an incomplete interval. Counter addition saturates with a flag instead of
introducing an arithmetic trap.

Read attempts include failed opens. Accepted-byte counters exclude partial or
failed authentication reads. Pack-decode bytes count authenticated bytes offered
to decoding, even if decoding subsequently refuses; pack-load count requires
success. Budget-header bytes count complete scanned headers before their validity
check. Publication duration includes temporary file work, rename and cleanup.
Producer/key setup, source reads, cursor HMAC work and historical `QueryReader`
access are outside this ordinary-record instrumentation.

Reporting stops at the driver's explicit finish/report point. A failed dirty
checkpoint may be retried by Drop afterward; `pending_checkpoint=1` marks that
incomplete exit-path account. Drop behavior was not changed. All six measured
stored-build reports have zero timing/overflow flags and pending checkpoint zero.
The [validation table](compiler-cache-io.checks.tsv) preserves every row; the
[raw parsed facts](compiler-cache-io.jsonl) retain the recorded numbers.

The focused tests passed in **4.19 s**, load **11.65→11.52**: deterministic
nested arithmetic and failure/overflow cases through native and the original
interpreter; exact cold publication bytes; unchanged image reads; pack
load/reseal counts; total phase accounting; byte-identical outputs; quiet default
output. The first fixture attempt used an incorrect `std::hash::sha256` import
and refused before the controls ran (0.12 s, load12.40→12.40). It was corrected
to the existing `std::sha256` export; no production change was needed.

The existing store attack suite initially stopped at a diagnostic comparison
because it removed only `LKJSCRIPT_STATS` lines (15.30 s, load10.68→10.24).
After extending that one filter to the new declared row prefix, the complete
suite passed: **49.08 s GNU / 49.09 s launch wall**, load **6.86→7.11**,
including hostile entries, rejection, unavailable storage, exact byte
reproduction and concurrent publication. A temporary second worker was allocated
for that existing suite, then released before this one-worker campaign.

Every retained production-source change was applied through checked operations:
**642/642 added/deleted .lkj lines**, 46 successful operations in requests
2/4/5/6. Requests1/3 refused unsupported source conveniences and left both files
unchanged. Independent review checked the request/apply hashes and complete
parent-to-final file chain; the [operation manifest](compiler-cache-io.operations.tsv)
records the exact evidence identities.

## What this selects, and what it does not

For this ordinary-store condition, the first large payload worth a bounded
transport experiment is the **native fragment pack**: 54.06% of assigned store
time, with full read/decode/retain/reseal for 10.43 MB. The frontend checkpoint is
the second large payload. This does not justify weakening graph/producer
identities or storing one tiny file per body: batched authenticated pages must
avoid turning record-local reuse into many filesystem opens. The current graph
identity and numeric native label dependencies remain until separately repaired.

The historical 100k **5.667465 s** store phase was measured **after an indexed
build**, in a store of **613,720,812 B**, with load6.87→7.20. Its preceding
index publication recorded 70,048 writes. `Store::writable_inner` lists the
store and opens every indexed entry's header during reconciliation. The present
ordinary row has only five scanned headers and cannot explain that indexed
population. A separately declared A/B experiment will compare the same producer,
generated path and changed source with ordinary versus index-populated stores.
Index preparation must be attributed against `semantic_index` as appropriate;
its store counters are not required to fit only the driver's `store_io` phase.

No page migration, >10% performance step, historical timing replacement,
million-line retry or backend floor is established here. All generated ordinary
trees, stores, per-run keys and outputs were removed by the harness and absence
verified. The three frozen producers are intentionally retained for the approved
separate indexed-condition experiment.
