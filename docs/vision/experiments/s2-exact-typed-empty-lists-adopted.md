# Experiment Registry: S2 Exact Typed Empty Lists: Adopted

[Authority](../experiments.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## S2 Exact Typed Empty Lists: Adopted

- Baseline: `8d221ac` (dedicated Unit and exact `if`)
- Candidate: `45e2d085e13638b92dd1c08e12b2095781f0d248`
- Environment/build: the S1 Linux x86-64 host and locked release procedure;
  each binary ran algorithm-equivalent source from its own commit
- Correctness: hello, Mandelbrot, and Leibniz output bytes were identical;
  64 tests, exact source closure, strict Clippy, release smokes, typed-list
  positive/negative CLI boundaries, lkjedit, and HTTP passed
- Timing: four warmups and 31 deterministic randomized samples per variant;
  entries are median / median absolute deviation / p95 process milliseconds

| Workload | Baseline | Typed empty list | Candidate / baseline |
| --- | ---: | ---: | ---: |
| hello compile + run | 0.447 / 0.022 / 0.497 | 0.448 / 0.019 / 0.507 | 1.002 |
| Mandelbrot compile + run | 5.171 / 0.118 / 5.990 | 5.061 / 0.206 / 5.790 | 0.979 |
| Leibniz 200,000 compile + run | 74.766 / 1.680 / 78.994 | 70.040 / 0.755 / 74.601 | 0.937 |
| Mandelbrot compile + disassemble | 0.672 / 0.026 / 0.967 | 0.674 / 0.013 / 1.169 | 1.003 |

Release binary size increased from 646,368 to 652,080 bytes (1.009x). No
performance threshold was recorded before this required semantic slice. The
runtime medians are diagnostic—especially the unexplained Leibniz movement,
which is not attributed to typed lists. The slice was adopted for exact type
semantics and removal of nil/list ambiguity. Temporary artifacts were removed
and the candidate release tree was rebuilt.
## S3 Explicit Option And No Nil: Adopted

- Baseline: `45e2d085e13638b92dd1c08e12b2095781f0d248`
- Candidate: `0f17d9749966101473746530b26415127371d5a1`
- Environment/build: the S1 Linux x86-64 host and locked release procedure;
  each binary ran algorithm-equivalent source from its own commit
- Correctness: hello, Mandelbrot, and Leibniz output bytes were identical;
  70 tests, exact source closure, strict Clippy, release smokes, Option/argument
  CLI boundaries, removed Nil diagnostics/opcodes, malformed-VM boundaries,
  lkjedit, HTTP, and Docker verification passed
- Timing: four warmups and 31 deterministic randomized samples per variant;
  entries are median / median absolute deviation / p95 process milliseconds

| Workload | Baseline | Option/no-nil | Candidate / baseline |
| --- | ---: | ---: | ---: |
| hello compile + run | 0.408 / 0.014 / 1.141 | 0.405 / 0.015 / 0.516 | 0.994 |
| Mandelbrot compile + run | 5.052 / 0.224 / 5.905 | 5.188 / 0.258 / 5.948 | 1.027 |
| Leibniz 200,000 compile + run | 70.093 / 1.057 / 80.102 | 71.174 / 0.824 / 77.158 | 1.015 |
| Mandelbrot compile + disassemble | 0.668 / 0.021 / 0.996 | 0.678 / 0.015 / 0.803 | 1.014 |

Release binary size increased from 652,080 to 660,112 bytes (1.012x). No
performance threshold was recorded before this required safety/semantic slice;
the figures are diagnostic and show no median regression above 3% in this
sample. The slice was adopted to eliminate type-confused absence and semantic
fallback values. Temporary artifacts were removed and the candidate release
tree was rebuilt.
## S4 Explicit Equality Families: Adopted

- Baseline: `13fbd1bd9a44d4d65864fad3f6a571df9901de9d`
- Candidate: `ba1d2219fcff34f53f8a7f316c2ece39ba6357bd`
- Semantic result: universal `eq`/`ne` and opcode 21 were removed; exact
  `equal-value`, `same-object`, bounded `list-equal`, and `f64-bits-equal`
  operations now resolve through HIR and execute in the VM. Str and Symbol use
  distinct constant/runtime categories.
- Correctness: 80 tests, exact 127-source/11-root closure, strict Clippy, hello,
  direct Mandelbrot, Brainfuck smoke, lkjedit, and HTTP passed. Runtime outputs
  were byte-identical across variants for hello, direct Mandelbrot,
  Leibniz-200000, and Brainfuck hello. Disassembly intentionally changed to the
  canonical equality vocabulary. A separate clean candidate full Brainfuck
  Mandelbrot correctness run completed in 1,265.803147 seconds and matched the
  independent 6,240-byte oracle with SHA-256
  `83a0aac65090b3b5e85c22337afac39d8ac17bfd88675f044b33bd55ca0c351b`.
- Docker: the first verification attempt exposed pre-existing broken links
  because the image omitted `meta/benchmarks`; commit
  `41d3d31346dc498cf441cdaa736187ff5c001c81` copied those committed link targets,
  and the exact Docker verification command then passed with `result=ok`.
- Environment: Linux 7.0.0-27-generic x86-64, AMD Ryzen 9 9955HX
  16-core/32-thread, 32 GiB RAM, Rust/Cargo 1.96.0, Python 3.12.3.
- Build: locked release workspace in one target tree; baseline and candidate
  binaries and exact source snapshots were retained only under `target/` for
  measurement.
- Timing: four warmups per variant/workload followed by 31 samples per variant
  and workload in deterministic randomized order (`0xE0A117`); monotonic
  end-to-end process wall time with stdout discarded. Entries are median / MAD
  / p95 milliseconds.

| Workload | Universal equality | Explicit equality | Candidate / baseline |
| --- | ---: | ---: | ---: |
| hello compile + run | 0.510 / 0.036 / 1.736 | 0.525 / 0.041 / 1.406 | 1.029 |
| Mandelbrot compile + run | 5.393 / 0.079 / 6.081 | 5.408 / 0.111 / 6.549 | 1.003 |
| Leibniz-200000 compile + run | 74.942 / 1.189 / 78.240 | 73.314 / 1.121 / 78.982 | 0.978 |
| Mandelbrot compile + disassemble | 0.824 / 0.055 / 2.065 | 0.801 / 0.032 / 1.287 | 0.972 |
| Brainfuck hello compile + run | 1.842 / 0.046 / 2.869 | 1.839 / 0.057 / 3.116 | 0.998 |

Release binary size increased from 660,112 to 663,888 bytes (1.006x). No
performance threshold was chosen before this required semantic migration. The
sample is diagnostic and shows no candidate median movement above 3%; it does
not support a general performance claim. The slice was adopted for exact static
semantics, removal of closure identity, bounded structural comparison, and
backend-ready operation identities. No samples were removed. Temporary source
snapshots, copied binaries, and detailed JSON were deleted after this compact
record was committed.
## S5 Immutable Nominal Products: Adopted

- Baseline: `e75dae80410c07df7f3ab42b237101e4eb65876f`
- Candidate: `6452104a098f8aa127a13de44fce3c34615e3f78`
- Semantic result: zero-to-15-field nominal product declarations, exact ordered
  construction, typed named access, immutable field replacement, deterministic
  ProductIds/field descriptors, bytecode/disassembly metadata, precise tracing,
  and malformed-VM checks are Current. Product declarations add no runtime
  global or initializer, and all equality families reject Product values.
- Correctness: 88 tests, exact 127-source/11-root closure, strict Clippy, product
  source-to-VM execution/disassembly, hello, direct Mandelbrot, Brainfuck smoke,
  lkjedit, HTTP, release build, and Docker verification passed. Baseline and
  candidate output bytes were identical for hello, direct Mandelbrot,
  Leibniz-200000, and folded Brainfuck hello. Candidate Mandelbrot remained
  1,176 bytes with SHA-256
  `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907`.
  A separate full folded Brainfuck Mandelbrot correctness run completed in
  1,381.302003 seconds and was byte-equal to the independent 6,240-byte oracle
  with SHA-256
  `83a0aac65090b3b5e85c22337afac39d8ac17bfd88675f044b33bd55ca0c351b`;
  its bounded 10-second diagnostic timed out as expected. This one correctness
  run is not a replacement performance sample. Mandelbrot disassembly
  intentionally gained empty product metadata tables.
- Environment: Linux 7.0.0-27-generic x86-64, AMD Ryzen 9 9955HX
  16-core/32-thread CPU with 20 logical CPUs available to the Python process,
  32 GiB RAM, Rust/Cargo 1.96.0, Python 3.12.3.
- Build: locked release builds from one Cargo target tree, run sequentially.
  Baseline binary SHA-256 was
  `2e34fa70e7ff579b4f4750762f24b457e2a646c08ce0eaa8165e97d9afc66987`;
  candidate SHA-256 was
  `d2c0a10b3d9febe276efcf5643f366b4d45761857db21df4d74ad957ac33c043`.
- Timing: four warmups per variant/workload followed by 31 samples per variant
  and workload in deterministic randomized order (seed
  `0x50524F44554354`, decimal `22608498539184980`); monotonic end-to-end process
  wall time with stdout
  discarded. Entries are median / MAD / p95 milliseconds.

| Workload | Pre-product baseline | Nominal products | Candidate / baseline |
| --- | ---: | ---: | ---: |
| hello compile + run | 0.496 / 0.064 / 1.344 | 0.501 / 0.050 / 1.386 | 1.011 |
| Mandelbrot compile + run | 5.547 / 0.162 / 6.932 | 5.252 / 0.086 / 6.460 | 0.947 |
| Leibniz-200000 compile + run | 72.659 / 1.424 / 86.283 | 72.201 / 1.405 / 75.228 | 0.994 |
| Mandelbrot compile + disassemble | 0.818 / 0.093 / 2.272 | 0.840 / 0.102 / 1.801 | 1.028 |
| Brainfuck hello compile + run | 1.799 / 0.160 / 3.056 | 1.763 / 0.079 / 2.705 | 0.980 |

Release binary size increased from 663,888 to 702,160 bytes (1.058x). No
performance threshold was selected before this semantic prerequisite. The
runtime sample is diagnostic: sub-millisecond process-startup noise is large,
Mandelbrot moved faster rather than slower, and no runtime median regression
exceeded 3%. The 5.8% binary-size increase is retained visibly for later native
backend/resource scorecards; it does not override the required product-state
semantics. No samples were discarded. Temporary worktree, copied binaries,
harness, and detailed JSON are deleted after this compact record is committed.
