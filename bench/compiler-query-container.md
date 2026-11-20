# Semantic snapshot containers: fixed 10k/100k result

The v3 container removes the measured cost of reconciling tens of thousands of semantic-node files during an ordinary body build. On the fixed 100,000-line specimen, the ordinary body build fell from **10.38 s to 5.33 s** (load **5.43,4.56,5.46 → 5.14,4.52,5.44** for v2; **4.89,4.48,5.42 → 4.65,4.44,5.40** for v3). Its Store metadata interval fell from **4,524.722790 ms to 0.390915 ms**, with the same respective loads. The checked fact maps, all bounded replies, and emitted images passed the comparisons declared before execution.

This is a tradeoff in cache representation. Two retained 100k snapshots occupy **1,232,718,650 bytes in the v3 snapshot family**, versus **615,284,946 bytes in v2**, because v3 duplicates each whole container while v2 shares almost all unchanged nodes. Index construction remains global and its peak memory is essentially unchanged: S0 v3 peak RSS is **2,330,656 KiB**, **0.61% above** v2's 2,316,460 KiB. The body build still scales with the input program; this experiment does not meet the note's size-independent edit target, establish a memory reduction, diagnose the historical 1M refusal, or measure the current compiler tree.

## Frozen scope and controls

The prelaunch bundle is commit **d459ab2bb8b5cdfe5cac7e861e4eff09256197c1**, based on **a0da7d7fd98e0897856d7c94fe5622f949509c7f** through protocol commit **8f28f9bf3741f684117fbe05575765f1d0598b34**. The protocol and implementation were frozen before this run. No measured command was retried, no producer or bound was substituted, and no 1M rung was attempted.

| Input or control | Frozen value |
| --- | --- |
| v2 producer | `adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7`, 31,716,730 bytes |
| v3 producer | `eed6ca32edcc182a92c4e697f4702ec685dff8e7778b0374a27552aa9c938947`, 31,922,848 bytes |
| Producer configuration | Both inline limit 64; candidate G1 and G2 byte-identical |
| Specimen output configuration | Inline limit 0; explicit frozen library and native runtime |
| Shape population | Compiler revision `0b30ca8987bd4ba343ab45ab01751b6721c7151a` |
| Generator seed / shape SHA-256 | 20261116 / `0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c` |
| Per-command address-space / time limit | 4,294,967,296 bytes / 900 s, then 10 s termination grace |
| Per-store writer capacity | 2,147,483,648 bytes |
| Free-space preflight | 403,021,434,880 bytes available; 9,126,805,504 bytes required |
| Query bounds | Depth 1, 96 facts, 32,768 output bytes; default reader capacity |
| Harness SHA-256 | `9a669c9d64ed997c33053a46a3da88479b45f90536dca47462bb712c2ccc570a` |
| Full-map/query comparator SHA-256 | `27a56b3601cf93ec41d1d0404ea636902c534428e1f65624569c6ef3b3bb05c5` |
| Store census SHA-256 | `d11bc22359a1296b4e259796b76a986931c35ec66bb89af632d08479263b515b` |

The v2 executable was materialized from the exact retained Git blob `13c4e6fd:seed/lkjscript.gz`, then verified against its declared raw digest. This corrects the executable-copy provenance without changing the predeclared producer. The candidate's factory source inventory, build argv, G1/G2 identity proof, and focused checks are retained in [the identity addendum](compiler-query-container-identities.md) and [validation record](compiler-query-container-validation.md). The earlier inline-0 correctness executables were not used in this A/B experiment.

The harness ran the 10k rung before the 100k rung. For each rung it ran v2 then v3 for each of five phases: ordinary cold S0, index S0, unchanged index S0, ordinary body S1, and index S1. A guarded edit changed only `edit::target`'s body from `value ^ 1u64` to `value ^ 2u64`. The target has eight direct source dependents. All source/read/runtime grants were explicit and narrow, and the private key was outside them. “Cold” means a fresh store; the host's operating-system caches were not flushed.

Each measured build enabled `LKJSCRIPT_STORE_IO=1`; a missing summary was a failure. A separately bounded census followed each build, before the next timed command. That census authenticates the signed Budget, checks physical lengths and accounting, and consequently warms metadata. All 20 censuses reported a valid Budget, capacity compliance, and no missing, length-mismatched, unindexed, or nonregular retained entries. Its status is separate from the compiler status. Full-map controls ran only after all 20 timed builds; all 104 bounded queries followed those controls. There are no repetitions or estimates of run-to-run variance.

| Lines | Source bytes | Files | Functions | Generic functions | Call sites | Nominal types | Fields | Variants | Nominal edges |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 10,000 | 384,223 | 12 | 384 | 17 | 4,973 | 14 | 24 | 19 | 10 |
| 100,000 | 3,923,070 | 169 | 3,764 | 155 | 47,607 | 196 | 637 | 204 | 375 |

The program generator and source statistics are unchanged from the declared scale machinery. These are generated specimens of compiler-derived shape, not a claim that generated text has every property of an independently written program.

## All 20 build observations

Wall/user/system times are GNU time values, in seconds. Each load cell gives the host's 1-, 5-, and 15-minute averages before and after that exact command. The complete argv, working directory, stdout, stderr, status, image digest, statistics, census, and wrapper timing are retained for every row.

| Lines | Format | Command | Wall s | User s | System s | Peak KiB | Load before → after |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 10,000 | v2 | Ordinary cold S0 | 1.54 | 1.46 | 0.01 | 126,028 | 5.72,5.05,6.01 → 5.66,5.04,6.01 |
| 10,000 | v3 | Ordinary cold S0 | 1.52 | 1.46 | 0.01 | 124,728 | 5.66,5.04,6.01 → 5.66,5.04,6.01 |
| 10,000 | v2 | Index S0 | 13.68 | 13.02 | 0.34 | 342,332 | 5.66,5.04,6.01 → 5.22,4.98,5.97 |
| 10,000 | v3 | Index S0 | 12.30 | 12.05 | 0.11 | 338,492 | 5.22,4.98,5.97 → 5.12,4.96,5.95 |
| 10,000 | v2 | Unchanged index S0 | 0.30 | 0.29 | 0.00 | 9,136 | 5.12,4.96,5.95 → 5.12,4.96,5.95 |
| 10,000 | v3 | Unchanged index S0 | 0.29 | 0.28 | 0.00 | 8,320 | 5.12,4.96,5.95 → 5.12,4.96,5.95 |
| 10,000 | v2 | Ordinary body S1 | 1.25 | 1.21 | 0.03 | 109,508 | 5.12,4.96,5.95 → 5.03,4.95,5.94 |
| 10,000 | v3 | Ordinary body S1 | 0.84 | 0.82 | 0.01 | 104,204 | 5.03,4.95,5.94 → 5.03,4.95,5.94 |
| 10,000 | v2 | Index S1 | 13.36 | 13.08 | 0.12 | 346,736 | 5.03,4.95,5.94 → 4.79,4.90,5.91 |
| 10,000 | v3 | Index S1 | 12.22 | 12.00 | 0.09 | 343,164 | 4.79,4.90,5.91 → 4.68,4.87,5.89 |
| 100,000 | v2 | Ordinary cold S0 | 11.17 | 10.95 | 0.10 | 773,876 | 4.68,4.87,5.89 → 4.34,4.79,5.85 |
| 100,000 | v3 | Ordinary cold S0 | 10.86 | 10.63 | 0.10 | 767,272 | 4.34,4.79,5.85 → 3.89,4.67,5.79 |
| 100,000 | v2 | Index S0 | 110.29 | 104.62 | 3.60 | 2,316,460 | 3.89,4.67,5.79 → 3.20,4.04,5.43 |
| 100,000 | v3 | Index S0 | 105.67 | 102.59 | 1.31 | 2,330,656 | 3.20,4.04,5.43 → 5.43,4.56,5.46 |
| 100,000 | v2 | Unchanged index S0 | 0.48 | 0.46 | 0.00 | 20,764 | 5.43,4.56,5.46 → 5.43,4.56,5.46 |
| 100,000 | v3 | Unchanged index S0 | 0.41 | 0.40 | 0.00 | 21,884 | 5.43,4.56,5.46 → 5.43,4.56,5.46 |
| 100,000 | v2 | Ordinary body S1 | 10.38 | 9.90 | 0.38 | 695,972 | 5.43,4.56,5.46 → 5.14,4.52,5.44 |
| 100,000 | v3 | Ordinary body S1 | 5.33 | 5.19 | 0.08 | 646,600 | 4.89,4.48,5.42 → 4.65,4.44,5.40 |
| 100,000 | v2 | Index S1 | 112.11 | 109.13 | 1.39 | 2,351,220 | 4.65,4.44,5.40 → 4.30,4.36,5.27 |
| 100,000 | v3 | Index S1 | 106.19 | 103.24 | 1.26 | 2,322,712 | 4.30,4.36,5.27 → 5.01,4.58,5.24 |

The 10k ordinary body wall reduction is **32.8%**; the 100k reduction is **48.65%**. These are single paired observations in the fixed order above. Index S0 wall falls from 13.68 to 12.30 s at 10k and 110.29 to 105.67 s at 100k, with the per-row loads shown above. Neither change is used as a general runtime-speed milestone.

## What changed in the body build

The dedicated per-node cache family is replaced by one container per snapshot. The ordinary writer still validates its signed accounting and scans owned entries. It now encounters one bounded container manifest plus its physical EOF, instead of opening each semantic node envelope.

| Lines | Format | Store I/O ms | Metadata ms | Budget read bytes | Headers / bytes | Extra manifest / EOF checks | Load before → after |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 10,000 | v2 | 461.773 | 356.960751 | 1,632,171 | 7,065 / 635,850 | 0 B / 0 | 5.12,4.96,5.95 → 5.03,4.95,5.94 |
| 10,000 | v3 | 65.976 | 0.347660 | 2,480 | 10 / 900 | 400 B / 1 | 5.03,4.95,5.94 → 5.03,4.95,5.94 |
| 100,000 | v2 | 5683.738 | 4524.722790 | 16,239,456 | 70,300 / 6,327,000 | 0 B / 0 | 5.43,4.56,5.46 → 5.14,4.52,5.44 |
| 100,000 | v3 | 720.406 | 0.390915 | 2,480 | 10 / 900 | 400 B / 1 | 4.89,4.48,5.42 → 4.65,4.44,5.40 |

`Store I/O ms` is the driver's `store_io` phase. `Metadata ms` is the independently instrumented exclusive `metadata_ns` Store subinterval. They are not interchangeable totals. `budget_header_bytes` continues to count only completed 90-byte headers; the extra 400-byte container-manifest read and EOF check have their own counters. The container front and EOF check authenticate its manifest and charge its full extent; they do not authenticate every node's content. A requested node remains independently authenticated.

At 100k, the signed Budget read shrinks from 16,239,456 to 2,480 bytes, and completed fixed-header inspections shrink from 70,300 to 10. The remaining v3 container work is one 400-byte manifest read and one exact EOF check. This experiment's v2 metadata observation is 4.525 s, with the load pair in the table; the earlier independent 6.903 s cohort is preserved in its own report and is not replaced by this result.

For indexed builds, streamed sealing/publication occurs inside the driver's `semantic_index` phase as well as Store operations. The migrated counter control therefore bounds the exclusive Store sum by the enclosing `store_io + semantic_index` intervals. It does not assert equality between those intervals or infer exclusivity merely from a broad bound. The earlier control that incorrectly required all indexed Store work to fit `store_io` alone remains in the implementation evidence as a fixture failure. `stream_write_ns` is included among the disjoint Store subintervals.

## Space, full snapshot duplication, and retained pins

“Snapshot family” below means only the dedicated page/root/latest/locator/container records. It excludes the ordinary semantic memo/input records, which remain included in “All store bytes.” The raw census namespace labels are preserved, including its residual `other` category. Signed Budget bytes are included in the all-store total.

| Lines | Format | After index | Snapshot files | Snapshot-family bytes | All store bytes | Signed Budget bytes | Retained pins |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 10,000 | v2 | S0 | 7,058 | 83,253,260 | 87,592,761 | 1,632,171 | 1 |
| 10,000 | v3 | S0 | 3 | 83,366,172 | 86,075,982 | 2,480 | 1 |
| 10,000 | v2 | S1 | 7,073 | 83,352,216 | 88,152,259 | 1,636,023 | 2 |
| 10,000 | v3 | S1 | 5 | 166,731,854 | 169,899,214 | 3,340 | 2 |
| 100,000 | v2 | S0 | 70,293 | 615,234,898 | 658,311,186 | 16,239,456 | 1 |
| 100,000 | v3 | S0 | 3 | 616,359,570 | 643,198,882 | 2,480 | 1 |
| 100,000 | v2 | S1 | 70,312 | 615,284,946 | 662,839,301 | 16,244,232 | 2 |
| 100,000 | v3 | S1 | 5 | 1,232,718,650 | 1,264,032,113 | 3,340 | 2 |

The candidate's individual container is **83,365,488 bytes at 10k** and **616,358,886 bytes at 100k**. Appending S1 retains the full S0 container and creates another complete container. The v2 representation instead retains only a few new nodes for this body edit. This is the principal storage cost of the change and would make aggressive snapshot retention materially more expensive.

All eight declared snapshot pins were retained and queried without rebuilding either S0 or S1. The 100k container exceeds the default query reader capacity of 256 MiB; queries still succeeded because allocation admission is per authenticated extent, while writer accounting charges the whole file. Queries of S0 were made after the source tree had become S1. They explicitly asked for the historical pin and do not establish current working-tree freshness.

| Lines / state | v2 snapshot | v3 snapshot |
| --- | --- | --- |
| 10,000 / S0 | d141c9820dbe434a56a37d438517a8ac2315a3493fc235db387b4e1b43f98753 | 5bd46ecfa54b71ba8d60edab6f221396b0dd4e128d26e23f4ea0da0696733938 |
| 10,000 / S1 | 6ea71f57c65647a6500a9a06aec76f18892f721393a88c8129700de81fe58845 | b9e4693dde9a86a7fae8a5d717716988bfa57d928b9d9156ff878d7319442330 |
| 100,000 / S0 | f23fe308ff0b2a753176c0f65a13afba05af8e9d541272111b7a98f4e1d15c7d | 60028dbc63b9f3a1d09afb6c6547d391e2e0c2cc795bfcad2cc108805ab0fef3 |
| 100,000 / S1 | e30b35b1469197276aac019e211145d3e62bdd12085f24843335943f11720296 | a5ab2cd141d10594d5dc9f956e73cbcaa0e51ec0dbea6750deeda12686ef51fb |

These snapshot/root identities differ across formats and producers by design. Exact equality of the old and new snapshot bytes was not the oracle.

## Full logical-map and output equality

The independent Perl comparator authenticates each format under its own producing compiler and private fixture key. It verifies all rooted nodes, v3 offset/length/postorder/contiguous-extent/EOF rules, the full logical key map, each exact logical value payload, and every logical continuation chain. Producer, snapshot, root, selector, and format-dependent cursor fields are validated against their own captured pins before the permitted provenance normalization. Fact text and other response content are not normalized.

| Lines / state | Logical pages / chains | Facts | Exact value bytes | Physical nodes | Wall s | Peak KiB | Load before → after |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 10,000 / S0 | 90,529 / 78,549 | 343,561 | 67,442,383 | 7,055 | 2.16 | 101,008 | 5.01,4.58,5.24 → 5.01,4.58,5.24 |
| 10,000 / S1 | 90,529 / 78,549 | 343,561 | 67,442,383 | 7,055 | 2.22 | 101,764 | 5.01,4.58,5.24 → 5.01,4.58,5.24 |
| 100,000 / S0 | 570,154 / 447,977 | 2,902,333 | 514,646,731 | 70,290 | 15.96 | 565,808 | 5.01,4.58,5.24 → 4.98,4.61,5.23 |
| 100,000 / S1 | 570,154 / 447,977 | 2,902,333 | 514,646,731 | 70,290 | 16.18 | 565,980 | 4.98,4.61,5.23 → 5.28,4.70,5.25 |

For both states at 100k, the **514,646,731 bytes of logical page values and all 2,902,333 facts are equal** across formats. The 10k equality covers 67,442,383 logical value bytes and 343,561 facts. Every chain is complete; the table's logical-page count differs from the physical radix-node count because one node may contain multiple logical key/value entries.

All four map controls returned success. For each rung and state, the native image emitted by v2 was byte-identical to the image emitted by v3; S0 and S1 image digests differ as expected. The experiment consists of **20 timed builds, four full-map controls, 104 query commands, and 52 pairwise query comparisons**. Setup/generator controls and censuses have separate records and are not counted as builds or query observations.

The small prelaunch helper fixture separately tested a forced nonempty continuation with limit 1, authenticating both the real v2 and v3 cursor envelopes and comparing their decoded traversal state. All rung queries happened to complete without a cursor. The result therefore proves complete rung maps plus the observed complete replies; it does not claim that a rung-specific nonempty cursor was exercised.

## All 104 bounded query observations

Each rung/state/producer executed exactly one command for each row of this question matrix.

| Kind | Identity |
| --- | --- |
| outline | `edit` |
| signature | `edit::target` |
| text | `edit::target` |
| slice | `edit::target` |
| uses | `edit::target` |
| callers | `edit::target` |
| callees | `main` |
| impls | `std::sequence::VecMutate` |
| instances | `edit::target` |
| type | `edit::target` |
| layout | `u64` |
| authority | `edit::target` |
| dependents | `edit::target` |

Every reply satisfied depth 1, fact limit 96, byte limit 32,768, and the requested kind/identity and producer/snapshot/source/IR selection. All fact-level labels were preserved byte-for-byte. Every normalized v2/v3 reply pair was byte-identical. Every command recorded `producer_bytes_hashed=0`. Replies contained 1–16 facts and read 4–17 physical nodes. All 104 commands recorded the same before/after load **5.28,4.70,5.25 → 5.28,4.70,5.25**.

| Format | Replies | Wrapper ms min / median / max | Semantic-query ms min / median / max | RSS KiB min / median / max | Output bytes min / median / max | Node reads min / median / max |
| --- | --- | --- | --- | --- | --- | --- |
| v2 | 52 | 2.208 / 2.734 / 5.288 | 0.551 / 0.910 / 3.376 | 4,088 / 4,216 / 6,320 | 722 / 817 / 2,496 | 4 / 5 / 17 |
| v3 | 52 | 2.332 / 2.904 / 6.127 | 0.689 / 1.147 / 4.323 | 3,336 / 3,400 / 5,500 | 722 / 817 / 2,496 | 4 / 5 / 17 |

The full **104 exact nanosecond wrapper durations**, rather than rounded summaries alone, are in [results/queries.tsv](compiler-query-container-data/results/queries.tsv), alongside each bound, selected identity, fact/node/traversal count, GNU time result, phase intervals, RSS, load pair, raw stdout SHA-256, and normalized SHA-256. The wrapper encompasses subprocess/resource-launch and stdout/stderr capture. It excludes the command/cwd/load metadata written before timing starts, and the subsequent census, hash, and validation work; the driver's `semantic_query` interval includes selected page authentication/decoding and output. `snapshot_auth` covers the pinned opening stage, not all later node authentication.

The wrapper medians are **2,733,923 ns for v2** and **2,903,907 ns for v3**; maxima are **5,287,518 ns** and **6,126,743 ns**. The candidate median is **6.22% higher**. All are small, warm, bounded questions after complete-map controls. This is useful evidence that large total container size does not force a whole-file allocation or read for these requests. It is not a fresh current-compiler latency campaign and does not replace the historical owner-table query measurements. No query-speed improvement is claimed.

## Remaining cost and two-point exponents

The 100k candidate body command remains **5.33 s**, versus **0.84 s** at 10k, a factor of **6.35** for ten times the source lines. Its major recorded driver phases are below; these are subintervals of the same command, with load **4.89,4.48,5.42 → 4.65,4.44,5.40**.

| Candidate 100k body phase | Milliseconds |
| --- | --- |
| Load and parse | 1,608.973 |
| Key and hash | 897.889 |
| Frontend restore | 733.498 |
| Store I/O | 720.406 |
| Type processing | 536.580 |
| Producer hash | 289.666 |
| Resolve | 161.424 |
| Frontend encode | 96.926 |
| Body check | 21.107 |
| Source read | 12.098 |

The command reused 4,208 checked functions out of 4,209. The reachable program still has 896,361 IR instructions; its emitted image is 4,473,047 bytes. Reuse of most function checks has not made source discovery, parsing, hashing, frontend payload work, or the retained fact construction independent of total program size.

The following exponents use exactly the two paired rungs, with `p = log(cost100k / cost10k) / log(10)`. CPU is user plus system time; RSS is peak resident KiB. Loads are the corresponding build rows above. The unchanged-build startup floor is excluded from this calculation.

| Command | v2 wall p | v3 wall p | v2 CPU p | v3 CPU p | v2 RSS p | v3 RSS p |
| --- | --- | --- | --- | --- | --- | --- |
| Ordinary cold S0 | 0.860532 | 0.853986 | 0.876045 | 0.863282 | 0.788204 | 0.788985 |
| Ordinary body S1 | 0.919287 | 0.802448 | 0.918571 | 0.802733 | 0.803146 | 0.792751 |
| Index S0 | 0.906450 | 0.934047 | 0.908501 | 0.931682 | 0.830377 | 0.837930 |
| Index S1 | 0.923838 | 0.939012 | 0.922867 | 0.936690 | 0.831294 | 0.830494 |

These are descriptors of two observations, not asymptotic proofs. Different generated graph sizes, fixed executable startup/hash cost, host load, and the single fixed order remain material. The largest accepted index still consumes about 2.3 million KiB of resident memory. Global fact materialization remains in memory, and v3 does not solve the original memory/size wall.

## Trust, compatibility, and validation limits

The format is v3 only. The manifest pins the actual producing executable, source identity, checked-IR identity, selector, root extent, and total file length. Every node is bounded, separately authenticated, and checked against its descriptor before a cache hit. Branches validate all child descriptors, including unselected children, without reading their payloads. A successful query returns an authenticated historical producer's claim with its existing source/checked-IR labels; it does not assert rechecking by the current executable or working-tree freshness.

The writer uses one newly owned private temporary file and held directory handles. It reserves the full staged extent and signed index headroom before each append, charges exact physical EOF, and publishes the complete file once. Exact legacy families are retired under the normal writer lock. Valid old locator accounting rows are recognized solely so retirement can preserve unrelated ordinary records; there is no v2 query fallback. Old writers are unsupported and may discard v3 records. First access to a mixed old store still incurs legacy discovery/retirement; the steady-state reduction does not erase that initial migration cost.

The independent static reviews found no blocker in the format, bounded extent checks, migration/accounting, final post-Memo retention, or the source-operation chain. The focused ordinary authentication/parser/CLI suites, no-create cases, mixed old-ledger control, too-large-total/small-reader-cap control, cached/unselected descriptor checks, deterministic poisoned-writer refusal, and final-Memo eviction witness are retained in [the validation record](compiler-query-container-validation.md). The production source has not been changed by this measurement/report commit.

A new syscall/race observer was stopped by automatic review during independent test preparation. It was not recreated or retried. Exact new concurrent mutation and interruption-boundary campaigns are consequently untested beyond existing ordinary facilities. A retained read-only file descriptor is not a kernel-sealed object; already-owned authenticated bytes may remain usable under the same fixed pin, and no assertion of detecting every append/restore race is made.

## Evidence, adoption, and reproducibility

The raw measurement directory is copied byte-for-byte into [measurement-1](compiler-query-container-data/measurement-1/). It retains all commands, statuses, stdout/stderr, timings, phase order, censuses, captured pin metadata, image digests, and oracle outputs. The raw and derived records are covered by [results.manifest.sha256](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-query-container-data/results.manifest.sha256). The independent postprocessor is [compiler-query-container-summarize.pl](compiler-query-container-summarize.pl); its derived tables do not modify raw evidence.

The earlier [implementation.manifest.sha256](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-query-container-data/implementation.manifest.sha256) remains unchanged. It covers 789 intentional text records, including refused operation requests, preflight failures, exact source inventory/hash chains, focused tests, build provenance, and independent audits. The original inline-0 cursor decoder failure and the later test-only historical cursor-domain correction remain visible with their narrower original coverage.

The retained production and standalone-case source delta is **945/945 operation-backed src .lkj lines**: 802 production lines and 143 case lines. Repeated Store operations used to validate the standalone dependency closure receive no duplicate credit. In addition, the frozen evidence package retains **58 manually authored .lkj specimen lines** at `implementation/no-create-1/probe-source.lkj`; these are **uncredited** and remain in place. Thus the all-retained-.lkj denominator for this bundle is **1,003 lines**, with **945** credited. This distinction prevents the exact src-only claim from inflating the project's broader adoption metric.

This report changes no measured source, language rule, authority policy, or repository truth. Text remains the stored source projection. The benchmark stores, private fixture key, source trees, images, and copied producers are retained outside Git until independent audit acceptance; only the intentional text evidence is committed. Subsequent integrated-current-compiler measurements require their own frozen producer, source pin, protocol, and explicit release.
