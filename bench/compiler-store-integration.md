# Store integration: accepted gate and retained fixture failures

The corrected coherent gate passes from a fresh empty store with 5,448 fresh
case requests and zero case reuse. The source/compiler changes, both integration
fixture repairs, accepted run and first failed run are retained separately below.

The first coherent empty-store gate exposed two stale test assumptions after
Store I/O attribution and no-create cache reads. The compiler's source and
library trees did not need a semantic change for either finding.

The failed run is retained under [compiler-store-gate-data](compiler-store-gate-data/).
It used source `4f8c213559b7de96b3b6bd6199da661dfc95f778`, a new owned store,
a private 32-byte key outside the source grant, four workers, a 4 GiB
address-space limit and a 2,400-second watchdog. It exited 1 after
679.92 s (load 6.81 → 13.07), with 1,756.52 user seconds, 33.26 system seconds
and peak RSS 1,219,464 KiB. Its log SHA-256 is
`c2cc2a17de8133aae6d9691851ad3b75aefe306430529066ab466bc16940890c`.
The final selection summary contains 5,352 requests from successful suites and
zero reuse; the failing toolchain's requests are not included in that summary.
The final IR-mutation step was not run. This run is a refusal, not a passed gate.

The additional page suite passed 70 exact replies and rooted payload parity
in 21.877 s (load 5.60 → 7.05). Store I/O validation passed in 4.008 s
(8.71 → 9.21), and the 25 no-create transport cases passed in 1.063 s
(9.21 → 9.21). The failure therefore appeared in integration consumers that
the focused suites had not covered.

## Statistics consumer

`tools/toolchain-test.lkj::run_values` treated every stderr line except
`LKJSCRIPT_STATS` as an unexpected diagnostic. The newly documented
`LKJSCRIPT_STORE_IO` records are optional statistics emitted by the same
`--stats` request. The repair forwards that second exact prefix and still
requires exactly one legacy statistics record. Other nonempty diagnostic lines
continue to fail the test.

One `replace-item` operation targeted `run_values`.
The [request](compiler-store-gate-data/toolchain.request.json) has SHA-256
`0b0686f436ccdd5bc958e1f9264ad3ce36cbcc4c312e424aac0ffe0d171d2c41`.
The applied file hash is
`aa8e648da2a9492af31f980871ee7dedbaccf448907d81e50b4f13024775bc4d`
→ `5368c5a334f7829239ca9059bcb1efb526ac023dd1af64c2791136ea081af8a3`.
All 1,206 tracked language files are covered by the retained before/after
manifests; only this file differs. Independent read-only reconstruction from
the baseline plus the literal request reproduces the whole resulting file.

The operation reports one site/file/identity, no authority delta, 424 freshly
checked instances, zero reused proofs and zero frontend resume, with demanded
library validation. It passed in 2.07 s (load 7.74 → 7.74), peak RSS
98,324 KiB. The complete retained delta is **two added lines, both credited**.

The rebuilt toolchain suite then passed in 70.54 s (load 8.18 → 9.92),
66.12 user seconds, 1.96 system seconds and peak RSS 61,796 KiB.
Its fresh fixture, store and private key were removed. Full stdout/stderr,
exit, GNU time and load observations remain in the data directory.

## Writer-lock fixture

The old closed-runner fixture required an absent proof lookup to create
`.budget-lock`. That was a consequence of the removed create-on-miss read,
not the lock-release contract. The actual requirement is that a corrupt,
opened regular proof can enter cleanup accounting, but its writer lock is
released before executing the closed child. A truly absent read should create
no store files. The dedicated lock-fixture repair preserves both cases and the
existing failed-child, busy-writer and serial-success controls; its focused
evidence is recorded in [the lock-fixture analysis](compiler-gate-reuse-lock.md),
with the raw observations also copied into [compiler-store-lock-data](compiler-store-lock-data/).

## Independent witness and producer identity

The fixed-point executable is 31,716,730 bytes, SHA-256
`adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7`.
The first gate's bootstrap took 76.544 s (load 6.81 → 5.92) and obtained exact
current-emitter reproduction.

The optional Rust witness passed in 79.17 s (load 9.80 → 8.22), 76.49 user
seconds, 4.55 system seconds and peak RSS 1,354,844 KiB. It checked 276
value/output cases, 344 verdicts, exact compiler bytes, and both Directory
suites across native, interpreter and Rust witness routes. The witness log
SHA-256 is `76817c9f8cebd3219873d664ca31cc52c10e7ea1d3cf396f199cf69051323311`.

## Accepted cold gate and seed

The corrected gate ran at `a0da7d7fd98e0897856d7c94fe5622f949509c7f`
from 2026-11-16 07:40:40 to 07:51:27 UTC. It used a new owned store,
a private 32-byte key outside source/library read grants, four workers, a 4 GiB
address-space limit and a 2,400-second watchdog. It passed in **647.122 s
(load 6.32 → 6.06)**; GNU time reports 647.12 wall seconds, 1,623.67 user
seconds, 32.18 system seconds and peak RSS 1,215,436 KiB.

All **5,448 case requests ran fresh**, with zero case reuse. The final fixed
128-mutant gate passed. The accepted bootstrap obtained the same exact producer
in 76.946 s (load 6.32 → 4.28). Relevant integration suites were:

| Suite | Seconds | Load before → after |
| --- | ---: | --- |
| Closed-proof attacks, including repaired lock fixture | 2.465 | 8.61 → 9.60 |
| Toolchain, including statistics consumer | 76.209 | 8.61 → 9.61 |
| Deferred semantic index page parity | 26.907 | 10.59 → 11.03 |
| Store I/O statistics | 4.436 | 11.16 → 10.91 |
| No-create cache reads | 1.076 | 11.16 → 11.16 |

The [accepted gate log](compiler-store-gate-data/accepted-cold.log) has SHA-256
`9f901ab92fad2bdaa2ef214b17e1e1d6d03d726b6a7c5ad86a44dfbd931fb555`.
Its time and metadata files are preserved beside it. The data manifest covers
both runs, the toolchain operation proof and the independent witness.
The new store occupied 316,078,231 bytes before cleanup; its directory and
private key were removed and the gate process was drained.

Neither fixture repair changes the `src/` or `library/` trees used to
produce this executable: those trees are byte-identical from `4f8c2135`
through `a0da7d7f`. Seed-only commit `13c4e6fd` stores the accepted
31,716,730-byte producer using deterministic `gzip -n -9`. The compressed
seed is 6,367,172 bytes, SHA-256
`6dbe650ab9e04d2391140e8e6ab53983672cd8bd3fdda284b330f4991e9835bb`.
A second compression compares byte-for-byte, and decompression compares exactly
with `target/lkjscript`. New CI acceptance will be recorded in
`docs/status.md`; no latency improvement is inferred from these correctness runs.
