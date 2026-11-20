# Scalar simplification: retained miss

The opt-in, independently rechecked Bool/Int simplifier is **discarded under the predeclared 10% rule**. Mean GNU runtime wall is 15.180000 s for inline64 and 15.073333 s with the scalar stage, an observed 0.702679% reduction. All six outputs are byte-identical to the frozen seed. The candidate was not broadened or run again after this result.

The [protocol](compiler-scalar-simplify-plan.md), [final producer pin](compiler-scalar-simplify-data/producer-pin.json) and [execution release](compiler-scalar-simplify-data/execution-release.md) were committed at `c887b7c5aca7bb80d70ca26d7dfdf1a44213a163` before the single cohort. The [independent acceptance](compiler-scalar-simplify-data/acceptance/report.md) verifies the raw observations and exact byte provenance.

## What was tested

One pinned emitter constructed two executables from frozen source `26bf2c3c021dd79810a81fa1d9054d59f3768b06`. A used inline64; B used inline64 plus the scalar stage. The old compiler logic in both executables then built the same source at inline0, with store disabled and no scalar flag. Construction is excluded from the runtime estimator.

The experimental source is `5b5ce7759c035ae0437e3512e18efc440aa50d2e`, above published prerequisite `e0b21e66659dc1830a5bebddf0c1f30fd2d8fa94`. Its emitter is 32,761,734 B, SHA-256 `f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64`. The source is retained as an exact Git bundle, and the actual executable as a verified compressed artifact. Neither is the main producer.

The fixed scalar subset covers Bool constants/identities and exact integer bitwise operations/comparisons. It excludes arithmetic, shifts, casts, ranges, aggregates, loop transforms and removal of dead producers. Address-taken locals, effects/control barriers and the complete sorted trap-message inventory constrain rewrites. Every selected output is rechecked; rejection preserves the last admitted candidate, and the original interpreter oracle stays unchanged.

## Complete cohort

Exactly one compute worker ran the two constructions and six executions in the declared order, with 4 GiB address space, a 900-second per-phase watchdog and ten-second TERM grace. There were no warmups, profiles, cache resets, retries or omitted observations.

| Phase | GNU wall s | CPU s | Peak RSS KiB | Load before → after |
| --- | ---: | ---: | ---: | --- |
| Construct A | 24.68 | 24.16 | 793,080 | 1.36/1.51/2.07 → 1.66/1.56/2.07 |
| Construct B | 32.41 | 31.78 | 1,169,088 | 1.66/1.56/2.07 → 1.81/1.61/2.07 |
| A1 | 15.01 | 14.73 | 526,328 | 1.81/1.61/2.07 → 1.77/1.61/2.06 |
| B1 | 15.13 | 14.76 | 523,792 | 1.77/1.61/2.06 → 1.55/1.57/2.04 |
| B2 | 15.15 | 14.91 | 519,656 | 1.55/1.57/2.04 → 1.43/1.54/2.02 |
| A2 | 15.22 | 14.89 | 521,184 | 1.43/1.54/2.02 → 1.68/1.59/2.03 |
| A3 | 15.31 | 15.03 | 523,724 | 1.68/1.59/2.03 → 2.00/1.66/2.04 |
| B3 | 14.94 | 14.64 | 527,144 | 2.00/1.66/2.04 → 1.92/1.66/2.04 |

Every output is 16,228,028 B, SHA-256 `8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132`, and was compared in full with the exact frozen Git seed. Runtime stdout/stderr are empty. Each construction emits only its one expected statistics row to stderr.

| Decision measure | A | B |
| --- | ---: | ---: |
| Runtime wall sum | 45.54 s | 45.22 s |
| Runtime arithmetic mean | 15.180000 s | 15.073333 s |
| Runtime median, secondary | 15.22 s | 15.13 s |
| Runtime CPU sum | 44.65 s | 44.31 s |
| Constructed image | 22,937,783 B | 22,929,591 B |

The exact integer comparison is `10 × 45,220,000,000 > 9 × 45,540,000,000`, or `452,200,000,000 > 409,860,000,000`. It fails the threshold. B/A is 0.992973210365; the median does not change the decision.

The image shrinks by 8,192 B (0.035714%). Construction wall increases 31.320908%, CPU 31.539735% and peak RSS 47.411106%. B admits one nonempty scalar candidate with 16,016 constant folds, 389 identities and 18 branch rewrites in 3,324 functions. Both constructions admit the same 30,619 inlined calls in 5,617 functions, with no inliner rejection. Scalar rejection, message mismatch and trap-guard counters are zero.

The construction statistics separately record 12,405.796/19,568.557 ms of body checking and 685.932 ms for B's scalar stage, under the construction load windows above. These integrated phase observations do not isolate the cost of individual transformations. Rewrite counts alone do not establish removed runtime cost.

This is one fixed-order cohort on a shared host. GNU elapsed values retain their centisecond resolution; the separately recorded nanosecond wrapper durations do not replace them. No randomized causal effect, new Rust-O ratio, backend floor or whole-program limit on optimization follows.

## Correctness before timing

The isolated candidate passed its focused CLI/cache/profile/worker-forwarding checks and full cold gate before execution release:

| Check | Result | Wall s; load before → after |
| --- | --- | --- |
| Bootstrap | Two generations agree; library validation passes | 77.47; 1.34/1.21/1.32 → 1.69/1.33/1.36 |
| Focused CLI | Conflict orders, original observations, nonempty scalar rewrite, policy-separated images, profiled fragments/IR, worker forwarding | 3.35; 1.64/1.53/1.44 → unchanged |
| Full local gate | 7,476 fresh case requests, zero reused | 754.526 internal /754.52 GNU; 1.32/1.44/1.42 → 5.01/4.81/3.26 |
| Independent result audit | Eight phases, exact source and six full outputs verified | 2.34; 1.67/1.81/1.98 → 1.69/1.81/1.98 |

The gate's 149 scalar conformance and 538 scalar generated admissions report zero scalar checker rejections or message mismatches. Two declared pending conformance cases per route remain pending; the existing inliner retains twelve fallback rejections. Generated routes have 805 source cases and 1,215 requests apiece. Both fixed 128-mutant routes report zero findings; the scalar route actually attempts, admits and executes four branch rewrites at inline0. The semantic host passes 36 native-observation contracts.

The source-form refusal, missing include-input setup, conservative replay refusal and later successful explicit test transaction are preserved. Retention also preserves all three auditor-only assertion/prototype/sort failures and the successful fourth reader, plus packaging/report warnings. None caused a measured program to be rerun.

The candidate's 1,401 retained added/deleted language lines were applied through 30 independently audited semantic operations. Because the candidate is discarded, those isolated lines add nothing to main's operation-adoption numerator or denominator. Main's production source, ordinary defaults, producer policy and seed remain unchanged by this item.

Candidate CI was not triggered: the conditional adoption step was not reached. Baseline [CI43904619997](https://github.com/lkjsxc/lkjscript/actions/runs/43904619997) passes both jobs at `7716f0d5`; it validates the unchanged main production source, not the scalar experiment. Exact pretrial and completed observations are retained.

## Retained evidence and recovery

[Acceptance JSON](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-scalar-simplify-data/acceptance/acceptance.json) verifies all 112 raw phase files, all 172 archive files (171 source/library files plus seed), 218 candidate source/library files, 216 main files, the release blobs and emitter. The full [raw cohort and outer captures](compiler-scalar-simplify-data/cohort-records/target/compiler-scalar-simplify.20261116-1/rows.tsv) are copied byte-for-byte with their original paths. [Copy verification](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-scalar-simplify-data/cohort-copy.json) binds those files and both frozen reviewer/proposal packages.

The [two-scope recovery proposal](compiler-scalar-simplify-data/cohort-retention-proposal/proposal.md) names all 337 files/207,027,139 B. Both actual constructed images and the exact frozen source tar are now retained in the [verified recovery package](compiler-scalar-simplify-data/cohort-package/README.md). A/B gzips occupy 4,530,022/4,529,831 B; the source tar gzip is 3,824,641 B, and all 154 raw records also occupy one 24,654 B archive. The 172 extracted source files recover from that exact source archive; six outputs plus the expected image recover from its one exact seed member. The copied emitter recovers from its already committed gzip, which was verified once in place. Every archive member and expanded artifact was byte-compared; no second emitter or seven output gzips were created. Root verified an exact 71-file/13,732,575 B package copy.

Packaging took 4.48 s (load 1.73/1.27/1.53 → 1.75/1.28/1.53). The original verification report serialized seven empty-stream digests after successful first-digest guards because the digest method resets its object. Both that report and a clearly labelled metadata-only correction are retained. The correction took 0.04 s (1.78/1.43/1.56 unchanged), changed no artifact and repeated no compression or byte comparison. Use verification-corrected.json, whose per-field provenance names those successful guards and fixed policy values. The separately released cleanup now removes exactly those 337 original files/207,027,139 B and 22 directories in 1.37 s (load 11.04/7.99/4.60 unchanged), with all 72 committed recovery files checked before any unlink. The [independent result audit](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-scalar-simplify-data/cleanup-acceptance/acceptance.json) verifies all 363 ordered progress events, both current ENOENT roots and unchanged canonical compiler bytes/device/inode/mode. Its one offline read takes 0.28 s (2.51/5.36/4.15 unchanged). Complete [raw execution](compiler-scalar-simplify-data/cleanup-execution/stdout), the frozen guard and release, and byte-exact retention copies remain alongside the recovery artifacts. This cooperative closure is non-atomic; no concurrent filesystem or rollback guarantee is claimed.

Earlier candidate source, original desired overlays, focused/gate records and actual preliminary/live/compiler artifacts remain under [the pretrial release](compiler-scalar-simplify-data/execution-release.md). The source bundle requires its published prerequisite and is not a standalone repository. No service, release or tag was changed.
