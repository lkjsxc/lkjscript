# Paired host consumer observation — 2026-10-10

## Question and fixed comparison

Does the already-integrated direct bulk-List construction materially improve an
actual native Map consumer? Compare the same frozen [native program](blocked.lkjc)
and artifact under two host Release executables, rather than comparing new samples
with the earlier unpinned [historical study](../study.md).

Before: source `b6a262f2e1ff95cb6f3889b0d10734a9ea93e078`, executable SHA256
`f9d8d5aff6f40c391ce2e46be1b4ce7f804fc30197630acff891c861cb054e43`.
After: accepted source `92330dc65151687ff2d23a07263ce1ce8ebfe2b0`, executable SHA256
`eb49f70d8822e45a8b3942add79c5a635f6398bc4b05b563a594f26179ebfbdb`.
Both are pinned Rust 1.98.0 Linux x86-64 GNU host Release builds. The source delta
under runtime/manifests/toolchain is confined to List construction and its tests.
This contribution itself adds no new runtime optimization relative to main.
The immutable public musl executable is not the performance predecessor.

Both use artifact SHA256
`d5a7073dc6fddf6eb14393f3d0eeef88e104d875746fc134f938d59a05cd5e01` and literal
native source SHA256
`0c52fb14b6e7739e46c631b943928449f989c6e35da2f83c49cb74a0acb5f267`.
No graph, input, callback or result law changes between executables.

## Method and complete observations

The predeclared matrix contains two executables, sizes 0/32/1,024/16,384, five
consumer strategies, eight repetitions inside the native program and ten outer
runs: **400 complete executions**. Mode order rotates/reverses and executable
order alternates within each pair. Run zero is retained as warm-up; each reported
median uses the nine remaining observations. All original samples remain retained.

The native program constructs a Map with values `3 * key + 7`. Length controls
return `n * rounds`; aggregation returns `(2*n*n + 5*n) * rounds`. Every result
matched that independent scalar law. Common Map construction remains included
in the invocation phase; these are not isolated callback nanobenchmarks.

The original measurement process returned exit 0. A separate reader admitted the
exact Cartesian inventory without missing/duplicate samples, every original result,
phase clock, unchanged input identity and joined cleanup. It confirmed 400 samples,
40 retained warm-ups, 40 groups of nine measurements and deterministic modeled
work in every exact-input group. The six process/measurement adapter tests passed.
The reader separately accepted a synthetic complete fixture and rejected six
corruptions; that self-test is not additional product or performance evidence.

Execution used CPU affinity 0–7 after the source, host/public-byte and candidate
selection checks finished. Existing services were not stopped. Frequency, ambient
load, NUMA placement and page-cache state were not controlled. The original Linux
process adapter's polling may add process-wall latency; product-reported invocation,
preparation and result-encoding clocks are separate. CPU/RSS are whole-process
observations. This experiment supplies no confidence interval or portable guarantee.

## Matched invocation medians

Times are milliseconds, with eight native repetitions and common Map construction
included. The entry-list strategies reuse one list except the explicit fresh-entry
length control. Mode 0 is Map length, 1 fresh entries then length, 4 standard fold,
8 blocked-four and 9 blocked-eight.

| Entries | Strategy | Before, ms | After, ms |
| ---: | --- | ---: | ---: |
| 0 | Map length | 0.073008 | 0.096060 |
| 0 | Fresh entries then length | 0.083137 | 0.111641 |
| 0 | Standard fold | 0.105820 | 0.125927 |
| 0 | Blocked-four | 0.125526 | 0.150924 |
| 0 | Blocked-eight | 0.126238 | 0.143470 |
| 32 | Map length | 0.160191 | 0.178897 |
| 32 | Fresh entries then length | 0.233730 | 0.263426 |
| 32 | Standard fold | 0.997578 | 1.027865 |
| 32 | Blocked-four | 0.781621 | 0.801639 |
| 32 | Blocked-eight | 0.750462 | 0.775500 |
| 1,024 | Map length | 2.642544 | 2.695965 |
| 1,024 | Fresh entries then length | 4.788824 | 4.771172 |
| 1,024 | Standard fold | 29.601883 | 29.361163 |
| 1,024 | Blocked-four | 21.396018 | 21.703267 |
| 1,024 | Blocked-eight | 19.892788 | 20.376949 |
| 16,384 | Map length | 46.878879 | 47.915567 |
| 16,384 | Fresh entries then length | 89.183834 | 86.940922 |
| 16,384 | Standard fold | 508.629860 | 494.868583 |
| 16,384 | Blocked-four | 400.824721 | 394.795619 |
| 16,384 | Blocked-eight | 351.807524 | 366.408958 |

Two comparison axes must remain separate. Within the **after** executable, the
large blocked-eight route takes about 26% less invocation time than its standard
fold. Across **before/after** executables, results are mixed: the same blocked-eight
route is slower in the after executable even though some other medians improve.
The List construction change therefore does not establish a uniform consumer or
whole-runtime speedup. Empty input also favors the standard fold over either
blocked alternative in the current executable.

Current large standard samples span 461.965946–503.826444ms; blocked-four spans
366.781813–404.920680ms; blocked-eight spans 340.724489–379.558568ms. Older large
standard samples span 489.054640–569.256407ms. Small cross-version median differences
and overlapping ranges are not evidence of a reliable runtime-level improvement.
Preparation is separately about 83ms for these selections and dominates the tiny
empty-input invocation; no resident warm-preparation benefit was measured here.

Modeled cumulative allocated bytes for the large standard route decrease from
271,894,571 to 271,622,411 across executables; blocked-eight decreases from
178,193,579 to 177,921,419. Within the current executable, standard and blocked-eight
instruction counts are 3,555,651 and 2,703,761. These are accounting, not measured
allocator traffic. Whole-process standard-route RSS medians are 36,980/32,088 KiB
before/after, but even the empty standard control differs at 27,792/22,416 KiB.
Those RSS differences cannot be attributed to reduced retained List storage alone.

## Decision and original evidence

Retain explicit native fold alternatives instead of replacing the standard fold
unconditionally or adding an unchecked cursor to the trusted kernel. The observed
consumer still pays for callback/loop setup; faster construction alone does not
remove those costs. Select subsequent prepared-call, traversal or region/view work
with matched consumers and complete contracts. The nested persistent-history
consumer remains correctness evidence, not a workload timed by this scalar matrix.
No C/Rust/Bun, production-application or API-cost comparison was performed.

Original evidence root:
`/home/coder/workspace/lkjscript-native-fold-study-20261009/.artifacts/fold-completion-20261010/paired-native-costs/`.
`METHOD.md`, source/executable copies, byte inventories, all 400 raw executions,
`evidence/groups.tsv`, `evidence/summary.json`, independent reader outputs and
self-tests remain at their original owner. Both measurement and reader exited zero.
A later supplemental calculation/read request was denied by tool policy and was
not repeated by another route. This report uses the earlier successfully returned
complete group table and independent-reader terminal, not unseen supplemental files.
