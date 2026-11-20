# Conditional source-reference capture

Ordinary and declaration-only compilation now omit the two source-reference export vectors. Full indexing and reference-dependent edits select recording before inference. This removes work that has no consumer in an ordinary build while preserving inference, lexical binding origins, closure capture and independently checked IR.

The fixed two-input comparison is complete: all six images per input are byte-identical. The compiler's GNU mean wall time is unchanged at 20.173333 s (load 3.37/2.63/2.85 → 2.21/2.52/2.79). Mean peak RSS is descriptively 0.771638% lower on the compiler and 1.367061% lower on the new 100k specimen. This sequence establishes neither statistical significance nor an explanation or relief of the earlier 1M R0008 refusal.

## Contract and implementation

`DefinitionTypes.local_paths` and `semantic_bindings` were populated by initial, demanded and fallback inference even when no full source catalog would be exported. Their only production reader is `DefinitionTypes::semantic`. The implementation guards their three recording writes and leaves the surrounding constraints, binding origins, capture paths, method selection and ownership work intact.

The driver chooses `capture_references = memo.capture_catalog && !memo.catalog_items_only` once, before initial inference. The concrete compiler and generic-verification compiler retain the same mode. Incremental ordinary/declaration-only resumption explicitly uses false. Existing direct frontend APIs retain full capture by default.

| Request | Recording | Checkpoint behavior |
| --- | --- | --- |
| Ordinary check, build or IR | Off | Eligible for existing ordinary frontend resumption |
| Declaration-only comparison or item/body operation | Off | Eligible; reference completeness remains false |
| Full index or reference-dependent edit | On | Re-infer unless a separate authenticated full snapshot is reusable |

A late request to export full evidence from capture-off inference, or a concrete/verifier mode mismatch, returns `E_SEMANTIC_CAPTURE` before taking or changing either caller catalog. Empty vectors never prove a complete set of references. The ordinary image and LKJFRONT2 checkpoint alone cannot satisfy a later full index. The existing full-snapshot authentication requirement remains the gate for unchanged indexed image reuse.

No IR field, store wire format, request-key mode bit, runtime counter, observer or optimization policy was added. Source facts remain frontend claims; checked authority, layout, ownership and loan facts retain their existing IR boundary. [IR contracts](../docs/ir.md) and [structural operations](../docs/structure.md) describe the distinction.

## Preliminary validation and retained refusals

The desired projection was checked and its separate native semantic suite was built by the prior canonical compiler `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc`. These runs validate the new source and its executed frontend paths; they are separate from the later candidate producer and full gate.

| Phase | GNU wall s | Peak RSS KiB | Load before → after | Result |
| --- | ---: | ---: | --- | --- |
| Core source check | 19.77 | 678,068 | 1.28/1.64/1.87 → 1.40/1.64/1.87 | Pass |
| Semantic build 1 | 16.89 | 596,556 | 1.42/1.27/1.52 → 2.08/1.43/1.56 | Refused new test forms |
| Semantic build 2 | 16.97 | 585,244 | 1.32/1.90/1.90 → 1.52/1.91/1.90 | Refused new predicate form |
| Semantic build 3 | 18.68 | 583,112 | 1.17/1.64/1.80 → 2.95/1.99/1.91 | Refused inferred unsized test binding |
| Semantic build 4 | 18.22 | 583,164 | 8.02/6.50/3.82 → 6.77/6.30/3.80 | Pass |
| Native closed semantic execution | 61.26 | 89,456 | 6.77/6.30/3.80 → 12.60/8.18/4.62 | 728 contracts, zero failures |

The first refusal found borrowed string views in heap-held test vectors and missing closure context. The second found an unsupported declared predicate signature; the third found an inferred unsized match result in the new assertion helper. The test source was rewritten to use local views, an ordinary borrowed loop, explicit owned binding names and direct Shape equality. No unrelated inference rule or heap-view capability was added. All failed source forms, exact corrections, diagnostics and manifests remain in the [preliminary package](compiler-reference-capture-data/preliminary/README.md).

The seven additional closed-core groups compare exact independently checked binary IR with capture off/on; interpret the fixed result; exercise demanded generic trait/closure selection and all-library validation; preserve diagnostic order and locations; reject late full export before publication; preserve legacy full exports; and force the real deferred-pattern and missing-definition fallback paths. Private assertions inspect both recording vectors without exposing a production counter. The core count is 192 and the unchanged four source partitions contribute 536, totaling 728.

## Actual semantic publication

| Stage | Operations | Sites | Files | Retained .lkj lines | Wall s; load before → after |
| --- | ---: | ---: | ---: | ---: | --- |
| Compiler-root replay | 21 | 20 | 7 | 280 | 66.16; 2.13/4.78/4.02 → 4.05/4.73/4.05 |
| Explicit test-root transaction | 3 | 3 | 3 | 305 | 37.10; 2.58/4.05/3.88 → 2.38/3.85/3.82 |
| Total | 24 | 23 | 10 | 585 | Two sequential transactions |

Core replay independently checks 12,924 instances and reports exact desired/candidate canonical checked-meaning identity `59681227ba0d1ace14608e4363d8901ccf2466c2d6e7e49e19102efe8bb31c16`. This is the established canonical comparison, not a claim that arbitrary serialized IR or all source bytes are equal.

The second transaction uses the real semantic-test root and three explicit operations: create the external module projection, replace the core body, and replace its expected-count body. It independently checks 10,532 instances and reports final checked meaning `0b770ac6868af0bd5a3647b4bb003af31ba1bafdb45a0934a98d8b402854dc4d`. No desired-replay equality is invented for this request.

The [independent adoption audit](compiler-reference-capture-data/adoption/report.md) verifies every baseline Git blob and publication hash chain: 1,742 initial files, 1,743 final files, and 1,733 untouched originals, including all 32 library and 1,526 test files. All 585 retained added/deleted language lines are attributed to the successful operations: 564 additions and 21 deletions. The new module contributes 300 lines. Shell scripts and archives are outside that language-line measure.

Three actual projections differ from desired only in the completely inspected detached comments and blank lines; 1,740 files match exactly. Six desired comment lines were not published and earn no retained-line credit. The raw patches are retained. Core had a successful desired-manifest preflight rather than a separate desired-before copy; the later exact before/after checkpoints remain identified instead of inventing that missing capture. The offline audit takes 0.20 s (load 3.85/3.81/3.77 unchanged), with no compiler execution.

The [committed-source closure](compiler-reference-capture-data/commit-acceptance/report.md) binds all 1,743 adopted hashes to exact Git blobs in source commit `bd8a9050a3786de9f227c6c4037f5acd43205f71`: 9,380,526 bytes, the same ten changed language paths, and 585/585 retained lines. No extra `.lkj` evidence file enters the denominator. The complete 4,146-byte Programs section matches both the baseline and immediate parent, SHA-256 `235aa6c283d334688289ab73e472cae920d25b1e8cbad76cb4461f30d18cc9f4`.

## Candidate producer and query lifecycle

Bootstrap succeeds with two equal current generations and all-library validation. The candidate is 32,588,975 B, SHA-256 `b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db`, built at the established producer inline64 policy. Bootstrap takes 79.97 s, peak RSS 1,232,572 KiB, load 1.58/3.39/3.66 → 2.78/3.37/3.63. Ordinary emitted programs retain inline0.

The separate seed-only commit `e5294c339d9aefbbafc2e951c997f4fefabcc9ad` stores deterministic gzip `c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae`, 6,551,428 B. Two independently written gzip outputs compare byte-for-byte; both the prepared expansion and the committed Git blob expand exactly to the validated producer. The [refresh capture](compiler-reference-capture-data/seed/outputs.sha256) takes 4.31 s (load 2.06/2.28/3.04 → 2.29/2.32/3.05). It occurs after the gate, whose before/after seed remains the old `505be947`.

The complete focused query suite passes in 8.33 s, peak RSS 88,668 KiB, load 2.42/3.21/3.56 → 5.64/3.89/3.78. Its new lifecycle case uses one four-definition source, seven builds and 25 bounded pinned replies. Ordinary and one-body resumed builds are followed by full index regeneration; each full result matches its separate cold indexed control exactly. The ordinary body edit resumes three definitions and freshly infers one. Each full regeneration infers all four definitions with no frontend reuse. The unchanged full index reuses its stored image with zero new definitions.

The five repeated question forms cover shadowed call/local references, local i64 type, generic references, generic outline and generic binding type. Before/after cold controls and unchanged reuse agree on complete reply bytes and snapshot identities. The existing authentication, provenance, corruption, missing-source and edit refusal cases remain part of the passing suite. This correctness run is not a new compiler-scale query-latency campaign. The [independent query audit](compiler-reference-capture-data/query-acceptance/report.md) verifies all seven terminal build records, 25 complete replies and 15 byte-equality comparisons. Sixty fact rows occupy 28,865 bytes, with at most five facts/1,680 bytes per reply and no continuation. Two warm full regenerations reuse three checked-IR proofs; fresh frontend inference is not a claim of zero IR proof reuse. Individual build-exit and per-query argv/status files were not separately captured; the retained suite exit and terminal records define that evidence boundary.

The four-worker empty-store gate passes in 611.802 s internally /611.80 s GNU, with 1,543.62 s user and 21.81 s system CPU, peak RSS 1,354,700 KiB, and load 3.45/3.72/3.74 → 3.68/4.14/3.94. All 5,455 case requests are fresh and zero are skipped; the closed structural route executes all 728 contracts. The fixed 128-mutant slice reports 104 rejections, 22 values, one trap, one nontermination and zero findings. Its internal 18 checked/three emitted reuses are separate from case-request reuse. The full source/tool/example/witness input manifest, canonical producer and old seed remain byte-identical across the gate. [CI run 43925403668](https://github.com/lkjsxc/lkjscript/actions/runs/43925403668) subsequently passes both jobs at pushed head `1148e56a2a324e2bbe0e92cfdb4762025519cd5c`: native gate 1,172.201 s (load 0.57→2.55), 5,455 fresh requests and zero skips; the optional Rust witness reports 276 value/output cases and 344 verdicts. Both bootstraps succeed. The completed observation at 2026-11-16T18:33:29Z, complete logs and earlier in-progress responses are retained in the [CI capture](compiler-reference-capture-data/ci-result.md). Later comparison releases change only records and helpers, leaving the gated source, library, tests and seed identities intact. The completed comparison publication at `d84cac6e` also passes both jobs in [CI43936512325](https://github.com/lkjsxc/lkjscript/actions/runs/43936512325); the [completed response](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-reference-capture-data/ci-publication-43936512325.json) records the native job finishing at 2026-11-16T19:25:36Z.

The [independent gate audit](compiler-reference-capture-data/gate-acceptance/README.md) verifies all 60 phases and 1,939 captured input files. One raw archive retains 424 regular files/seven directories/1,201,273 logical bytes, including the complete gate and focused-query captures, every existing gate log and seed-transition metadata; every member matches its original bytes. The audit preserves its initial syntax stop and its subsequent comparison against the intentionally refreshed seed, and then distinguishes the historical gate seed from the later seed commit. Those audit stops did not rerun a gate or a workload. The sum of phase walls overlaps and is not total elapsed time.

## Fixed comparison: preparation and release

The [prospective protocol](compiler-reference-capture-data/measurement-preparation/plan.md) compares producer A, `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc`, with validated producer B, `b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db`. A is recovered from exact seed commit `7d0731e99aeaf241c4334fc5ff065a341fb388c8`; B from `e5294c339d9aefbbafc2e951c997f4fefabcc9ad`. Their source commits are respectively `d461c2d98e93aeed6061508ee80ab36b9eeaa04f` and `bd8a9050a3786de9f227c6c4037f5acd43205f71`. The baseline is not a later mutable canonical executable.

Both inputs use the fixed A1/B1/B2/A2/A3/B3 order, one compiler worker, identical explicit source/library/runtime selectors, no store, inline0 and statistics. Each command has a 4 GiB address-space ceiling and a 900 s watchdog with 10 s termination grace; free space must be at least 3 GiB. No warmup, operating-system cache reset, extra sample, profile or larger rung is added. Preflight reads and after-build validation can warm caches; the GNU compiler intervals exclude that separately captured metadata work. Setup and measurement have separate releases.

The first setup stopped before any compiler or generator command: source-archive validation rejected two unresolved Git PAX long-path placeholders. Recover-inputs returned 255; all four later setup phases and all twelve original measurement rows remain not-run. The outer setup took 0.40 s (load 2.07/2.24/2.84 unchanged). The [original failure and closed cohort](compiler-reference-capture-data/original-cohort-closed.json) are retained without alteration.

The [explicit corrected-setup amendment](compiler-reference-capture-data/corrected-setup-release.md), committed at `5e1b82f3`, permits one separate preparation after that metadata-only reader failure. It interprets the two preceding per-file PAX path records, while preserving the complete Git path/content/mode checks. No performance sample existed to repeat or select. Read-only archive validation passed for all 1,742 files and 42 directories in 0.28 s (load 1.16/1.50/2.23 → 1.22/1.51/2.23), without extraction or native execution.

Corrected setup runs once and passes all five phases: 5.25 s outer wall, 128,740 KiB peak RSS, load 2.54/1.86/2.30 → 2.66/1.89/2.31. Generator construction takes 2.07 s (2.54/1.86/2.30 → 2.66/1.89/2.31); generation takes 1.10 s (2.66/1.89/2.31 unchanged). The source/input artifacts and [independent setup acceptance](compiler-reference-capture-data/setup2-acceptance/report.md) are committed at `1404b4af` before measurement. The audit checks all 3,817 input files and all five successful phase captures in 1.77 s (load 1.46/1.42/1.88 → 1.50/1.43/1.88), peak RSS 163,228 KiB.

The compiler input is the exact 1,742-file Git archive at `c887b7c5aca7bb80d70ca26d7dfdf1a44213a163`, source tree `c710586b4d1b7ab476fd179a7d2c699477a2054a`. The new specimen is generated with seed 20261116 from the retained generator/shape recipe: 169 source files, 100,000 lines and 3,923,070 source bytes, plus one workload metadata file. It contains 3,764 functions, 155 generic functions, 47,607 calls, 196 types, 637 fields, 204 variants and 375 type edges. Its shape input is the historical 83,992-line compiler census, not a new census of current source. Historical 100k bytes had been cleaned; their old manifest is not substituted for this new input. Complete recoverable inputs remain under [setup2](compiler-reference-capture-data/input-retention.md).

The final controls are retained under [trial-preparation-2](compiler-reference-capture-data/trial-preparation-2/protocol.md). A narrow pre-execution correction validates unsigned numeric fields before comparison rather than relying on JSON scalar encoding; both versions and the exact diff remain. The once-only release is committed at `0547c7041de0710dbe6e3e98b590a9231fc32329`, after input recovery, the cold gate and completed candidate CI. The original closed cohort is not resumed.

## Twelve observations

All twelve compiler commands and fourteen metadata commands succeed in their fixed, non-overlapping order. Every compiler stdout is empty; each stderr holds exactly one successful statistics object. Reuse and inlining counters are zero. All original source and producer pins match before and after. Both the twelve retained phase images and the two final reused output slots compare in full bytes to the respective A1.

| Input | Actual image bytes | SHA-256 of all six outputs and final slot |
| --- | ---: | --- |
| New 100k | 4,468,259 | `42443b75fe211c9f9efa7c0ee26c9233280c38a124ec5e5b514666aab97ce361` |
| Compiler | 21,489,004 | `5ab411b7fb91498adc1453a08d96893f85c1d644a2e050b93b3e9dc850585597` |

| Input | Run | Wall s | User + system s | Peak RSS KiB | Load before → after (1/5/15 min) |
| --- | --- | ---: | ---: | ---: | --- |
| New 100k | A1 | 9.67 | 9.27 | 697,408 | 1.39/2.22/2.75 → 1.71/2.27/2.75 |
| New 100k | B1 | 9.47 | 9.26 | 698,128 | 1.71/2.27/2.75 → 2.05/2.32/2.77 |
| New 100k | B2 | 9.10 | 8.99 | 697,564 | 2.05/2.32/2.77 → 2.35/2.38/2.78 |
| New 100k | A2 | 9.13 | 8.98 | 720,068 | 2.35/2.38/2.78 → 2.98/2.52/2.82 |
| New 100k | A3 | 9.11 | 8.98 | 719,664 | 2.98/2.52/2.82 → 2.97/2.53/2.82 |
| New 100k | B3 | 9.09 | 8.97 | 712,232 | 2.97/2.53/2.82 → 3.37/2.63/2.85 |
| Compiler | A1 | 20.28 | 19.82 | 749,748 | 3.37/2.63/2.85 → 3.50/2.72/2.88 |
| Compiler | B1 | 20.10 | 19.80 | 745,784 | 3.50/2.72/2.88 → 3.10/2.67/2.86 |
| Compiler | B2 | 20.30 | 19.94 | 740,940 | 3.10/2.67/2.86 → 2.99/2.68/2.86 |
| Compiler | A2 | 19.97 | 19.72 | 751,124 | 2.99/2.68/2.86 → 2.63/2.61/2.83 |
| Compiler | A3 | 20.27 | 19.87 | 745,260 | 2.63/2.61/2.83 → 2.49/2.59/2.82 |
| Compiler | B3 | 20.12 | 19.79 | 742,076 | 2.49/2.59/2.82 → 2.21/2.52/2.79 |

The outer capture takes 179.66 s wall, 174.67 s user plus 1.73 s system CPU and 751,124 KiB peak RSS, with load 1.39/2.22/2.75 → 2.21/2.52/2.79. It includes separately timed preflight, validation and postflight work. It is not a thirteenth compiler observation.

| Input / metric | Mean A | Mean B | Median A / B | Range A / B | Mean B/A | Input interval load 1/5/15 min |
| --- | ---: | ---: | --- | --- | ---: | --- |
| New 100k wall, s | 9.303333 | 9.220000 | 9.13 / 9.10 | 9.11–9.67 / 9.09–9.47 | 0.991042637048 | 1.39/2.22/2.75 → 3.37/2.63/2.85 |
| New 100k CPU, s | 9.076667 | 9.073333 | 8.98 / 8.99 | 8.98–9.27 / 8.97–9.26 | 0.999632757988 | 1.39/2.22/2.75 → 3.37/2.63/2.85 |
| New 100k peak RSS, KiB | 712,380.000000 | 702,641.333333 | 719,664 / 698,128 | 697,408–720,068 / 697,564–712,232 | 0.986329393488 | 1.39/2.22/2.75 → 3.37/2.63/2.85 |
| Compiler wall, s | 20.173333 | 20.173333 | 20.27 / 20.12 | 19.97–20.28 / 20.10–20.30 | 1.000000000000 | 3.37/2.63/2.85 → 2.21/2.52/2.79 |
| Compiler CPU, s | 19.803333 | 19.843333 | 19.82 / 19.80 | 19.72–19.87 / 19.79–19.94 | 1.002019861976 | 3.37/2.63/2.85 → 2.21/2.52/2.79 |
| Compiler peak RSS, KiB | 748,710.666667 | 742,933.333333 | 749,748 / 742,076 | 745,260–751,124 / 740,940–745,784 | 0.992283623580 | 3.37/2.63/2.85 → 2.21/2.52/2.79 |

These are descriptive results from three observations per producer on each of two separate inputs. New-100k mean wall is 0.895736% lower for B; compiler mean wall is equal, with exact A/B sums of 60.52 s. CPU changes are −0.036724% and +0.201986%, respectively; mean peak RSS changes are −1.367061% and −0.771638%. The alternating order and load records expose the actual observations; they do not establish significance or allocate those RSS differences to the two vectors. No 10% generated-code threshold applies to this frontend work-removal change.

The comparison is closed. These results do not justify rerunning the 1M refusal, establish a new scaling exponent, or meet a native/Rust-O milestone. Whole-source loading/inference, shared graph identity and global packaging remain measured size walls.

## Result acceptance and recovery

The [independent result audit](compiler-reference-capture-data/comparison-acceptance/README.md) verifies all 26 intervals, all 501 raw files, all 25 setup pins, all 1,912 live input members, exact Git content for the 1,742 compiler members and complete output equality. Integer centiseconds and rational coordinates reproduce every sum, mean, median, range and ratio. Its successful offline pass takes 1.44 s wall /1.41 s CPU, 131,484 KiB peak RSS, load 1.01/1.06/1.63 → 1.17/1.09/1.64.

Two earlier audit readers stopped on format assumptions: a stored RSS digit string versus its numerically serialized summary, then the outer launcher's different GNU-time grammar. Their scripts and captures remain intact. Only the offline reader changed; no measured command or original helper reran. A final sealing command emitted a Perl print-spacing warning with exit zero; direct inspection confirmed the actual 67-record manifest. The complete 68-file audit is copied byte-for-byte, manifest SHA-256 `170be128353e235b30cb71fa29f261b468092432379b3b1486c6c857f8586159`.

The [result recovery package](compiler-reference-capture-data/comparison-recovery/report.md) covers both original roots: 501 files, 254,521,215 logical bytes and 32 directories. All 484 ordinary records are stored individually. Fourteen image paths recover from two actual representatives, with every alias compared in full. The three producer/seed paths recover from exact committed seed blobs and complete, byte-verified expansions.

One deterministic 5,237,981-byte archive contains 490 regular members: the 484 records, two actual images and four recovery metadata files. SHA-256 is `2451a0a876fc2ddf839e5774280a39c500500b17d65a494028c1a18002af1833`. Every member and original alias passes the byte/mode check. Packaging takes 3.50 s (load 1.00/1.08/1.51 → 1.08/1.10/1.51), peak RSS 418,864 KiB; compression runs once. The surrounding 36-file package has manifest `07e6f2c3cca42ef94261951399f99e32a699e470cee8d8001cfb33303f050935`. Root independently verifies every copied package file; [copy coordinates](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-reference-capture-data/comparison-retention.json) retain both complete populations. The original comparison roots were subsequently removed by the final guarded cleanup below.

The preliminary recovery archive contains all seven preparation/desired/check roots: 1,884 regular files, 50 directories and 42,874,424 logical bytes, including all three failed builds, original source.tar and the actual successful 21,095,338-byte semantic image. Every member was byte-compared once. Packaging takes 1.32 s (load 4.02/3.84/3.78 unchanged); the archive is 7,839,018 B. Its ancillary seal display repeated a consumed digest finalizer, but the stored manifest is correct and root reverified every copied file. No archive rerun or artifact correction was needed.

Completed bootstrap/gate disposable stores have separately been removed: 16,063 regular files, 568,814,894 bytes and six directories, in 0.38 s (load 1.40/1.71/2.17 unchanged), after complete fixed-scope metadata preflight. Cache payloads were neither read nor archived. The [independent cleanup acceptance](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-reference-capture-data/cache-cleanup-acceptance/acceptance.json) verifies both absences and unchanged canonical bytes. All retained gate and query evidence stays recoverable.

During preparation for the remaining evidence cleanup, root found that a subordinate's older recovery mapper and its syntax check actually used Python 3, contrary to the explicit no-Python instruction. This is an instruction violation, not an approved tool choice. Its [original source and captures](compiler-reference-capture-data/cleanup-mapping/copy-coordinates.json) are preserved as evidence and will not be executed again. The source copy is non-executable data ending in `.py.txt`; its original `.py` path and actual interpreter remain explicitly recorded. The separately reviewed [Perl guard](compiler-reference-capture-data/cleanup-preparation-2/README.md), committed at `ce05f3a1`, independently verifies all named committed recoveries, complete populations, stored metadata and full live bytes before its first unlink; the Python-derived verdict is not trusted. Its single execution removes exactly 29 completed roots, 3,050 regular files and 160,418,647 logical bytes, with all roots absent by ENOENT. It takes 29.55 s wall, 25.82 s user /3.24 s system CPU, 303,264 KiB peak RSS, load 1.11/0.99/1.11 → 1.69/1.14/1.16. [Raw execution](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-reference-capture-data/cleanup-execution-2/guard-result/result.json) and the outer capture are retained; canonical producer bytes are unchanged. Removal is enumerated and cooperative, with no atomic multi-root or hostile-path-race guarantee. The frozen map has no historical nanosecond fields, and none are invented. No production source or measured comparison command used the prohibited mapper.

The [independent cleanup acceptance](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-reference-capture-data/cleanup-acceptance-2/acceptance.json) verifies all 6,370 progress events, exactly 3,050 unlink intent/completion pairs, 119 removed directories including the 29 roots, all current absences and the unchanged 32,588,975-byte canonical producer. All 29 root preflights and the complete preflight precede the first unlink intent. The five excluded scopes remain present. The audit takes 1.36 s, peak RSS 66,648 KiB, load 0.81/0.88/1.01 unchanged. Its first reader stopped because root had added the five exact retained result copies after the auditor's initial listing; the preserved second reader checks that sole added subtree in full. The cleanup itself did not rerun.

Before final cleanup, the seven remaining completed comparison/input scopes received [exact pinned recovery](compiler-reference-capture-data/final-recovery-1/report.md) for all 4,468 files, 342,778,212 bytes and 141 directories. Their independent Perl mapping takes 3.35 s (load 0.66/0.91/1.12 → 0.69/0.91/1.12), peak RSS 344,196 KiB, with no new compression or unresolved file.

The separately reviewed [final finite guard](compiler-reference-capture-data/final-cleanup-1/README.md), pinned at `6114aa74`, completes once after checking every recovery and original byte before the first mutation. It removes ten explicit directory roots and nine literal top-level files: 4,532 regular files, 354,054,095 logical bytes and 145 directories. Wall is 7.04 s, user/system CPU 4.53/2.40 s, peak RSS 349,436 KiB, load 12.21/6.40/3.39 → 11.47/6.34/3.38. The actual release, command, complete progress stream and result remain in [final-cleanup-1](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-reference-capture-data/final-cleanup-1/execution-1/result.json); no cleanup command is retried.

The [independent final acceptance](compiler-reference-capture-data/final-cleanup-acceptance-1/README.md) verifies all 9,367 events, including every adjacent file/directory intent and completion, complete preflight before the first mutation, and all 19 current literal ENOENT results. No pending mutation remains. Canonical `b01eabfa` stays 32,588,975 bytes with identical complete digests. Its sole auditor run takes 2.24 s, CPU 2.20 s, peak RSS 68,564 KiB, load 1.23/1.71/2.26 unchanged. This post-removal audit relies on the pinned guard and its successful full-byte preflight; it does not claim to reread deleted payloads. The cooperative partial-stop contract, whole-second historical metadata and absence of an injected failure remain explicit. The 18-file acceptance package is frozen by manifest `7f66e7a5c65810eea44630e37339991fc193d359d371fade25e30d2f56adc3ec`.

All source, publication, inputs, results and failures are retained under compiler-reference-capture-data. Global keys, unrelated operation credentials and live services remain excluded.
