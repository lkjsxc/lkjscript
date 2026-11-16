# Conditional reference capture: independent full-gate acceptance

The existing full gate is accepted from its retained records. No compiler, native executable, test, profiler, official reader or observer was rerun. The audit also preserves the distinct focused-query capture and the later seed-transition metadata.

## Gate result and accounting

The captured gate ended with status 0 and empty outer stderr. All 60 named phases correspond to the pinned gate script, have successful summaries and matching detailed logs, and lie within the reported gate interval. Every case-bearing summary was recomputed from its underlying statistics or closed-runner records:

| Phase | Rerun | Reused | Wall seconds | Load before → after |
| --- | ---: | ---: | ---: | --- |
| closed-semantics | 28 | 0 | 0.546 | 2.49 → 2.49 |
| toolchain | 96 | 0 | 64.705 | 2.49 → 4.43 |
| diagnostic-cases | 65 | 0 | 11.339 | 4.50 → 4.43 |
| program-directory | 4 | 0 | 40.332 | 4.43 → 3.95 |
| http | 28 | 0 | 29.021 | 3.95 → 3.77 |
| conformance | 813 | 0 | 72.964 | 3.39 → 4.38 |
| generated | 1215 | 0 | 54.151 | 4.38 → 5.20 |
| optimized-conformance | 813 | 0 | 73.666 | 5.20 → 3.96 |
| optimized-generated | 1215 | 0 | 56.804 | 3.96 → 4.77 |
| semantic-structure | 728 | 0 | 18.740 | 4.77 → 4.53 |
| semantic-cases | 436 | 0 | 12.590 | 4.53 → 4.46 |
| references | 14 | 0 | 20.671 | 4.46 → 3.56 |
| Total case requests | 5,455 | 0 | — | — |

The structural count is exactly **192 + 134 + 134 + 134 + 134 = 728**; its five complete child logs byte-concatenate to the detailed structural log.

Full-gate reported wall is **611.802 s**, load **3.45 → 3.68**. The independent outer nanosecond interval is **611.806682349 s**. GNU time records **611.80 s wall, 1,543.62 s user, 21.81 s system, 1,354,700 KiB peak RSS**, with full load **3.45/3.72/3.74 → 3.68/4.14/3.94**. The 60 phase wall durations sum to **889.982 s** because phases overlap under the four-worker gate; that sum is neither total wall nor CPU. The union of the printed, millisecond-rounded intervals is 611.739 s, leaving 0.063 s of printed gaps. Every individual duration agrees with its printed endpoints within one millisecond. Exact phase timings and adjacent loads are retained in `phases.tsv`.

Bootstrap records **77.069 s**, load **3.45 → 2.13**, and its complete detail log reports success. The existing mutation phase records **2.428 s**, load **3.56 → 3.68**; its inner summary reports 2.426 s for four workers. Its partition is **128 = 104 rejected + 22 values + 1 trap + 1 nontermination**, with zero resource outcomes and **zero findings**. It also records 18 checked reuses, three emitted reuses, zero store rejects and default inline0. The persistent mutation directory is empty: the existing aggregate log is retained, while individual temporary worker/per-mutant observations are not reconstructed.

## Exact inputs and the later seed transition

The gate before/after input manifests are byte-identical, SHA-256 `9134771a038cf3238f3fbde38d67bdf690c4751cc8269417a57160f82ad65c10`. All **1,939** named current source/test/tool/example/witness inputs independently match those digests. The gate script itself belongs to this manifest.

The canonical producer remains **b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db / 32,588,975 B**, matching both captured producer manifests. The recorded gate working base is `d03ad66bf672b3338ec7dd073560cf599e182303`; it is not presented as a commit containing every then-uncommitted candidate edit.

The gate captured the old seed **505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2** both before and after. A later, intentional seed refresh changed the live file. Its separately retained capture follows the gate and is now committed at **e5294c339d9aefbbafc2e951c997f4fefabcc9ad**. The new gzip is **c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae / 6,551,428 B**. This audit hashes both historical/current compressed Git blobs and the current seed; it preserves the root's successful repeated-gzip and expanded-byte comparison records without rerunning those operations. Recorded refresh cost is **4.31 s wall / 4.26 s CPU / 3,744 KiB**, load **2.06/2.28/3.04 → 2.29/2.32/3.05**.

## Complete raw retention

One deterministic `raw-logs.tar.gz` was created, **285,009 B**, SHA-256 **ae99b1ce2bff31a68cc4fe4b9bcd0a54469c5041b9ac8cef73f085e6d4149d0f**. All **424 regular members / 1,201,273 logical bytes**, plus **seven directories**, match the held originals; there are no duplicate, omitted or unexpected members within the declared selection. Full member mappings, original metadata and byte-equality proofs are retained.

| Original scope | Files | Logical bytes | Selection |
| --- | ---: | ---: | --- |
| target/compiler-reference-capture-gate-1 | 143 | 658,948 | Complete top captures and query evidence |
| target/gate | 119 | 351,661 | Every existing .log, including five structural child logs and mutation detail |
| target/compiler-reference-capture-query-1 | 140 | 188,007 | Complete focused capture and evidence |
| target/compiler-reference-capture-seed-1 | 22 | 2,657 | Completed transition metadata; three recoverable images excluded |
| Total | 424 | 1,201,273 | Seven directories |

The focused run is separately named and retained, including all **25 conditional-capture replies and two .lkj source projections**. Its recorded status is 0, **8.33 s wall / 8.21 s CPU / 88,668 KiB**, load **2.42/3.21/3.56 → 5.64/3.89/3.78**. The separate lifecycle acceptance belongs to semantic_queries; this gate audit makes no additional lifecycle verdict.

The seed capture's `candidate.gz`, `repeated.gz` and `expanded` are excluded because the exact seed commit provides recovery. No store, cache or key content and no executable binary copy enters this archive. The original roots remain untouched.

## Audit execution and preserved stops

Audit v1 failed its Perl syntax check before reading data or creating an archive. V2 passed source/current producer checks, then stopped when it found the later seed change. Root supplied the exact transition evidence and authorized the qualified historical continuation. Both helpers, launchers and original raw stop captures remain; their failures are not overwritten. A read-only discovery request also records an attempted nonexistent `tools/gate.lkj` lookup; the actual gate script is `tools/check.sh`.

V3 exited **0** in **0.31 s wall / 0.33 s CPU / 25,780 KiB**, load **2.17/2.26/2.85 → 2.17/2.26/2.85**, under 4 GiB AS / 900 s timeout / 10 s grace. It created the sole archive and compared every member against the original. No compression retry occurred. Original metadata and selected path sets remained unchanged. The final package manifest binds all audit versions, captures, reports and the archive.
