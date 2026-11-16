# Independent acceptance: copy-site cohort 2

Accepted the completed once-only cohort after one independent, data-only audit. No discrepancy was found.

Release commit: 88789cac357a455f8e752e6d820f59ce3cc38fb7.
Release JSON SHA-256: cdf8f17528800d6d152e8f7986e389f37da9712bbcd740547b59452e639f692b.
Launcher SHA-256: abf52e1d5f22370a999e6e7765c4b28fe34671ff16430aa93da8d7112fd71bd6.
Producer SHA-256: 6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc.

Inputs were restricted to /home/coder/workspace/lkjscript/target/compiler-copy-sites-evidence.20261116-2, its held run directory, the exact release inputs, and their pinned Git objects. No compiler, measured image, official profile reader, frozen accounting helper, extractor, extra detail page, cleanup, or later census input was executed or opened.

## What the audit established

- All seven raw argv/cwd/status/GNU-time/load/UTC/nanosecond records match their complete capture records. The commands have the exact fixed source/library/runtime selectors and their required inline/no-store flags. Every wrapper applies 4 GiB / 900 s / 10 s; every command and capture exits zero; phase intervals are sequential.
- All 175 evidence files and 186 held files have identical before/after inventories. The 172 frozen witness inputs match both captured inventory tables and Git blobs at 26bf2c3c021dd79810a81fa1d9054d59f3768b06. All 216 current src/library files match the index, whose committed trees equal the producer trees.
- The canonical producer and copied producer have the released identity. The pinned producer seed decompresses to that identity. The witness seed decompresses byte-for-byte to the held oracle, and both measured output images equal the oracle exactly: 16,228,028 bytes, SHA-256 8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132.
- Both full compressed artifacts were independently decoded and compared byte-for-byte with their held originals. Recorded gzip/gunzip/hash statuses, lengths and digests also agree.
- Every raw symbol, kind interval, copy row and selected-function map entry was parsed independently. Section extents end exactly at EOF; code/symbol/copy intervals and origin/role flags satisfy the format; copy intervals remain within their mapped function and existing KIND_COPY region.
- All six reconstructed TSVs are byte-identical to the recorded accounting outputs, including zero-count rows, every exact-width/role/flag bucket, all selected-function slots and explicit unmapped rows. The seven-record accounting manifest matches its complete population.
- The sole official 200-row detail page matches the exact globally sorted raw prefix and every displayed numerical field. Displayed function/type names match the companion declarations. Summary integer marginals and top-function ordering agree. No second detail page was requested.

## Raw accounting

| Quantity | Exact value |
| --- | ---: |
| Serial S / completed T / function self total | 14,658 / 14,658 / 14,658 |
| KIND_COPY K / mapped / detail D | 1,503 / 1,503 / 1,503 |
| Unmapped / serial gap / detail-leading in-flight | 0 / 0 / 0 |
| Copy sites / nonzero sites | 68,092 / 822 |
| Selected-function slots / symbols | 10,072 / 5,893 |
| Kind intervals | 3,106,452 |
| Visible rows / their samples | 200 / 825 |

The observed KIND_COPY marginal is 1,503 / 14,658 = 10.253786328%. This is the new cohort's observation, not a reconstruction or causal change from the historical 10.369% marginal. The 825 visible samples are not a whole-function census; all unshown raw sites for selected functions must be joined explicitly before any subsequent coverage claim.

## Observed cost, one fixed-order pair

CPU is user plus system time. Loads below retain all three observed load averages. Summary, detail and accounting are separate later phases and are excluded from the construction/execution pairs.

| Phase | Wall s | CPU s | Peak RSS KiB | Load before | Load after |
| --- | ---: | ---: | ---: | --- | --- |
| Ordinary construction | 23.86 | 23.57 | 789,660 | 3.11 / 2.81 / 2.76 | 3.31 / 2.88 / 2.78 |
| Profile construction | 28.02 | 27.71 | 990,972 | 3.31 / 2.88 / 2.78 | 2.83 / 2.80 / 2.76 |
| Ordinary execution | 15.20 | 14.80 | 521,892 | 2.83 / 2.80 / 2.76 | 2.51 / 2.73 / 2.73 |
| Profile execution | 14.95 | 14.69 | 615,192 | 2.51 / 2.73 / 2.73 | 2.25 / 2.66 / 2.71 |
| Official summary | 0.26 | 0.24 | 64,608 | 2.25 / 2.66 / 2.71 | 2.25 / 2.66 / 2.71 |
| Official copy detail | 14.64 | 14.42 | 578,080 | 2.25 / 2.66 / 2.71 | 2.74 / 2.74 / 2.74 |
| Frozen accounting | 4.95 | 4.85 | 136,320 | 2.74 / 2.74 / 2.74 | 2.60 / 2.71 / 2.73 |

Profile/ordinary construction ratios are wall 1.174350377, CPU 1.175647009 and RSS 1.254935035. Execution ratios are wall 0.983552632, CPU 0.992567568 and RSS 1.178772620. Image size changes from 22,936,514 to 78,470,677 bytes (3.421212003 times). Both constructions accepted the same inlining attempt: 30,619 calls, 5,617 functions, zero rejection; store-level reuse fields are zero. The shared 4,455 checked_local_reused count is in-memory reuse, not persistent store reuse.

The slightly lower profiled execution time is not evidence of speedup or reliable negative overhead. Each comparison contains one ordinary-then-profile pair under changing shared-host load and cache state. Construction includes all profiling machinery and selected-IR publication; it does not isolate the incremental cost of the joint copy table. No optimization gain, fresh Rust ratio or confidence interval is established.

## Retained artifact identities

| Artifact | Raw bytes | Raw SHA-256 | Gzip bytes | Gzip SHA-256 |
| --- | ---: | --- | ---: | --- |
| Complete profile | 55,464,197 | 7849d4dd69906491d07664f493eb0f4fd8340cfde3760f6e4f168dbbde95f302 | 9,668,885 | 94f564f3cb73ff5275493adb725b55f06aa7c47e2acd9442d00ef8ab2863d03b |
| Exact selected IR | 57,649,044 | 32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca | 9,701,300 | 43f48575397f144c8640f9b2db90d14cf6e07b51d075404dd3e4eb4dfe6863c1 |

The 72-byte copy rows occupy 4,902,624 bytes and the 16-byte function map occupies 161,152 bytes. Adding the 80-byte header growth gives 5,063,856 bytes relative to an otherwise equal v3 encoding. This is format arithmetic, not an observed old/new image delta.

## Proof and limitations

acceptance.json is the full machine-readable result (SHA-256 adfb9a9837ec044cc1a03cec186a7ada643300bab7efe79b324df6e597bddcbe). audit.pl is the independent parser/hash/retention auditor (SHA-256 8f0ebc237cddc22b4a488a86d8362c10bcdfd3b4f52f56a75114ed0bf2c7a4a1). The observed/ directory retains all non-gzip evidence bytes; reconstructed/ retains the six independently reproduced tables. The complete gzip files remain in the measurement evidence directory and are bound by the evidence inventories.

The independent audit ran once: exit 0, empty stderr, 8.53 s wall, 7.63 s user, 0.39 s system, 182,856 KiB peak RSS, load 13.02 / 4.77 / 3.33 -> 11.85 / 4.80 / 3.35, under 4 GiB / 300 s / 10 s. These are validation costs, not additional compiler timings. Raw argv/cwd/time/load/status and gateway completion are retained.

Some initial read-only gateway read_file requests returned HTTP 429/internal transport errors. inspection-transport-failures.json preserves them. The later batched reads succeeded; these were inspection failures, not cohort phases or retries. No measurement was repaired or rerun.

IR admission, operand-role semantics and layout remain the separately retained official detail command's result. This auditor rechecked binary/accounting structure and declaration names, not the IR checker. No eligibility, ownership/loan recovery, SROA opportunity, sample-times-width traffic or removable-time claim is made. Hashes and inventories identify observed bytes without a freshness or kernel-sealing claim.

Evidence inventory SHA-256: 30042317e420b1db9919ad7af4f4561f2731ec59b3414e3f2385be6300d88726 (175 files; root independently obtained the same inventory while retaining the measurement bundle). Held-run inventory SHA-256: 76e5543bab3a09bd542f54d64982796dd9bcc2f6f3018e7f2f6f2ebe6d41d60a (186 files). No cleanup is authorized by this acceptance.
