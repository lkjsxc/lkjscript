# Native compiler producer inlining

**Discard this trial.** Building the compiler producer with inline limit 16
reduced the six-sample mean from **18.910 to 17.580 seconds (7.03%)**. Mean CPU
time was **18.553 to 17.263 seconds (6.95%)**. Both reductions are below the
ten-percent continuation rule. All six preselected observations are retained,
including the slower final inline16 run; no runtime sample was repeated.
The compiler's default policy and implementation remain unchanged.

## Controlled computation

Both executables were constructed by the same retained emitter from the exact
frozen source used by [compiler_attribution.md](compiler_attribution.md).
Their construction policies were 0 and 16. Every measured execution then built
that same frozen source with **`--inline-limit 0 --no-store`**. Thus the execution
comparison excludes the cost of constructing and admitting an optimized output.
All six outputs were byte-identical to the original **16,228,028-byte** image.

| Input | Identity |
| --- | --- |
| Frozen revision | `26bf2c3c021dd79810a81fa1d9054d59f3768b06` |
| Source tree | `3431671024d0a550b3130f2401bde067026d9bd3` |
| Library tree | `e442fb3920db7eda2377f92fde0d92761f0562be` |
| Retained emitter SHA-256 | `558249941382f089a33535176972f81ee04067970645bffc7944c3b09cabfd88` |
| Every measured output SHA-256 | `8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132` |
| Inline0 measured producer SHA-256 | `cbc50c0353c37c349e17d3713a19b589b921b3761f05dc7c18cac1259371b0c2` |
| Inline16 measured producer SHA-256 | `6e50c5c8496151ffc4ff962f4381a9de2e2189d2453e18fe71e5480dae7ed9f3` |

Run one sequential worker:

```sh
bash bench/compiler_native_inline.sh --compiler target/lkjscript \
  --results target/compiler-native-inline.tsv
```

The harness archives the source, library, matching optimizer oracle and its
fixture corpus from the frozen revision. It checks for 3 GiB free space and uses
a 3 GiB address-space limit plus a 900-second watchdog per command, with a
ten-second termination grace. These are shared-host experimental limits.
It records exact input/output digests, construction statistics, wall/user/kernel
time, peak RSS and all three load averages. Exit cleanup removes owned archives,
executables, keys and oracle scratch data; selected result files remain.
The bounded recovery modes `--phase validation` and `--phase oracle` do not
repeat runtime samples.

## Runtime observations

Loads are 1/5/15-minute averages immediately before → after each command.
The order was fixed as 0,16,16,0,0,16 to avoid selecting favorable repetitions.

| Producer / sample | Wall / user / kernel s | Peak RSS KiB | Load |
| --- | ---: | ---: | --- |
| Inline0 / 1 | 17.77 / 17.42 / 0.10 | 540,016 | 19.63/24.61/17.40 → 15.40/23.32/17.13 |
| Inline16 / 1 | 16.15 / 15.80 / 0.09 | 539,004 | 15.40/23.32/17.13 → 12.79/22.36/16.91 |
| Inline16 / 2 | 16.29 / 15.89 / 0.10 | 541,688 | 12.79/22.36/16.91 → 10.39/21.19/16.64 |
| Inline0 / 2 | 19.79 / 19.23 / 0.14 | 536,104 | 10.39/21.19/16.64 → 10.15/20.60/16.52 |
| Inline0 / 3 | 19.17 / 18.64 / 0.13 | 532,548 | 10.15/20.60/16.52 → 8.97/19.64/16.29 |
| Inline16 / 3 | 20.30 / 19.74 / 0.17 | 509,460 | 8.97/19.64/16.29 → 11.12/19.44/16.29 |

The means above use these rows and their load windows; they are not additional
measurements. Host variation is visible in both wall and CPU times. This bounded
trial does not establish a repeatable ten-percent gain. Rust -O was not rerun,
so it does not replace the attribution report's paired-source ratios or establish
a 4× milestone or a backend floor.

## Construction and independent admission

These costs are paid to construct the producer, separately from its executions.

| Producer constructed | Wall / user / kernel s | Peak RSS KiB | Image bytes | Load |
| --- | ---: | ---: | ---: | --- |
| Inline0 | 19.65 / 19.11 / 0.21 | 490,392 | 16,197,592 | 30.26/26.89/17.78 → 24.32/25.76/17.60 |
| Inline16 | 24.81 / 24.12 / 0.20 | 668,812 | 16,893,912 | 24.32/25.76/17.60 → 19.63/24.61/17.40 |

The image grows **696,320 bytes (4.30%)**; construction wall cost grows 26.26%.
The original has 10,072 instances and 928,672 IR instructions. The untrusted
inliner removes 18,553 calls in 3,885 bodies. Independent checking accepts one
candidate, rejects none and reuses 6,187 exact unchanged local body proofs.
No stored proof is reused in either construction.

Within the construction load windows above, the inline transform takes
403.950 ms. The body-check timer aggregates original and candidate admission:
5,612.297 ms for inline0 and 10,804.882 ms for inline16. Their 5,192.585 ms
difference is an across-build observation, not an independently isolated
candidate-check duration. The separate `check_optimized_ms=0.008` is orchestration
after the nested body timer, so it must not be reported as the cost of rechecking.

## Exact-byte hashing

The [hash workload](hash.lkj) uses the compiler's 64 KiB read/update path and the
frozen source library. Each row makes twenty fresh file rewinds and SHA states;
every digest is checked against the host SHA-256 result. Internal clock time
excludes construction and startup. This is an isolated hashing cost, not a
one-shot semantic query.

Recovery reconstructed the two images in a fresh owned archive. Their sizes
match the measured producers, but their exact identities are separately recorded:
inline0 `2e1dec28e895add66214196622768f96d41a33059ff3e0f0c001093e1b15bd1c`;
inline16 `ce5f927fff4e07452a14d2f0c5376e82bcc8e035ed8d1b9122488c9bbae74ebe`.
The hash workload SHA-256 is
`51a60f70a211cccecb7222fe27b5ff72e1f93bf9fdcf33be0b8cb20f65286a25`.

| Hash code / input image policy | ms/hash | Wall / user / kernel s | Load |
| --- | ---: | ---: | --- |
| 0 / 0 | 194.175 | 3.88 / 3.78 / 0.03 | 7.68/14.51/14.90 → 8.03/14.47/14.88 |
| 0 / 16 | 184.001 | 3.68 / 3.59 / 0.02 | 8.03/14.47/14.88 → 8.19/14.39/14.86 |
| 16 / 0 | 171.556 | 3.43 / 3.35 / 0.03 | 8.19/14.39/14.86 → 8.09/14.27/14.81 |
| 16 / 16 | 191.325 | 3.82 / 3.73 / 0.02 | 8.09/14.27/14.81 → 8.09/14.27/14.81 |

Peak RSS is 2,132 KiB in all four rows. The larger image is faster under code0
and slower under code16 in these single batches; the observations do not isolate
a repeatable size penalty. The matching-policy diagonal improves only 1.47%.
Every row remains above 100 ms, before any index access. No hash or runtime row
was remeasured to improve the result.

| Hash executable construction | Wall / user / kernel s | Peak RSS KiB | Image bytes | Load |
| --- | ---: | ---: | ---: | --- |
| Inline0 | 0.21 / 0.20 / 0.00 | 36,532 | 110,337 | 7.68/14.51/14.90 → 7.68/14.51/14.90 |
| Inline16 | 0.23 / 0.22 / 0.00 | 32,440 | 106,241 | 7.68/14.51/14.90 → 7.68/14.51/14.90 |

## Oracle and recovery accounting

The first attempt used the current optimizer test with the frozen compiler.
Its raw bounds-trap assertion expects the newer path format and an explicit
Stderr grant; 26bf's unchanged contract is `R0003 at line 3 file 1:`.
That assertion failed. The next attempt used the unchanged frozen test and
passed admission, original/native comparisons, exact custom and cross-file
traps, and self-image checks, but stopped at the mutation smoke because the
archive omitted its seed corpus. Both are harness setup failures, retained here.

The final attempt includes the unchanged frozen `tools/generators` and
`tests/conformance/ir` corpus. The complete frozen optimizer oracle passes:
original rejection before optimization, original-IR interpretation versus checked
native execution, proof-reuse equivalence against full checking, exact trap
records, self-image behavior and 128 requested mutants with zero findings and
nonzero accepted/executed rewrites. No assertion or compiler source was changed
to manufacture compatibility. The inliner source is identical in frozen and
current trees (SHA-256
`961d59858c83afd8c5e8e2f9ec00d0f3a90cc91953ba8903528589f41a9da705`).
This bounded oracle does not claim that the compiler self-build was interpreted.

| Recovery / validation step | Wall / user / kernel s | Peak RSS KiB | Load |
| --- | ---: | ---: | --- |
| Current-script mismatch | 3.46 / 3.27 / 0.12 | 20,640 | 11.12/19.44/16.29 → 11.11/19.30/16.27 |
| Reconstruct inline0 for hashing | 19.08 / 17.44 / 0.15 | 503,308 | 6.18/15.51/15.23 → 5.67/14.79/15.00 |
| Reconstruct inline16 for hashing | 27.21 / 26.43 / 0.28 | 645,856 | 5.67/14.79/15.00 → 7.68/14.51/14.90 |
| Frozen oracle, missing mutation inputs | 5.27 / 5.00 / 0.16 | 31,060 | 8.09/14.27/14.81 → 8.64/14.28/14.82 |
| Reconstruct inline16 for final oracle | 36.44 / 24.38 / 0.23 | 671,196 | 49.55/23.96/18.04 → 45.11/25.12/18.61 |
| Complete frozen oracle | 15.74 / 5.28 / 0.21 | 34,468 | 45.11/25.12/18.61 → 51.63/27.93/19.68 |

The final oracle executable is 16,893,912 bytes, SHA-256
`c6b1e3ca1406f403ee50bba9c72f0034cde56e3e1c1644225f36c1ea859bf6b3`.
The frozen oracle script SHA-256 is
`53b16740dfc28ba7a46bcf35d6d96f186d992b34ad15e4295d3610c575a6ed67`.
Reconstruction and oracle rows are excluded from the six execution means.
All generated images, archives and test keys were removed. Only this report and
the reproducer are retained; no compiler/default policy change follows.
