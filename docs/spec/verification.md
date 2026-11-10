# Development checks

Run `bash tools/check.sh` before pushing. It checks Rust formatting and whitespace,
builds every workspace test target once, then executes each routine test once.
It reports build time, test time, counts, sampled peak RSS and the ten slowest tests.
It also prints the profile, filters, thread count and stack reservation. A nonzero
build or harness exit, or an empty test selection, fails the command. Cargo owns
incremental artifact reuse.

During development use `bash tools/check.sh focused FILTER`. It uses the same
workspace and test profile as the routine suite, so selected tests reuse the same
build products. Set `RUST_TEST_THREADS` to tune concurrency. The routine and focused
default is sixteen; the slow tier defaults to one because complete-compiler tests
can each consume many gigabytes.
Test threads reserve stacks from physical memory; `RUST_MIN_STACK` overrides the
reservation when the caller needs a specific test-host configuration.

Bootstrap fixed points and scale controls carry a descriptive `#[ignore]` reason.
Run relevant controls explicitly with `bash tools/check.sh slow FILTER`. Both
focused and slow commands require a test-name filter first; multiple filters select
their union. Put harness options after the filter, for example
`bash tools/check.sh slow whole_maintained_raw_source_fixed_point --nocapture`.
Ignored measurement/export tools may require the environment named in their test.
`bash tools/selfhost/verify.sh` runs the maintained raw-source compiler fixed point;
arguments replace that selection with explicit ignored-test filters. Use
`LKJSCRIPT_CHECK_PROFILE=release` when comparing optimized compiler or runtime
performance. This builds a separate Cargo profile.

The maintained compiler has three independently selectable slow controls:

| Test filter | Coverage |
| --- | --- |
| `whole_maintained_raw_source_fixed_point` | Exact raw source, `C2 = C3`, `Ahost = A1 = A2 = A3`, strict artifact provenance, independent scalar 42→43 and compiler diagnostic 12→77 mutations |
| `selfhost_source_artifact_maintained_native_lifecycle` | Whole-compiler same-process allocation, retained output, cancellation, failure recovery and cleanup for both Rust-built C1 and source-built C2 |
| `selfhost_source_artifact_maintained_semantic_corpus` | Active scalar/compound/custody, binary-filter, bulk-byte, callable, nominal, 100,001-expression sequence and layered rejection cases against stabilized C2 |

Select them together to share one immutable maintained source preparation:

```sh
bash tools/check.sh slow whole_maintained_raw_source_fixed_point \
  selfhost_source_artifact_maintained_native_lifecycle \
  selfhost_source_artifact_maintained_semantic_corpus
```

The public CLI publication-to-detached-compiler control is also explicit:
`bash tools/check.sh slow native_elf_public_build_runs_lkjscript_compiler_as_detached_binary_filter`.
Run it when compiler publication, CLI build orchestration or the detached filter changes.

The deep unused task-signature DAG control uses the slow filter
`selfhost_source_effect_types_admit_deep_unused_input_and_result_edges`.
Depth 16 runs through all four reader engines; digest-ordered depths 256, 512 and
1,024 run through both native producers.

Other complete-compiler and capacity controls are explicit slow selections:

| Test filter | Coverage |
| --- | --- |
| `selfhost_normalized_byte_inventory_exports_current_compiler` | Complete normalized compiler input admitted by both evaluators |
| `selfhost_byte_backend_reproduces_its_complete_native_closure` | Normalized backend fixed point, artifact corpus and complete compiler-source mutation |
| `selfhost_maintained_proposals_match_authoritative_function_bodies` | Replay of every maintained compiler declaration and canonical unit comparison |
| `selfhost_compiler_is_deterministic_and_source_mutation_changes_emitted_behavior` | Complete proposal replay and observable compiler diagnostic mutation |
| `selfhost_compiler_accepts_more_than_historical_node_limit_without_default_allowance` | 32,770-node compiler input across evaluator and native paths |
| `selfhost_source_map_scale_crosses_historical_row_limits` | Independent 100,001-row and 65,537-row logical maps |
| `selfhost_source_maps_match_current_compiler_state` | Complete maintained owner, dependency and retirement maps |
| `selfhost_source_state_binds_complete_maintained_compiler` | Complete logical source-state commitment from all maintained map rows |
| `selfhost_source_objects_bind_every_record_to_logical_directory_identity` | Every maintained compiler record bound to its directory identity |

The routine suite keeps compact normalized admission, compiler input mutations,
actual native-closure mutation, map page transitions, record identity faults and
reader semantic cases. All twelve temporary reader-probe modules share one graph and normalized program,
authored in one public transaction over the unchanged maintained compiler.
Reader preparation uses an immutable native compiler seed.
Its image is keyed by the exact normalized closure; self-produced reader images
are keyed by both compiler and reader input digests. Per-key initialization runs
outside shared map locks. Every reader invocation still creates fresh execution
state for VM, reference, Rust-native and self-produced-native comparisons.
The explicit compiler differential controls keep their VM/reference/native runs.

Run `bash tools/check.sh slow selfhost_source_reader_cache_observes_three_distinct_cold_keys`
alone in a fresh process to compare three distinct cold reader preparations with
warm reuse. It checks output identities and unchanged repository/executable bytes.

Preparation checks the original repository before and after opening it, copies
its exact bytes, proves the generation equations and drops the isolated disk
copy. Each control checks the original file inventory again and launches fresh
native invocations. Once C2 and C3 are proved byte-identical, per-input behavior
runs once for that compiler identity. The native lifecycle control retains both
producer paths. The default selfhost command selects the fixed point and its
compact mutation controls; corpus and lifecycle changes select the other tests.

Keep a compact independent semantic comparison and source mutations that change
emitted behavior beside compiler fixed points. Share immutable fixture preparation
inside a harness. A fixed point checks stabilization; semantic tests check results.
Process time and memory envelopes belong to the test runner. RSS observations sample
Linux `/proc` every 500 ms: the maximum observed process `VmHWM` and the largest
simultaneous sum of harness/descendant `VmRSS`. Short-lived processes can escape
sampling, and the tree sum can count shared pages more than once. These are sampled
observations, not exact allocation totals.

Report what ran, what failed and what was skipped. Git records source changes.
No additional receipts, source seals, authenticated build inventories, duplicated
changed/full passes or implementation-commit pairing are required.
