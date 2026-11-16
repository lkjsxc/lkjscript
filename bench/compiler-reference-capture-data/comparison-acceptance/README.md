# Independent acceptance of reference-capture comparison 2

Accepted the completed cohort from release `0547c7041de0710dbe6e3e98b590a9231fc32329`.
This package is an independent offline audit of the two frozen roots
`target/compiler-reference-capture-comparison-2` and
`target/compiler-reference-capture-comparison-launch-2`.
No compiler, native image, original trial helper, compression, observer, credential or store payload operation ran.

## Exact result

All 12 compiler commands and 14 metadata commands have complete, successful raw captures. Their 26 intervals are sequential, do not overlap, and lie inside the outer launch capture. Payload argv, wrapper argv, cwd, stdin, free-space record, status, GNU time, nanoseconds, UTC and load match the declared recipe and stored rows. The command order is new-100k A1 B1 B2 A2 A3 B3, then compiler A1 B1 B2 A2 A3 B3. Each compiler stdout is empty; each stderr has exactly one successful `LKJSCRIPT_STATS` object. Reuse and inline counters are zero. Every emitted size matches its retained file.

The audit verifies 501 raw evidence files, 25 setup pins, and 1,912 live input members against byte-identical before/after/setup inventories. All 1,742 compiler input members also match exact Git blobs at `c887b7c5aca7bb80d70ca26d7dfdf1a44213a163`. The generated input has 169 source files, 100,000 lines and 3,923,070 source bytes, plus its separate workload metadata file. Existing accepted archive recovery is bound by exact committed archive hashes; this audit does not decompress them.

All 12 retained phase outputs and two final reused output slots compare in full bytes to the respective A1:

| Input | Image bytes | SHA-256 |
|---|---:|---|
| new-100k | 4,468,259 | `42443b75fe211c9f9efa7c0ee26c9233280c38a124ec5e5b514666aab97ce361` |
| compiler | 21,489,004 | `5ab411b7fb91498adc1453a08d96893f85c1d644a2e050b93b3e9dc850585597` |

Producer A remains `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc` (32,584,542 bytes), source `d461c2d98e93aeed6061508ee80ab36b9eeaa04f`. Producer B remains `b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db` (32,588,975 bytes), source `bd8a9050a3786de9f227c6c4037f5acd43205f71`. Both source trees, the common library tree and committed seed identities match the release. The candidate's retained gzip is byte-identical to seed commit `e5294c339d9aefbbafc2e951c997f4fefabcc9ad`. The paired source/gzip identity is the accepted bootstrap contract; this audit does not rerun a seed expansion.

Release JSON `6ea5952f0c0f366f86a829a85bb82292d8a9a656cd488d52305287eaae63d3ff`, control manifest `3c3bb9864c1d57cbb44d5f1f56923a3d230971d9121bac957ca0861c49fe1fc3`, all five controls, and the five cited setup/input/gate/CI records match their committed bytes. The actual release records completed CI run 43925403668 at head `1148e56a2a324e2bbe0e92cfdb4762025519cd5c`, both jobs successful; this is the pretrial observation, not a fresh CI request. The original cohort remains closed with all twelve measurement rows not run.

## Recomputed measurements

GNU hundredths are parsed as exact integer centiseconds. Integer sums and rational mean/ratio numerators and denominators are retained in `acceptance.json`; floating-point formatting is used only for displayed decimal summaries. Both inputs remain separate.

| Input | Wall sums A / B (s) | Wall means A / B (s) | Medians A / B (s) | Ranges A / B (s) | Mean B/A | Input interval load 1/5/15 min |
|---|---:|---:|---:|---|---:|---|
| new-100k | 27.91 / 27.66 | 9.303333 / 9.220000 | 9.13 / 9.10 | 9.11–9.67 / 9.09–9.47 | 0.991042637048 | 1.39 / 2.22 / 2.75 → 3.37 / 2.63 / 2.85 |
| compiler | 60.52 / 60.52 | 20.173333 / 20.173333 | 20.27 / 20.12 | 19.97–20.28 / 20.10–20.30 | 1.000000000000 | 3.37 / 2.63 / 2.85 → 2.21 / 2.52 / 2.79 |

new-100k: CPU sums A/B 27.23 / 27.22 s; peak-RSS sums 2137140 / 2107924 KiB and means 712380.000000 / 702641.333333 KiB. The same input-level load interval and the individual loads below apply.

compiler: CPU sums A/B 59.41 / 59.53 s; peak-RSS sums 2246132 / 2228800 KiB and means 748710.666667 / 742933.333333 KiB. The same input-level load interval and the individual loads below apply.

The new-100k mean wall observation is 0.895736% lower for B, while the compiler mean wall observation is equal. Mean peak RSS is 1.367061% lower for new-100k and 0.771638% lower for compiler. CPU changes are −0.036724% and +0.201986%, respectively. These are descriptive observations from one fixed alternating sequence with three samples per producer. They are not significance estimates, an allocation-cause diagnosis, a result for the 1M wall, or a generated-code speed claim. No 10% threshold applies. Operating-system caches were not reset; preflight reads and post-build validation may warm caches.

| Phase | GNU wall (s) | CPU (s) | Peak RSS (KiB) | Load before → after (1/5/15 min) |
|---|---:|---:|---:|---|
| 01-new-100k-A1 | 9.67 | 9.27 | 697408 | 1.39 2.22 2.75 → 1.71 2.27 2.75 |
| 02-new-100k-B1 | 9.47 | 9.26 | 698128 | 1.71 2.27 2.75 → 2.05 2.32 2.77 |
| 03-new-100k-B2 | 9.10 | 8.99 | 697564 | 2.05 2.32 2.77 → 2.35 2.38 2.78 |
| 04-new-100k-A2 | 9.13 | 8.98 | 720068 | 2.35 2.38 2.78 → 2.98 2.52 2.82 |
| 05-new-100k-A3 | 9.11 | 8.98 | 719664 | 2.98 2.52 2.82 → 2.97 2.53 2.82 |
| 06-new-100k-B3 | 9.09 | 8.97 | 712232 | 2.97 2.53 2.82 → 3.37 2.63 2.85 |
| 07-compiler-A1 | 20.28 | 19.82 | 749748 | 3.37 2.63 2.85 → 3.50 2.72 2.88 |
| 08-compiler-B1 | 20.10 | 19.80 | 745784 | 3.50 2.72 2.88 → 3.10 2.67 2.86 |
| 09-compiler-B2 | 20.30 | 19.94 | 740940 | 3.10 2.67 2.86 → 2.99 2.68 2.86 |
| 10-compiler-A2 | 19.97 | 19.72 | 751124 | 2.99 2.68 2.86 → 2.63 2.61 2.83 |
| 11-compiler-A3 | 20.27 | 19.87 | 745260 | 2.63 2.61 2.83 → 2.49 2.59 2.82 |
| 12-compiler-B3 | 20.12 | 19.79 | 742076 | 2.49 2.59 2.82 → 2.21 2.52 2.79 |

Outer capture: 179.66 s GNU wall, 176.40 s CPU, 751,124 KiB peak RSS; load 1.39 / 2.22 / 2.75 → 2.21 / 2.52 / 2.79. Metadata commands and output checks are outside the twelve compiler timings.

## Audit retention and reader corrections

The successful reader is `audit-v3.pl`, SHA-256 `c68b2ca6c0ec487221e2b606267a74163e7b1c14ab745d3aae3a1f39768d3bc0`. Its single completed pass took 1.44 s wall / 1.41 s CPU, 131,484 KiB RSS, load 1.01 / 1.06 / 1.63 → 1.17 / 1.09 / 1.64. `phase-3` retains exact syntax/run argv, raw output, status, timing and load.

Both earlier offline reader stops remain intact. V1 required JSON type identity for the raw digit-string `max_rss_kib` that the frozen completer intentionally serializes as a number; V2 normalizes only that strictly validated unsigned field. V2 then applied the per-command multiline GNU grammar to the outer launcher's different single-line grammar; V3 reads the exact outer format. These were audit format corrections, not measurement failures or retries. All three readers and captures are retained. No original result was changed.

Primary proof: `acceptance.json`, SHA-256 `06d025264e70b890f144e62452849fef42d131254b05b457a119ce2825957ce2`.
`rows.json`/`rows.tsv` retain each independent observation; `timeline.json` retains exact integer interval coordinates; `raw-inventory.json` binds all 501 original evidence files; `observed-inputs.json` also binds source and artifact reads; `git-read-commands.json` records the Git-only provenance reads. The final manifest binds this entire audit package. Raw recovery packaging is a separate retained operation.
