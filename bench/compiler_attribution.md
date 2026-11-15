# Fixed-source compiler attribution

The original compiler benchmark is repeated on the exact frozen source behind
the earlier 1.826× Rust opt0 / 6.053× Rust -O observation. With the retained native
rotation and canonical-result selectors, the new paired mean is **18.515 s**,
versus **18.620 s** for the baseline compiler. That 0.56% wall reduction does not
establish a whole-compiler speedup. The retained mean is **1.850× opt0 and
6.757× opt3** on this run. Its 4×, 3× and 2× milestones remain unmet.

The [SHA workload](hash.md) motivated and justifies the retained selectors because
exact producer hashing is a measured semantic-query cost. Its substantial gains
are a separate workload result. They must not be substituted for the compiler
ratios here.

## Reproduction and identity

Run one worker with the current producer:

```sh
bash bench/compiler_attribution.sh --compiler target/lkjscript \
  --results target/compiler-fixed-26bf.tsv
```

The default revision is `26bf2c3c021dd79810a81fa1d9054d59f3768b06`.
The harness archives that revision's compiler, source library, witness and seed
into one temporary tree. It builds the frozen compiler with the candidate native
emitter and each Rust optimization level, then runs each resulting compiler on
the same frozen tree with stores disabled. The candidate executable has new
machine selection but frozen compiler logic: its emitted compiler must therefore
remain byte-identical to the frozen baseline's output. This keeps the measured
source and computation fixed while changing the machine code that performs it.

| Input | Identity |
| --- | --- |
| Frozen compiler source tree | `3431671024d0a550b3130f2401bde067026d9bd3` |
| Frozen library tree | `e442fb3920db7eda2377f92fde0d92761f0562be` |
| Frozen witness tree | `bb2bbc6ef1932e22fe26a9e9d479ec4e7e14d28c` |
| Frozen seed and every measured compiler output | SHA-256 `8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132` |
| Producer that built the retained executable | SHA-256 `eaef4156f4649e9560b22cec59d404fe41bd387f0068e4e29d5fd1089f38dc09` |
| Frozen compiler emitted by that producer | SHA-256 `9b79feb0615b379eb9bf7ffb2ffed42ede7f534fa0471ff4acd6bf0d2d1116aa` |

Every timed run and both profile runs produced exactly **16,228,028 bytes** with
the common output digest above. The retained frozen executable itself is
16,196,887 bytes; its output contains the frozen emitter rather than the new one,
so those two artifact sizes need not agree.

Rust was `rustc 1.97.0 (2d8144b78 2026-07-07)`. Cargo uses one build job, one
codegen unit, the frozen manifest's overflow checks and default panic behavior.
Native IR inlining remains zero. The opt3/MIR-only-off route sets scoped
`RUSTC_BOOTSTRAP=1 -Zinline-mir=no`. The both-off route additionally passes
`-C llvm-args=--force-remove-attribute=alwaysinline` and
`-C llvm-args=--force-attribute=noinline`. No persistent toolchain configuration
changes, and Rust's prebuilt standard library is not rebuilt by those flags.

The harness checks for 3 GiB of free space and applies an explicit 3 GiB
address-space ceiling and 900-second watchdog per command. These are experimental
shared-host limits, not language resource policy. The largest observed witness
build RSS was 1,206,480 KiB. Each Cargo build is sequential and its owned build
directory is deleted before the next level. Exit cleanup removes the frozen tree,
generated images and samples; result tables and reports remain. If a supervisor
disappears, `--cleanup PATH` verifies the recorded repository and dead owner,
refuses paths still referenced by a command, preserves the current checkpoint,
time and output, then removes only that owned tree.

## Execution measurements

Wall, user and kernel times are seconds. Loads are the 1/5/15-minute averages
immediately before → after each timed command. The native routes are paired
twice; each Rust level has one observational run.

| Route | Wall / user / kernel s | Peak RSS KiB | Load |
| --- | ---: | ---: | --- |
| Native baseline 1 | 18.70 / 18.20 / 0.14 | 524,160 | 5.74/5.15/5.24 → 5.55/5.14/5.23 |
| Native retained 1 | 18.35 / 17.93 / 0.15 | 524,884 | 5.55/5.14/5.23 → 5.27/5.10/5.22 |
| Rust opt0 | 10.01 / 9.79 / 0.12 | 371,256 | 5.27/5.10/5.22 → 4.85/5.02/5.19 |
| Rust opt1 | 3.48 / 3.35 / 0.08 | 363,072 | 4.85/5.02/5.19 → 4.62/4.97/5.17 |
| Rust opt2 | 2.72 / 2.61 / 0.07 | 365,376 | 4.62/4.97/5.17 → 4.57/4.95/5.16 |
| Rust opt3 | 2.74 / 2.63 / 0.07 | 363,360 | 4.57/4.95/5.16 → 4.57/4.95/5.16 |
| Rust opt3, MIR inlining off | 2.70 / 2.59 / 0.08 | 364,348 | 4.57/4.95/5.16 → 4.44/4.92/5.15 |
| Rust opt3, MIR + LLVM inlining off | 5.77 / 5.57 / 0.12 | 364,992 | 4.44/4.92/5.15 → 4.49/4.92/5.15 |
| Native baseline 2 | 18.54 / 18.17 / 0.11 | 539,900 | 4.49/4.92/5.15 → 5.13/5.04/5.19 |
| Native retained 2 | 18.68 / 18.29 / 0.13 | 528,372 | 5.13/5.04/5.19 → 5.41/5.11/5.20 |

The native baseline mean is 18.620 s wall / 18.310 s CPU. The retained mean is
18.515 s wall / 18.250 s CPU: reductions of 0.56% and 0.33%. These means use the
individual load windows above; they are not additional timed runs. The historical
1.826× / 6.053× observation remains historical. The exact source is preserved,
but this host/load and toolchain observation yields the ratios stated here.

Disabling MIR inlining alone changes opt3 from 2.74 to 2.70 s, within ordinary
single-run variation. Disabling both MIR and LLVM inlining changes it to 5.77 s.
This is evidence that inlining and transformations it enables matter, but it
does not allocate 3.03 s solely to call overhead. The intervention also prevents
scalar helpers from exposing constants and aggregate lifetimes to later passes.
The retained native compiler is still 3.209× that both-off witness.

Opt0→opt1 and opt1→opt2 differ much more than opt2→opt3 on this workload. These
levels change many passes together. Range-check removal, scalar replacement of
aggregates, loop-load hoisting and instruction scheduling have not been isolated
separately, so no percentage of the remaining gap is assigned to them. Native
and Rust also use different collection/runtime implementations, including the
native AVL arena versus Rust's BTreeMap. Those costs are part of this ordinary
compiler workload rather than evidence for an instruction-selection-only floor.

## Construction measurements

These rows build the executable used by a later measurement. They are not the
self-build latency being compared above. A baseline executable is already supplied
by the frozen seed; profile construction is shown explicitly.

| Executable constructed | Wall / user / kernel s | Peak RSS KiB | Image bytes | Load |
| --- | ---: | ---: | ---: | --- |
| Native retained | 18.97 / 18.49 / 0.15 | 516,252 | 16,196,887 | 2.76/3.82/4.92 → 3.74/3.98/4.95 |
| Baseline profile | 19.99 / 19.51 / 0.19 | 577,428 | 51,853,186 | 3.74/3.98/4.95 → 4.60/4.17/5.00 |
| Retained profile | 19.70 / 19.19 / 0.20 | 571,032 | 51,579,574 | 4.60/4.17/5.00 → 5.14/4.34/5.03 |
| Rust opt0 | 7.19 / 6.69 / 0.41 | 932,572 | 15,980,912 | 5.14/4.34/5.03 → 5.21/4.37/5.04 |
| Rust opt1 | 36.56 / 35.65 / 0.33 | 1,206,480 | 7,379,160 | 5.21/4.37/5.04 → 5.71/4.57/5.08 |
| Rust opt2 | 43.06 / 41.95 / 0.49 | 1,086,916 | 5,709,696 | 5.71/4.57/5.08 → 6.52/5.01/5.21 |
| Rust opt3 | 42.68 / 41.68 / 0.23 | 1,125,640 | 6,041,744 | 6.52/5.01/5.21 → 5.77/5.03/5.21 |
| Rust opt3, MIR inlining off | 43.82 / 42.40 / 0.37 | 1,058,488 | 6,050,680 | 5.77/5.03/5.21 → 5.58/5.05/5.21 |
| Rust opt3, MIR + LLVM inlining off | 23.16 / 22.38 / 0.40 | 988,948 | 7,753,024 | 5.58/5.05/5.21 → 5.74/5.15/5.24 |

## Native profiles

Profiles request a 1,000 μs sampling period and record real frame chains.
They add instrumentation and are interpreted as CPU attribution, not substituted
for the unprofiled wall measurements.

| Route | Wall / user / kernel s | Peak RSS KiB | Load |
| --- | ---: | ---: | --- |
| Baseline profile | 18.01 / 17.63 / 0.09 | 633,188 | 5.41/5.11/5.20 → 4.88/5.00/5.17 |
| Retained profile | 17.89 / 17.48 / 0.11 | 611,016 | 4.88/5.00/5.17 → 4.85/4.98/5.16 |

The baseline collected 17,529 samples and the retained executable 17,462.
Emitted-kind counts sum to those totals. Percentages below are recomputed from
the counts; the compact profile command truncates its displayed percentages.

| Emitted kind | Baseline samples / % | Retained samples / % |
| --- | ---: | ---: |
| Inline copy | 2,788 / 15.91 | 2,788 / 15.97 |
| Checks | 225 / 1.28 | 251 / 1.44 |
| Call instruction | 149 / 0.85 | 149 / 0.85 |
| Frame load/store | 842 / 4.80 | 922 / 5.28 |
| Other memory | 1,887 / 10.77 | 1,933 / 11.07 |
| Arithmetic / comparison / conversion | 2,113 / 12.05 | 1,966 / 11.26 |
| Branch | 446 / 2.54 | 497 / 2.85 |
| Allocator | 1,716 / 9.79 | 1,677 / 9.60 |
| Runtime | 443 / 2.53 | 399 / 2.28 |
| Register-to-register move | 2,442 / 13.93 | 1,967 / 11.26 |
| Address formation | 1,052 / 6.00 | 1,321 / 7.56 |
| Immediate constant | 953 / 5.44 | 1,034 / 5.92 |
| Argument setup / result capture | 395 / 2.25 | 413 / 2.37 |
| Prologue / epilogue | 1,588 / 9.06 | 1,630 / 9.33 |
| Return | 490 / 2.80 | 515 / 2.95 |
| Unknown | 0 / 0.00 | 0 / 0.00 |

The leading self functions were byte-slice ordering (702 / 577 samples),
byte append (545 / 520), release (432 / 406), allocation (398 / 401),
allocation-size classification (375 / 378), UTF-8 validation (277 / 230), and
the checker body (222 / 211). The checker body's inclusive share was
41.1% / 42.6%; driver compile was 95.2% / 95.1%. A function's inclusive count
includes its descendants and must not be added to their self counts.

Unlike the SHA profile, the compiler is distributed across copies, allocation,
memory traffic and call-frame work. The observed 1.28% / 1.44% checks category
does not by itself explain a 6.757× gap. This is not a formal bound on what
range facts or inlining could enable: transformations interact, and sampled
machine categories do not identify all higher-level causes.

## Oracle, retained scope and remaining work

Both selectors leave checked IR unchanged. The admitted original is still
interpreted independently in the scalar/rotation/crypto cases; generated code
must agree in values, output and exact trap identity. The fixed compiler run
adds the stronger whole-output byte comparison above, but it is not a claim
that a full compiler self-build was interpreted. [hash.md](hash.md) records the
final focused 23-case / 46-route oracle and the separately measured rejected
trials. Only exact rotation selection and canonical scalar results survive.

The hash workload justified moving this work ahead of the rest of the ladder:
exact producer hashing was the query's dominant measured cost. Its current
fixed-byte mean remains above 100 ms. Whole-compiler work has not crossed 4× -O,
and the table does not establish a 2× backend floor. The next general optimization
needs a measured transformation-specific trial on this fixed source, not another
small-kernel ratio. Full cold gate, fixed point and CI belong to the parent
integration and are not asserted by these focused measurements.
