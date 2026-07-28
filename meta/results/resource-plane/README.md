# Semantic Resource Plane Evidence

## Scope

This retained evidence compares every complete scheduler policy and affinity mode
at commit `52ddc5cbe3d95e455cd78afbab411a7a93c51596` on one Linux x86-64 host.
It also measures forced proof-edit discovery, owner transfers, false sharing,
and a declared interference case. The evidence does not grant Linux scheduling
authority to lkjscript: Linux still schedules all worker threads system-wide.

## Protocol

- Release binary, locked dependency graph, 4 requested workers, 256 verified tasks.
- Two warmups and 15 measured samples for each policy/workload/affinity case.
- Three warmups and 30 samples for each proof-discovery worker count.
- Workloads: shared reuse, disjoint streaming, imbalance, false-sharing metadata,
  padded metadata, and owner transfer with remote release.
- Policies: sequential, static partition, global FIFO, local work stealing,
  hierarchical locality, and owner compute.
- Affinity: kernel managed, CPU pinned, and LLC-domain masked.
- Reported latency is wall-clock p50/p95/p99. Throughput uses all measured wall
  time. Queue wait and task time are monotonic runtime observations and do not
  affect semantic decisions.
- The interference case repeats CPU-pinned measurements with `yes` fixed to CPU
  0; it is an artificial sensitivity probe, not a production forecast.

## Host

Linux `7.0.0-27-generic`, x86-64, 32 observed processing units, one NUMA node,
and a restricted irregular 20-CPU effective cpuset. Cgroup CPU quota was
unlimited, the governor reported `powersave`, boost was enabled, transparent
huge pages were `madvise`, NUMA balancing was disabled, and `sched_ext` was
disabled. `CONFIG_SCHED_CACHE` is **unknown** because no readable running-kernel
configuration was available.

PMU data is unavailable because `perf` was not installed and
`perf_event_paranoid=4`. resctrl was unmounted, DAMON admin was unavailable, and
OS migration counters were unavailable. These observations are recorded as
unavailable, never as zero. Exact host and plan details and source JSON digests
are in `host.json`.

## Parallel Policy Result

Each case is normalized to its fastest parallel policy; lower is better.

| policy | median p50 | mean p50 | worst | wins |
| --- | ---: | ---: | ---: | ---: |
| owner compute | 1.020 | 1.040 | 1.297 | 5 |
| local work stealing | 1.021 | 1.039 | 1.164 | 3 |
| global FIFO | 1.028 | 1.097 | 1.442 | 5 |
| hierarchical locality | 1.033 | 1.046 | 1.170 | 4 |
| static partition | 1.104 | 1.232 | 1.703 | 1 |

Owner compute has the best median normalized p50 and ties for most wins, so it
is the resource-runtime default. Local work stealing has a slightly better
mean and worst case; it remains a complete selectable candidate. There is no
claim that owner compute universally wins.

Kernel-managed placement remains the default despite CPU pinning's lower median
on this host. CPU pinning was 1.001 median normalized p50, kernel managed 1.032,
and LLC masking 1.232. Pinning is host- and interference-sensitive and would
unnecessarily constrain Linux's normal placement authority.

All observed steals were within the one discovered LLC/NUMA group, so cross-LLC
and cross-NUMA counts are true zeros for this topology, not evidence about a
multi-domain host. Queue wait, task time, parks, steals, transfers, remote
releases, allocated bytes, peak live bytes, live objects, and checksums are in
the per-workload records.

## Sequential And Proof Result

Sequential won 7 of 18 overall workload/affinity cases because reuse,
streaming, and owner-transfer tasks are deliberately fine grained. It is not
removed. The proof-discovery fixture returned identical 4-record, 164-byte
certificates, 10,001 optimizing native entries, and zero VM fallbacks:

| workers | p50 ns | p95 ns | p99 ns |
| ---: | ---: | ---: | ---: |
| 1 | 1,029,064 | 1,053,521 | 1,078,127 |
| 2 | 1,112,261 | 1,154,871 | 1,229,762 |
| 4 | 1,127,890 | 1,158,238 | 1,178,185 |

Therefore proof-edit discovery remains sequential by default. Forced
multi-worker discovery remains available and uses deterministic static
partitions; reconstruction and the independent checker remain sequential.

## Interference Result

With one competing process fixed to CPU 0, owner compute had the lowest median
parallel degradation at 2.722x; its worst case was 5.292x. Other parallel
median ratios ranged from 3.358x to 3.811x. This rejects fixed CPU pinning as
the general default. The artificial run has 7 measured samples and is retained
under `interference/`.

## Data Layout

`summary.json` contains aggregate formulas and raw TSV SHA-256 identities.
`proof-discovery.json` retains proof results. The affinity directories retain
all numeric records split by workload to satisfy repository bounds. The
interference directory retains the sensitivity run. Every record includes its
output checksum; owner-transfer records show 256 transfers, 256 remote releases,
65,536 allocated bytes, and zero final live objects per sample.

## Raw TSV Identities

- kernel managed: `ebc585630353509865c8aa3ef748443470696e34e202a4e5f0f412a8fe9044ce`
- CPU pinned: `1f5f829443bad459cccf61eabb3517e674828ae0cc972d0b9294eea94ddd479f`
- LLC masked: `b2d21f00c0eae215786396d7ee99dc54181594b8afc2f2ffe0cc85816a66d12b`
- interference: `866b6ca5ab2423e678a698289be55b727dddf87a7f1eef54592cba2773f3bfe0`
- proof discovery: `fc66b11861d252139790a9a1e84dedc69cc9ac963fa11ab941e85cc0924d6f3e`
