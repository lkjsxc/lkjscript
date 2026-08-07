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
committed, so this baseline is **pre-reset orientation only**. A final post-reset comparable
measurement is pending.

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

**Accepted architecture; app cutover implemented, internal deletion pending.** The product
synchronously prepares one scalar baseline-native reachable group before effects. It enters that
group when preparation succeeds and otherwise executes the unchanged program in the VM. Native
entry is a commit point: there is no VM retry afterward. Baseline native is a specialization inside
one product path, not a public engine contract. The CLI no longer exposes forced tiers, thresholds,
or auto controls. Forced/auto APIs and optimizing machinery remain temporarily only for existing
differential tests and deletion work.

The selection hypothesis was that baseline native materially helps closed scalar groups, while the
VM remains the complete generic route and repeated automatic transitions cost more than they save.
`target/reset-audit/final/runtime-matrix.json` recorded three runs for each workload/engine cell. The
following compact values are median process wall milliseconds; `unsupported` means the forced native
engine rejected a required type or operation rather than running equivalent semantics.

| Workload | VM | current auto | baseline-JIT | optimizing-JIT |
|---|---:|---:|---:|---:|
| hello | 235 | 236 | unsupported | unsupported |
| scalar | 474 | 1,700 | 245 | 244 |
| optimizing | 379 | 395 | 244 | 251 |
| bench | 1,005 | 3,493 | unsupported | unsupported |
| mandel | 302 | 331 | unsupported | unsupported |

The same records show current `auto` performing 99,936 native invocations for `scalar`, 199,937 for
`bench`, and 600,129 VM fallbacks for `bench`, while each supported forced baseline run enters one
native invocation. Optimizing-JIT provides no representative advantage over baseline in the two
supported cells and shares the same unsupported cells. This supports one synchronous baseline-group
attempt rather than per-entry auto tiering or optimizer retention.

The artifact does not record machine/compiler metadata, peak memory, generated-code size, release
binary size, or target coverage, so it is selection evidence rather than the final post-cutover
baseline. Product metrics now identify `execution_path=baseline-native|vm-fallback`, a nullable
fallback reason, whether native entry began, and preflight/lower/install/prepare/native/VM/total
durations. Threshold, auto, public engine, and tier fields are absent. Retain the harness, record the
missing dimensions with environment metadata after internal losing-path deletion, and reverse the
choice if equivalent representative measurements show the group preflight cost or baseline
maintenance cost outweighs its scalar benefit.

### Cutover path smoke evidence

**Implemented-path verification, not a performance baseline.** After the cutover, one warm-cache
locked release process was run per workload on `devbox`, Linux x86-64, 20 logical CPUs,
63,873,589,248 host bytes, with `rustc 1.96.0 (ac68faa20 2026-05-25)`. One sample has no median or
tail value and establishes path selection and timing-field integrity only.

| Workload | Path | Native entered | Process wall | Product total | Native | VM |
|---|---|---:|---:|---:|---:|---:|
| scalar | baseline-native | yes | 240.831 ms | 0.457 ms | 0.222 ms | 0 ms |
| hello | VM fallback | no | 239.944 ms | 0.108 ms | 0 ms | 0.103 ms |
| bench | VM fallback | no | 986.250 ms | 747.499 ms | 0 ms | 747.494 ms |
| mandel | VM fallback | no | 299.363 ms | 58.134 ms | 0 ms | 58.133 ms |

All four returned successfully. Hello emitted 7 bytes, bench 18 bytes, and mandel 1,176 bytes once;
the scalar fixture emitted none. The three fallback records reported `unsupported-shape`, while the
scalar record had a null fallback reason. The final multi-sample post-deletion baseline remains
pending.
