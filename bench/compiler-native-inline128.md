# Producer inline128: fixed-source follow-on trial

**Discard inline128; retain producer64.** Across all six predeclared runtime observations, mean wall time increased from **14.906667 s to 15.073333 s**, a **1.118068% slowdown**. Mean CPU time increased from **14.656667 s to 14.853333 s**, a **1.341824% slowdown**. The runtime load window was 2.76/3.91/3.52 → 2.26/3.51/3.42; every individual load pair is below.

All six outputs were byte-identical to the same frozen original seed. Both constructions and every execution succeeded on their first attempt. The candidate did not meet the predeclared ≥10% mean-wall reduction rule, so the conditional original-IR oracle was **not run**. No compiler, optimizer, bootstrap, default-output or seed-policy change follows from this measurement.

This tests one specified increase of the existing rechecked inliner, from 64 to 128, motivated by the remaining call-boundary work in [the attribution pair](compiler-profile-inline64.md). The [accepted inline64 campaign](compiler-native-inline64.md), earlier inline16 miss and these observations remain separate. No threshold was searched or retuned after data.

## Every construction and runtime observation

The first two rows construct the producers. The remaining six execute them in the fixed order **64,128,128,64,64,128**, always compiling the same frozen computation with output `--inline-limit 0 --no-store`. The tables preserve all observations, including the one pair in which 128 was slightly faster.

Wall is GNU time's elapsed duration around the `prlimit/timeout` invocation. Raw launch-inclusive nanosecond captures are also retained. Load is the 1/5/15-minute average immediately before and after each phase. Root's optional Rust witness was an announced concurrent worker; its progress was not used to select, delay or repeat these rows.

| Phase | Policy/repetition | Wall s | User s | Kernel s | Peak RSS KiB | Load before → after |
|---|---|---:|---:|---:|---:|---|
| Construct | 64 | 28.53 | 27.81 | 0.23 | 764,108 | 2.69/4.15/3.57 → 2.92/4.05/3.55 |
| Construct | 128 | 33.07 | 32.10 | 0.27 | 888,020 | 2.92/4.05/3.55 → 2.76/3.91/3.52 |
| Execute | 64 /1 | 14.94 | 14.56 | 0.15 | 495,808 | 2.76/3.91/3.52 → 2.87/3.88/3.52 |
| Execute | 128 /1 | 15.17 | 14.78 | 0.15 | 503,732 | 2.87/3.88/3.52 → 3.06/3.87/3.52 |
| Execute | 128 /2 | 15.18 | 14.76 | 0.16 | 500,620 | 3.06/3.87/3.52 → 3.12/3.84/3.52 |
| Execute | 64 /2 | 14.84 | 14.45 | 0.15 | 497,144 | 3.12/3.84/3.52 → 2.73/3.72/3.48 |
| Execute | 64 /3 | 14.94 | 14.50 | 0.16 | 496,828 | 2.73/3.72/3.48 → 2.35/3.58/3.44 |
| Execute | 128 /3 | 14.87 | 14.56 | 0.15 | 502,264 | 2.35/3.58/3.44 → 2.26/3.51/3.42 |

The paired wall reductions for repetitions 1/2/3 are −1.539491%, −2.291105% and +0.468541%. The decision uses all three observations per policy: their wall sums are 44.72 s and 45.22 s. Integer centiseconds implement the predeclared comparison, candidate sum × 10 ≤ baseline sum × 9; here 45,220 > 40,248. CPU means are secondary descriptive evidence.

Construction wall grew **15.913074%**, image size grew **23.729474%** and construction peak RSS grew **16.216556%**. These costs are separate from the runtime decision.

| Constructed producer statistic | Limit 64 | Limit 128 |
|---|---:|---:|
| Image bytes | 22,940,180 | 28,383,764 |
| Inlined calls | 30,619 | 33,323 |
| Changed functions | 5,617 | 6,586 |
| Reused local checks | 4,455 | 3,486 |
| Accepted/rejected transformations | 1 /0 | 1 /0 |
| Inlining stage ms | 596.317 | 770.102 |
| Aggregated body-check stage ms | 14,538.389 | 18,889.897 |

The construction phase load pairs above also apply to these internal stage measurements. Both stats records describe an original input of 10,072 functions and 928,672 IR instructions; those are not expanded-instruction counts. Limit 128 expands 2,704 additional calls in 969 additional changed functions, but this campaign does not establish a runtime benefit from them.

## Exact identities and common inputs

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| Common emitter | 20,627,356 | `4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09` |
| New producer64 | 22,940,180 | `99f8c19328045fe682897fda1327797538b308466ce320d6c7a3edcadfd96cb6` |
| New producer128 | 28,383,764 | `23fd391b93b0609f3115a255029751c825ecad2e9f03d72044e380d4b5e4bb7b` |
| Original expected seed and all six outputs | 16,228,028 | `8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132` |

The isolated worktree began at `d4d6911a99fc9f3daa6c6994f2898a5ecc528ad8`. The common emitter was decompressed directly from `a4fd685a9bb2e7736110a4ed2a43651dfe040d56:seed/lkjscript.gz`; the compressed archive hash is `6c4b72837d4982c1c90c24ee8212c0934f6a5b836bb7d06194f5a894be3cf306`. The mutable main-worktree target was never used.

Both constructed producers and their computation use revision `26bf2c3c021dd79810a81fa1d9054d59f3768b06`, source tree `3431671024d0a550b3130f2401bde067026d9bd3` and library tree `e442fb3920db7eda2377f92fde0d92761f0562be`. Source is literally `src/main.lkj`; library and runtime selectors point to the same archived physical tree. Runtime-source SHA-256 is `eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14`. All 171 actual archived source/library files were checked against their frozen Git blobs after the campaign.

### Why this reconstructed producer64 is 282 bytes larger

The earlier accepted 64 campaign's producer was 22,939,898 bytes. This newly constructed baseline is 22,940,180 bytes and has its own digest. It must not be treated as the same executable merely because its logic, emitter and inlining policy are frozen.

The retained metadata and exact construction argument lists agree after replacing only their fixture-directory prefixes. The previous source root has 104 bytes; this source root has 106. Both current producers contain a decoded native diagnostic table with 141 pointer/length filename entries. Every path is rooted in the current source directory, maps to a verified frozen source/library file, and the complete path lists are identical between 64 and 128. No occurrence of that fixture root lies outside this table.

| Native data property | Producer64 | Producer128 |
|---|---:|---:|
| RW segment file offset | 22,593,536 | 28,037,120 |
| RW segment bytes | 346,644 | 346,644 |
| Diagnostic table file offset | 22,598,425 | 28,042,009 |
| Filename pool start/end | 22,600,689 /22,619,182 | 28,044,273 /28,062,766 |
| Filename entries | 141 | 141 |

The emitter source at `ec3e2ce8:src/native.lkj` appends the program/runtime source list through `native::diagnostics::data`; its `table` function writes pointer/length entries followed by the path bytes. The measured root-length difference therefore accounts for **141 × 2 = 282 bytes**, exactly the observed footprint difference. The companion audit preserves all table offsets/paths, normalized arguments and verified source blobs.

The prior producer binary was cleaned after its accepted campaign, so this is not an old/new binary comparison and does not assert byte identity outside a replaced string range. The internally paired comparison here uses the same current physical paths in both producers; all six measured output images independently equal the original frozen seed.

## Predeclared oracle and resource policy

Only a candidate with at least 10% reduction in arithmetic mean wall time was to run the frozen original-IR optimizer oracle. The oracle script was archived unchanged, SHA-256 `53b16740dfc28ba7a46bcf35d6d96f186d992b34ad15e4295d3610c575a6ed67`. A predeclared wrapper translates only the exact optimized argument pair `--inline-limit 64` to `--inline-limit 128`, retaining per-invocation translation counts and actual arguments. Zero/default commands and mutation seed 1701/count 128/jobs 1 are unchanged; frozen script labels can still say 64.

Because the threshold was missed, the wrapper and oracle were never executed. No broader original-IR admission/value/trap/cache/self-image/mutation-oracle success is claimed for 128. The metadata records the conditional oracle configuration; the actual decision record says `oracle=not-run-below-threshold`.

The harness fixed one worker, a 4 GiB external address-space ceiling, 900 s per measured command and a 10 s termination grace. These are host experiment limits, not language policy. Free space before generation was 116,157,652 KiB, above the 3 GiB preflight requirement. File caches were not reset. There were no refusals, retries, profiler runs, hash microbenchmarks or new Rust optimization-level measurements.

## Interpretation and retained evidence

This single higher-threshold hypothesis failed the retention rule. The small measured slowdown is not a claim that 128 is always slower on every host or program. Additional inlining changed code size and checking work, but no 128 profile was collected, so instruction-cache pressure, register pressure and individual transformations are not assigned causal shares. The result supplies neither a new Rust-`-O` milestone nor a backend floor. Producer64 remains the accepted policy.

The predeclared protocol and final harness received independent read-only review before execution. All 8 measured phases exited 0; all 6 output comparisons passed and execution stdout/stderr were empty. Original rows are unchanged.

Durable files are `compiler-native-inline128.tsv`, `.meta`, `-parity.tsv`, `-raw.log`, `-source-blobs.tsv`, `-paths.tsv` and `-path-audit.log`, alongside `compiler_native_inline128.sh`. The raw log includes the exact original protocol text, its launch hash, all arguments/captures/times/stats, the decision/outcome, and the read-only audit script. The source-blob table has path, expected Git object and actual Git object columns; the path table includes both producers' exact diagnostic filenames and offsets.

Harness SHA-256 is `07f65111d669dacd130e7f8a3a0fcf1cc1e1f688f21d16185600eb7516f09519`; the original pre-run protocol hash is `1f1f87b02b4b899fdf994237ed173468608f0789c3d594fbd7cef0e8cadc7671`. Reproduction uses `bash bench/compiler_native_inline128.sh --results NEW_PREFIX`; it refuses existing evidence or any different pinned emitter/seed.

Root independently accepted the report, means/decision, actual emitter/producers/all six output hashes and path/source manifests. Guarded cleanup then removed the owned fixture, and its absence was verified. Its archived source, seeds, emitter, both producers, all six outputs and unused oracle wrapper are gone; textual evidence is unchanged. No worker is active and no further compute work is planned. This measurement commit changes no compiler or production policy.
