# Performance

**Role: measurement method and compact recorded evidence.** This document does not promise a
performance level or select a runtime architecture. Raw output belongs in `target/` or CI artifacts;
only reproducible workload definitions and compact results belong here.

## Measurement protocol

Before a comparison:

1. state the hypothesis, equivalent semantics, workload, selection criteria, and reversal condition;
2. record commit, dirty state, machine/OS/architecture, CPU and memory, Rust version, build profile,
   engine options, and cold/warm cache state;
3. use `--locked` release builds for product measurements;
4. measure repeated runs and report sample count, median, and a tail percentile; separate compile,
   startup/time-to-first-result, and steady-state work;
5. record peak RSS, allocations/bytes where available, emitted code and binary size, and retained
   cache memory; and
6. keep the harness and command, store raw samples outside Git, and commit only the compact result.

Runtime comparisons must cover equivalent scalar, branch, call, product/enum/match, bytes/list,
ownership/cleanup, error/early-exit, and host-boundary workloads. Generated scale tests establish
correctness and expose complexity; they are not substitutes for representative application
benchmarks.

Noise-aware thresholds are required. A single developer-machine timing is orientation, not a hard
regression gate.

## Pre-reset session baseline

**Recorded historical baseline; not rerun after this documentation cutover.** The consolidation
session supplied the following pre-change measurements from `devbox`, Linux x86-64, with
`rustc 1.96.0`:

| Measurement | Result |
| --- | ---: |
| Cold locked release compile for the application/hello run | 78.010 s |
| Stripped release `target/release/lkjscript` size | 8,606,424 bytes |
| Warm hello process latency, median | 218.611 ms |
| Warm hello process latency, p95 | 222.068 ms |

The hello workload is:

```sh
cargo build --locked --release -p lkjscript-app
./target/release/lkjscript run src/examples/hello/main.lkjscript
```

A successful run prints `3628800`. For a comparable final measurement, use a fresh target directory
for the cold build, record the number of warm process samples and the same host metadata, and measure
the command above without recompilation. The original raw samples and sample count were not
committed, so this baseline is **pre-reset orientation only**. A representative post-reset runtime
baseline remains pending.

## Phase 2 build and binary comparison

**Measured build evidence, not a runtime-performance claim.** The hypothesis was that deleting the
unused broader-platform products would reduce the final `lkjscript` binary and cold build work while
leaving an unchanged warm no-op build in the same order of magnitude. The workload was exactly:

```sh
cargo build --locked --release -p lkjscript-app --bin lkjscript
```

[`meta/scripts/build-footprint.py`](../meta/scripts/build-footprint.py) retains the harness. Each
cold sample used a fresh empty `CARGO_TARGET_DIR`; Cargo registry/download caches remained warm. A
warm sample immediately repeated the unchanged command in that same target directory. Three paired
samples were collected before deletion at `698b40aa96682b6d875959306130bde7d82f513e` and three after
the final Rust/manifests/lock changes. The post-change working-tree diff before this results-only
documentation update had SHA-256
`3366b85194a65cbbd08e796f0e68da8a25c4746c7fe579c38e99cdf54438ead4` against that base.
Raw JSON, build logs, and sample values were retained under
`target/reset-audit/phase2-direct-deletion/` and are intentionally not committed.

Environment: `devbox`, Linux `7.0.0-27-generic` x86-64, AMD Ryzen 9 9955HX, 20 logical CPUs,
32 GiB RAM, `rustc 1.96.0 (ac68faa20 2026-05-25)`, and Cargo 1.96.0. The workspace release profile
used LTO, one codegen unit, and symbol stripping. For three samples, nearest-rank p95 is the maximum;
it is shown as orientation rather than a stable tail estimate.

| Measurement | Before median (p95) | After median (p95) | Median change |
| --- | ---: | ---: | ---: |
| Fresh-target locked release build | 79.606 s (80.327 s) | 76.638 s (77.139 s) | -2.967 s (-3.73%) |
| Immediate warm no-op build | 0.0265 s (0.0272 s) | 0.0238 s (0.0379 s) | -0.0026 s |
| Stripped `lkjscript` binary | 8,503,176 bytes | 8,082,368 bytes | -420,808 bytes (-4.95%) |

The warm samples are too short and noisy to support an improvement claim; the post-change p95 is
higher despite the lower median. The measured cold median and binary size support the deletion's
build-footprint hypothesis on this host only. The architectural deletion does not depend on a timing
win: restore a removed component only for a demonstrated product requirement, and re-investigate if
repeated equivalent builds show a material cold-build or binary-size regression.

No startup, execution throughput, generated-code, allocation, or peak-memory comparison was made in
this Phase 2 measurement. Existing runtime-selection evidence below is unchanged.

## Phase 5 semantic-workspace cutover

**Architectural/correctness evidence, not a performance result.** The compiler now consumes
`WorkspaceSnapshot` directly, and the former syntax-shaped stdio/session/text-publication path is
deleted rather than retained as a comparison path. Parser-counter tests cover direct compilation,
rename, replacement, and hole fill. Concise entity/body/type/reference/hole projection is iterative
and fallible; the retained ignored 20,000-level small-stack geometry also exercises body projection.
No latency, allocation, peak-RSS, or output-byte samples were recorded for this cutover, so no
projection or edit-performance claim is made. Future incremental measurements should use the
implemented operations described in [`status.md`](status.md), the flow in
[`architecture.md`](architecture.md), and the contract in
[`spec/workspace.md`](spec/workspace.md) rather than reviving the deleted wire service.

## Borrow-call validation and generic-preparation correction

**Measured product-path correction, not a universal performance promise.** The hypothesis was that
instruction-indexed full validator states plus per-instruction cleanup scans caused quadratic time
and memory, while generic preparation performed whole-program native specialization and identity
work with no cache, transfer, persistence, or runtime consumer. Equivalent semantics were the same
generated program: one string owner, repeated borrowed calls, VM execution, and result `42`.

The retained harness is [`meta/scripts/compiler-scale.py`](../meta/scripts/compiler-scale.py) plus
`borrow_call_scale_sample` in
[`source_scale.rs`](../crates/lkjscript-app/tests/source_scale.rs). Each sample used a locked release
build, warm Cargo dependencies/artifacts, and a fresh test process. The script polls `/proc` every
10 ms and sums resident pages for the Cargo process tree; this is approximate process-tree RSS, not
unique physical memory. Three samples per size report median and nearest-rank p95 (the maximum with
three samples). Commands were:

```sh
meta/scripts/compiler-scale.py --label before-block-validation \
  --sizes 1024,2048,4096 --samples 3
meta/scripts/compiler-scale.py --label final-complete \
  --sizes 1024,2048,4096,8192,16385 --samples 3 --exact-stress
```

Environment: `devbox`, Linux `7.0.0-27-generic` x86-64, AMD Ryzen 9 9955HX, 20 online logical CPUs,
32 GiB RAM, `rustc 1.96.0 (ac68faa20 2026-05-25)`, and Cargo 1.96.0. The before samples used product
code at `54f63fb4cc5164ba5d22579a027c69faa874e86b`; the dirty tree contained the replacement
`AGENTS.md` and measurement harness but no validator or compiler-path change. The original baseline
harness recorded only HEAD and a dirty Boolean, so its exact dirty tree relies on this operator
attestation. First-correction samples used the completed compiler tree with combined worktree
SHA-256 `d086cb27d141a0c0349b17b05d47a02a6cd0b640d11882d9b64c40476b4e121a`; the harness records tracked-diff and untracked-file hashes. Raw JSON remains under
`target/compiler-scale/` and is not committed.

| Calls | Bytecode validation before | Bytecode validation after | Generic preparation before | Package-validation timer after | Compile total before | Compile total after | Peak RSS before | Peak RSS after |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,024 | 86.181 ms (96.959) | 0.297 ms (0.303) | 3.617 ms (3.655) | <0.001 ms | 114.156 ms (128.895) | 24.706 ms (26.449) | 399.9 MiB (399.9) | 34.2 MiB (34.7) |
| 2,048 | 316.264 ms (318.288) | 0.541 ms (0.550) | 6.582 ms (6.722) | <0.001 ms | 390.046 ms (391.729) | 65.750 ms (71.090) | 1,491.5 MiB (1,492.7) | 43.2 MiB (43.4) |
| 4,096 | 1,197.054 ms (1,219.526) | 1.055 ms (1.098) | 12.762 ms (13.684) | <0.001 ms | 1,407.925 ms (1,435.959) | 194.084 ms (198.502) | 5,837.4 MiB (5,837.6) | 56.6 MiB (56.7) |

Across a fourfold input increase in the before matrix, validator time grows 13.89x and RSS 14.60x.
After the cutover, validator time grows 3.55x and RSS 1.65x; over the full 1,024-to-16,385 matrix,
validation grows 13.80x for 16x as many calls. At 4,096 calls, median validation is 1,135x faster and
approximate peak RSS is 103x smaller. Generic preparation was only 3.6-12.8 ms at these safe sizes,
so this baseline does **not** reproduce the old 261 s preparation observation under near-capacity
memory pressure. The producer/consumer audit nevertheless found its work wholly unconsumed;
deleting it and the rest of the in-process prepared identity reduced post-validator compile medians
by 10.2-15.9% at the three shared sizes.

First-correction phase medians showed the remaining shape; `SSA` is the sum of the separate
construction, verification, and normalization medians. These temporary paths are development
fixtures, so their 0.00014-0.00017 ms
package-validation timer is the no-op development branch, not a locked-package validation
measurement.

| Calls | HIR analysis | Memory planning | SSA | Bytecode lowering | Bytecode validation | VM execution | Test body | Peak RSS |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,024 | 7.818 ms | 9.315 ms | 1.981 ms | 1.158 ms | 0.297 ms | 2.543 ms | 27.656 ms | 34.2 MiB |
| 2,048 | 29.893 ms | 18.207 ms | 4.015 ms | 3.962 ms | 0.541 ms | 8.746 ms | 79.435 ms | 43.2 MiB |
| 4,096 | 116.291 ms | 37.269 ms | 7.563 ms | 15.128 ms | 1.055 ms | 28.106 ms | 226.315 ms | 56.6 MiB |
| 8,192 | 479.293 ms | 77.088 ms | 15.570 ms | 56.208 ms | 2.189 ms | 106.038 ms | 777.840 ms | 89.2 MiB |
| 16,385 | 1,876.925 ms | 172.229 ms | 31.305 ms | 233.240 ms | 4.100 ms | 573.811 ms | 2,967.116 ms | 148.5 MiB |

The exact named 16,385-call stress test was retained in that first-correction JSON and returned
`42`; its sample reported 2.457 s compile, 4.805 ms bytecode validation, 554.856 ms VM execution,
3.021 s body time, 3.062 s process wall time, and 153.7 MiB approximate peak RSS. The separate three-sample
parameterized row above uses the same generator and compile/run helper. The old recorded 333.24 s /
30.4 GiB result is historical and was not copied forward as a new sample.

The structural correction is block-entry dataflow with a range sweep and incrementally maintained
cleanup summary; no instruction, block, iteration, state, or program quota was added. Generic
preparation, both whole-program content encoders, the discarded native-specialization transform,
and their descriptor/identity contracts were deleted rather than bypassed. Reverse the validator
change if malformed-bytecode or fixed-point semantics fail, not for compatibility with old internal
state layout. Reintroduce a target-specific artifact identity only at a real cache, transfer,
persistence, or executable-artifact boundary that consumes the artifact.

## Remaining borrow-call scale correction

**Measured completion of the same product-path investigation, not a general compiler-throughput
promise.** The follow-up hypothesis was that borrow ownership repeatedly reconstructed suffix uses,
bytecode local classification and emission repeatedly scanned complete SSA functions, VM boundary
checks searched every cleanup range, and bytecode frame size incorrectly used SSA value count rather
than allocated physical slots. The workload and semantics are unchanged: one string owner, repeated
borrowed calls, validated bytecode, unrestricted VM execution, and result `42`. No validity quota,
general cache, parallel path, or alternate IR was added.

The retained commands were:

```sh
meta/scripts/compiler-scale.py --label before-remaining-scale-correction \
  --sizes 1024,2048,4096,8192,16385 --samples 3 --exact-stress
meta/scripts/compiler-scale.py --label final-remaining-scale-correction \
  --sizes 1024,2048,4096,8192,16385 --samples 5 --exact-stress
```

Both matrices used locked release builds, warm Cargo dependencies/artifacts, a fresh test process
per sample, and the same 10 ms approximate process-tree RSS polling described above. Environment:
`devbox`, Linux x86-64, AMD Ryzen 9 9955HX, 20 online logical CPUs, 32 GiB RAM,
`rustc 1.96.0 (ac68faa20 2026-05-25)`, and Cargo 1.96.0. Nearest-rank p95 is the maximum for both
three and five samples and remains orientation rather than a stable production tail estimate.

The before matrix used `0705fdf8d233e0be3f8eb0370c9d4d9454ba2038` with combined worktree
SHA-256 `965b4fbe611827283c21ea8ff4a4aed6c0ed0685985e647d7e9bf3a653502695`.
The final matrix used clean commit `81222f140de5bdfde71cccd01478010466a0c330` with combined
worktree SHA-256 `c23b4cecce65071a7b65351f30bb2811308af598a2695ea39f5a76c6850b2b5e`.
Raw JSON remains ignored at `target/compiler-scale/`; the final JSON SHA-256 is
`d60a246e29f773d1dca9418aa2bf37d02412122b15a114fe8452e5f53711cb57`.

| Calls | HIR analysis | Bytecode lowering | VM execution | Compile total | Test body | Peak RSS | Physical locals |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,024 | 0.578 ms (0.599) | 0.406 ms (0.411) | 0.450 ms (0.470) | 16.746 ms (17.373) | 17.558 ms (18.350) | 32.9 MiB (33.2) | 3 |
| 2,048 | 1.061 ms (1.139) | 0.798 ms (0.942) | 0.842 ms (0.896) | 32.626 ms (34.274) | 35.505 ms (38.490) | 40.9 MiB (43.3) | 3 |
| 4,096 | 1.870 ms (1.909) | 1.534 ms (1.559) | 1.642 ms (2.001) | 65.254 ms (68.105) | 68.966 ms (72.979) | 57.4 MiB (57.6) | 3 |
| 8,192 | 3.666 ms (4.242) | 3.135 ms (3.504) | 3.225 ms (3.594) | 135.611 ms (135.998) | 141.457 ms (145.593) | 86.4 MiB (86.5) | 3 |
| 16,385 | 7.303 ms (7.766) | 7.924 ms (8.034) | 6.612 ms (6.698) | 290.426 ms (299.238) | 303.119 ms (311.186) | 147.9 MiB (148.5) | 3 |

For 16.001x as many calls, final HIR analysis grows 12.63x, bytecode lowering 19.49x, VM
execution 14.70x, compile total 17.34x, and body time 17.26x. The corresponding before factors were
209.95x, 194.02x, 201.67x, 89.15x, and 98.45x. At 16,385 calls, median HIR analysis fell from
1,895.232 ms to 7.303 ms (259.5x), lowering from 227.915 ms to 7.924 ms (28.8x), VM execution from
583.732 ms to 6.612 ms (88.3x), compile total from 2,392.294 ms to 290.426 ms (8.24x), and body time
from 2,980.669 ms to 303.119 ms (9.83x). Approximate peak RSS is effectively unchanged at that
geometry because retained compiler state, not the now three-slot VM frame, owns the sampled peak.
Memory planning is now the largest measured compile phase: its final median grows from 9.104 ms at
1,024 calls to 165.349 ms at 16,385 calls.

A temporary three-sample A/B on the physical-slot checkout retained the per-frame cursor: binary
lookup at every boundary had a 16,385-call VM median of 9.573 ms (p95 10.034 ms), while the cursor
had a 6.102 ms median (p95 6.394 ms), 36.3% lower. The cursor advances one adjacent range in
constant time and uses binary lookup for multi-range jumps and backedges; it adds one `usize` to an
active frame. Raw labels are `after-physical-binary-only` and `after-physical-cursor-ab` under the
same ignored directory.

The exact named 16,385-call final stress returned `42` with 3 physical locals. It reported 6.993 ms
HIR analysis, 8.827 ms bytecode lowering, 4.711 ms bytecode validation, 300.367 ms compile,
7.960 ms VM execution, 312.321 ms body time, 360.757 ms process wall time, and 147.8 MiB approximate
peak RSS.

The correction is one iterative ownership-liveness plan with sparse direct-use indexes, one
per-function local metadata pass shared by coloring and emission, checked `max(slot) + 1` physical
frame sizing, binary unwind lookup, and one sequential per-frame cleanup-range cursor. Failed call
entry and post-step policy regressions additionally preserve reverse argument cleanup, unentered
plans, next-boundary state, and failure-atomic tail-frame reservation. Reverse these changes for a
semantic or cleanup failure, or if representative equivalent workloads show their retained state
cost outweighs the measured benefit; do not restore suffix reconstruction, whole-function local
rescans, per-step range scans, or SSA-sized physical frames for compatibility.

## Retained generated-scale evidence

**Historical rows retained for geometries not remeasured here.** These observations came from the
local AMD Ryzen 9 9955HX host and `rustc 1.96.0`. The 16,385-call row is superseded by the measured
post-correction section above; the other rows were not rerun by this change.

| Retained harness | Geometry | Recorded time |
| --- | --- | ---: |
| `crates/lkjscript-app/tests/source_scale.rs` / `four_thousand_ninety_seven_functions_compile_validate_and_execute_in_vm` | 4,097 functions through HIR, SSA, bytecode, and VM | 3.60 s |
| same file / `sixteen_thousand_three_hundred_eighty_five_calls_and_borrow_scopes_execute_in_vm` | 16,385 calls and inferred borrow scopes through VM | 333.24 s |
| `crates/lkjscript-compiler/src/hir/memory_plan/tests/bounds.rs` / `generated_hir_crosses_use_loan_obligation_destination_and_drop_path_boundaries` | 65,537 uses/loans and more than 32,768 obligations/destinations/drop paths | 9.91 s |
| same file / `structural_destinations_cross_the_former_limit_in_validated_bytecode` | 16,385 structural destinations through validated bytecode | 27.24 s |
| `crates/lkjscript-core/src/validation/tests/structural/mod.rs` / `structural_operation_references_cross_the_former_limit` | 65,537 validated operation references | 0.01 s |
| `crates/lkjscript-ir/src/tests/verification_region_product_scale.rs` / `region_product_metadata_crosses_the_former_sixteen_thousand_limit` | 16,385 verified SSA region products | 0.02 s |

The historical 16,385-call run peaked at approximately 30.4 GiB process-tree RSS. Its emitted
compiler metrics attributed 140 ms to memory planning, 59.44 s to bytecode validation, and 261.05 s
to preparation. It remains evidence of the removed pathology, not current performance.

The two application scale tests are ignored release stress tests. Reproduce individual rows with a
release build and the unique test-name filter, for example:

```sh
cargo test --locked --release -p lkjscript-app --test source_scale \
  four_thousand_ninety_seven_functions_compile_validate_and_execute_in_vm -- \
  --ignored --nocapture
cargo test --locked --release -p lkjscript-app --test source_scale \
  sixteen_thousand_three_hundred_eighty_five_calls_and_borrow_scopes_execute_in_vm -- \
  --ignored --nocapture
cargo test --locked --release -p lkjscript-compiler \
  generated_hir_crosses_use_loan_obligation_destination_and_drop_path_boundaries -- \
  --ignored --nocapture
cargo test --locked --release -p lkjscript-compiler \
  structural_destinations_cross_the_former_limit_in_validated_bytecode -- \
  --ignored --nocapture
cargo test --locked --release -p lkjscript-core \
  structural_operation_references_cross_the_former_limit -- --ignored --nocapture
cargo test --locked --release -p lkjscript-ir \
  region_product_metadata_crosses_the_former_sixteen_thousand_limit -- --nocapture
```

The exact 16,385-call geometry is still ignored so ordinary debug CI does not absorb a release
scale workload; its current measured release RSS is recorded above.

## Accepted runtime selection

**Accepted architecture; losing-path deletion implemented.** The product synchronously prepares
one eligible baseline-native reachable group before effects. It enters that group when preparation
succeeds and otherwise executes the unchanged program in the VM. Native entry is a commit point:
there is no VM retry afterward. Baseline native is a specialization inside one product path, not a
public engine contract. The VM has no JIT dependency or native transition branches. Automatic
thresholds, call observation, retries, invalidation, runtime sessions, forced helpers, optimizing
native lowering, proof metadata, and the optimizer implementation are deleted.

The selection hypothesis was that baseline native materially helps closed scalar groups, while the
VM remains the complete generic route and repeated automatic transitions cost more than they save.
`target/reset-audit/final/runtime-matrix.json` recorded three runs for each historical
workload/engine cell. The following compact values are median process wall milliseconds;
`unsupported` means the then-forced native engine rejected a required type or operation rather than
running equivalent semantics.

| Workload | VM | current auto | baseline-JIT | optimizing-JIT |
|---|---:|---:|---:|---:|
| hello | 235 | 236 | unsupported | unsupported |
| scalar | 474 | 1,700 | 245 | 244 |
| scalar redundancy (historical `optimizing`) | 379 | 395 | 244 | 251 |
| bench | 1,005 | 3,493 | unsupported | unsupported |
| mandel | 302 | 331 | unsupported | unsupported |

The same records show current `auto` performing 99,936 native invocations for `scalar`, 199,937 for
`bench`, and 600,129 VM fallbacks for `bench`, while each supported forced baseline run enters one
native invocation. The measured optimizing tier provides no representative advantage over baseline
in the two supported cells and shares the same unsupported cells. This supports one synchronous
baseline-group attempt and complete deletion of the losing tier. The comparable source workload was
renamed to `src/examples/scalar-redundancy`; its operation mix is
retained rather than presented as an engine-specific example. The CLI scalar fixture is likewise
`scalar-loop.lkjscript`.

The historical selection artifact did not record machine/compiler metadata, peak memory,
generated-code size, or target coverage, so it remains selection evidence rather than a complete
runtime baseline. Product metrics identify `execution_path=baseline-native|vm-fallback`, a nullable
fallback reason, whether native entry began, and preflight/lower/install/prepare/native/VM/total
durations. Threshold, automatic-transition, public engine, and tier fields are absent. Reverse the
choice if equivalent representative measurements show the group preflight or baseline maintenance
cost outweighs its scalar benefit.

### Post-cutover product-path baseline

**Measured orientation, not a cross-machine performance promise.** At
`e291d849971e4abe4b3135ee794754b5bd955ef0`, five warm-cache release process samples per workload
were run on `devbox`, Linux x86-64, 20 logical CPUs, 63,873,589,248 host bytes, with
`rustc 1.96.0 (ac68faa20 2026-05-25)`. Nearest-rank p95 is the maximum of five samples. Raw JSON is
retained outside Git at `target/reset-audit/final/selected-runtime-matrix.json`.

| Workload | Selected path | Process median (p95) | Compile median | Product total median |
|---|---|---:|---:|---:|
| hello | VM fallback | 228.36 ms (235.72 ms) | 111.73 ms | 0.14 ms |
| scalar | baseline native | 228.34 ms (231.20 ms) | 110.90 ms | 3.96 ms |
| scalar redundancy | baseline native | 226.65 ms (229.25 ms) | 111.25 ms | 2.19 ms |
| bench | VM fallback | 980.55 ms (1,012.84 ms) | 111.04 ms | 753.50 ms |
| mandel | VM fallback | 286.86 ms (293.87 ms) | 114.82 ms | 58.40 ms |

All 25 runs succeeded. The selected path entered native code exactly for the two eligible scalar
workloads and used the complete VM fallback for the other three. Relative to the historical
multi-transition `auto` medians above, selected-path scalar process latency fell from 1,700 ms to
228.34 ms and bench fell from 3,493 ms to 980.55 ms on this host. Those large differences support
the cutover; the smaller differences are within the limits of this sample geometry.

The final stripped `target/release/lkjscript` was 6,452,416 bytes, 2,154,008 bytes (25.03%) smaller
than the 8,606,424-byte pre-reset orientation. An incremental locked workspace release build after
the semantic-service deletion took 57.387 s; that is not comparable to the pre-reset fresh-target
build. Peak RSS, allocation counts, generated native-code size, other targets, and application-scale
steady-state throughput remain unmeasured.
