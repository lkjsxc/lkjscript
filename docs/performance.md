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

## Retained generated-scale evidence

**Recorded before this documentation cutover; harnesses remain committed.** The measurements below
came from the local 20-logical-CPU AMD Ryzen 9 9955HX host with 32 GiB RAM and `rustc 1.96.0`. Times
are recorded test-body observations from the prior reset work, not results rerun by this docs-only
change.

| Retained harness | Geometry | Recorded time |
| --- | --- | ---: |
| `crates/lkjscript-app/tests/source_scale.rs` / `four_thousand_ninety_seven_functions_compile_validate_and_execute_in_vm` | 4,097 functions through HIR, SSA, bytecode, and VM | 3.60 s |
| same file / `sixteen_thousand_three_hundred_eighty_five_calls_and_borrow_scopes_execute_in_vm` | 16,385 calls and inferred borrow scopes through VM | 333.24 s |
| `crates/lkjscript-compiler/src/hir/memory_plan/tests/bounds.rs` / `generated_hir_crosses_use_loan_obligation_destination_and_drop_path_boundaries` | 65,537 uses/loans and more than 32,768 obligations/destinations/drop paths | 9.91 s |
| same file / `structural_destinations_cross_the_former_limit_in_validated_bytecode` | 16,385 structural destinations through validated bytecode | 27.24 s |
| `crates/lkjscript-core/src/validation/tests/structural/mod.rs` / `structural_operation_references_cross_the_former_limit` | 65,537 validated operation references | 0.01 s |
| `crates/lkjscript-ir/src/tests/verification_region_product_scale.rs` / `region_product_metadata_crosses_the_former_sixteen_thousand_limit` | 16,385 verified SSA region products | 0.02 s |

The 16,385-call run peaked at approximately 30.4 GiB process-tree RSS. Its emitted compiler metrics
attributed 140 ms to memory planning, 59.44 s to bytecode validation, and 261.05 s to preparation.
Those downstream phases and memory use are active scale problems, not reasons to restore a validity
quota.

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

Do not run the 16,385-call geometry on a constrained host without expecting near-machine-capacity
memory pressure.

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
