# Scalar cohort: independent acceptance

The completed cohort is accepted as a retained observation. The scalar candidate misses the frozen 10% threshold and is to be discarded without broadening or another trial.

Release: `c887b7c5aca7bb80d70ca26d7dfdf1a44213a163`. Candidate source: `5b5ce7759c035ae0437e3512e18efc440aa50d2e`. Producer: `f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64` (32761734 bytes).

| Phase | GNU wall s | CPU s | Peak RSS KiB | Load before → after |
| --- | ---: | ---: | ---: | --- |
| construct.A.1 | 24.68 | 24.16 | 793080 | 1.36 1.51 2.07 → 1.66 1.56 2.07 |
| construct.B.1 | 32.41 | 31.78 | 1169088 | 1.66 1.56 2.07 → 1.81 1.61 2.07 |
| execute.A.1 | 15.01 | 14.73 | 526328 | 1.81 1.61 2.07 → 1.77 1.61 2.06 |
| execute.B.1 | 15.13 | 14.76 | 523792 | 1.77 1.61 2.06 → 1.55 1.57 2.04 |
| execute.B.2 | 15.15 | 14.91 | 519656 | 1.55 1.57 2.04 → 1.43 1.54 2.02 |
| execute.A.2 | 15.22 | 14.89 | 521184 | 1.43 1.54 2.02 → 1.68 1.59 2.03 |
| execute.A.3 | 15.31 | 15.03 | 523724 | 1.68 1.59 2.03 → 2.00 1.66 2.04 |
| execute.B.3 | 14.94 | 14.64 | 527144 | 2.00 1.66 2.04 → 1.92 1.66 2.04 |

All three execution observations give A mean 15.180000000 s and B mean 15.073333333 s: B/A 0.992973210365, an observed 0.702678964% wall reduction. The exact test is **452200000000 ≤ 409860000000**, which is false. Runtime CPU sums are 44.65 s and 44.31 s. Medians 15.22 s and 15.13 s remain secondary.

A and B images are 22937783 and 22929591 bytes: delta -8192 bytes (-0.035714001%). Construction wall increases 31.320908%, CPU 31.539735% and peak RSS 47.411106%; these are separate from execution and do not isolate individual rewrite/recheck costs.

B admits one scalar rewrite: 16016 constants, 389 identities and 18 branches in 3324 functions; zero scalar rejections, message mismatches or trap guards. Both constructions admit the same inlining population. These counts do not prove removable runtime cost.

The audit joined all eight raw captures and verified all six full 16228028-byte outputs, the exact Git seed oracle, 172 archive files, 171 frozen src/library files, 218 current candidate files and 216 current main files. Input and output manifests, committed release blobs, passed validation summaries and current emitter bytes agree. Construction stderr contains exactly one statistics record each; runtime stdout/stderr are empty.

The result is one fixed-order A1/B1/B2/A2/A3/B3 cohort, with no cache reset or randomized causal claim. Candidate CI remains pending until/if adoption; baseline CI is context only. GNU values have their recorded centisecond resolution and are not replaced by nanosecond wrapper durations. Source bundle retention was compared, without a fresh-repository reconstruction. No measured program, benchmark or compiler was rerun, and nothing was compressed, deleted or modified outside this audit directory. The first auditor assertion-helper failure is retained separately; its scalar-context correction did not modify the cohort.
