# Performance evidence

No performance leadership claim is made. These are bootstrap baselines whose purpose is to expose
costs before optimization.

## Retained reset baseline

### Environment

- date: 2026-08-14;
- code state: `5785f650cbe11c38350a8005ba17f4bd40bb84b6`;
- host: `devbox`, AMD Ryzen 9 9955HX, 20 logical CPUs visible, 32 GiB memory;
- OS: Linux 7.0.0-29-generic x86-64, glibc 2.39;
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`;
- mode: release for runtime measurements;
- workload: one 10-node source-free workspace whose entry computes `40 + 2`;
- oracle: typed result must equal `i64(42)` and artifact round-trip must be byte-identical.

### Build and test baseline

A fresh target directory avoided deleting or reusing repository build state:

```sh
fresh_target=$(mktemp -d /tmp/lkjscript-fresh-target.XXXXXX)
TIMEFORMAT='fresh_release_build_elapsed_s=%3R'
time CARGO_TARGET_DIR="$fresh_target" cargo build --workspace --release --locked
```

Observed elapsed time: 6.665 s. The fresh release directory occupied 7.9 MiB.

A separate fresh target ran the complete test boundary:

```sh
fresh_target=$(mktemp -d /tmp/lkjscript-fresh-test.XXXXXX)
TIMEFORMAT='fresh_full_test_elapsed_s=%3R'
time CARGO_TARGET_DIR="$fresh_target" \
  cargo test --workspace --all-targets --all-features --locked
```

Observed elapsed time: 2.122 s; 29 tests passed and the manual performance baseline was ignored.
`/usr/bin/time` was unavailable, so retained maximum-RSS evidence is not available.

### Product-path scalar baseline

The retained ignored integration test starts the real foreground daemon binary in a new temporary
directory and sends production protocol requests. It performs one warm-up, samples summary and run
31 times, shuts down, then samples workspace restart 11 times. Percentiles use nearest-rank
ordering. Request wall time includes a new Unix-socket connection and one frame each way but not a
new client process. Internal compile/execute timings exclude IPC.

```sh
cargo test --release --test semantic_vertical product_path_performance_baseline -- \
  --ignored --nocapture --test-threads=1
```

| Measurement | Median | p95 | Samples |
| --- | ---: | ---: | ---: |
| workspace query, request wall | 7.504 us | 9.769 us | 31 |
| run, request wall | 7.344 us | 9.087 us | 31 |
| direct SPG validation/lowering/Core IR verification | 0.481 us | 0.782 us | 31 |
| interpreter execution | 0.070 us | 0.130 us | 31 |
| daemon restart with one retained workspace | 5,314.193 us | 5,347.456 us | 11 |

The retained run began at `2026-08-14T09:53:48Z`. Single observations: initial daemon readiness
5,319.614 us; durable workspace creation 10,093.611 us; durable bootstrap transaction 7,900.848 us.
The revision-1 artifact was 501 bytes. Its full wall time was 5.650 s including a release rebuild.

## Pre-change hole-discovery baseline

The required pre-change observation used a detached disposable worktree at
`a503c0b1269ed3e149f83bb0f8ad8d4f75550cbc`. A temporary ignored integration test reused the real
foreground daemon, old protocol-v1 `Client`, and the existing incomplete-hole fixture. Fixture
creation was excluded. For each discovery request it measured production framed binary bytes with
`encoded_request_size`/`encoded_response_size` and wall time around `Client::request`; the worktree
and instrumentation were removed afterward.

```sh
git worktree add --detach /tmp/lkjscript-prechange a503c0b1269ed3e149f83bb0f8ad8d4f75550cbc
# append disposable prechange_hole_discovery_measurement to tests/semantic_vertical.rs
cargo test --release --test semantic_vertical prechange_hole_discovery_measurement --locked -- \
  --ignored --nocapture --test-threads=1
git worktree remove --force /tmp/lkjscript-prechange
```

| Old request | Request | Response | Elapsed | Whole node record |
| --- | ---: | ---: | ---: | --- |
| blockers | 39 B | 99 B | 15,850 ns | no |
| hole exact view | 64 B | 130 B | 15,219 ns | yes |
| owner block exact view | 64 B | 168 B | 11,872 ns | yes |
| owner region summary | 64 B | 118 B | 9,979 ns | no |
| function exact view | 64 B | 178 B | 10,931 ns | yes |
| prior operation exact view | 64 B | 137 B | 8,807 ns | yes |
| following operation exact view | 64 B | 149 B | 8,476 ns | yes |
| **total** | **423 B** | **979 B** | **81,134 ns** | **5 of 7** |

The largest old response was 178 B. Seven daemon requests were required to identify the hole and
expected type, read its owner block, reconstruct nearby order, walk manually to the function, and
inspect the immediate preceding/following operations. The simple fixture's one visible `i64`
candidate and one incoming `add_i64` use could then be inferred manually from those exact records.
The old protocol had no visibility/dominance query and no incoming-use query, so those two facts were
not directly available in the general case without recursively traversing and interpreting node
records. It also had no legal-constructor fact. No whole-workspace dump was requested, but five
whole node records were required. No model was invoked and no token estimate was made.

The current correct workflow is three real generic-CLI requests (repair context, refinement, run).
Its one context response directly composes typed owner/body/visibility/use/constructor facts; it
requires no whole node record. The old 7-request discovery-only number and current 3-request
completion number are therefore direct round-trip evidence, not an equal-payload latency or byte
comparison.

## Agent-repair campaign environment

Measurements below use the uncommitted campaign tree based on
`a503c0b1269ed3e149f83bb0f8ad8d4f75550cbc`, Linux 7.0.0-29-generic x86-64, 32 logical CPUs visible,
AMD Ryzen 9 9955HX, 32 GiB memory, and Rust 1.96.0. They are single observations unless a sample
count is shown. Shell `time` supplied elapsed time; `/usr/bin/time` remains unavailable, so maximum
RSS was not measured.

### Real generic-CLI repair cost

`agent_repair_cost_measurement` starts the real daemon and launches the real generic JSON CLI once
per request. After an incomplete 11-node workspace exists, the simple correct workflow is exactly
repair context, `RefineHole`, and revision-bound run. The result oracle is `i64(42)`. The separate
invalid edit wires `bool` into `add_i64`, rejects once, and does not count toward the three-request
correct workflow. Blocker discovery, one complete diff page, and workspace summary are also reported
separately. No whole-workspace dump was requested. The retained observation was rerun after final
request- and response-side EOF hardening.

```sh
cargo test --release --test agent_repair_json agent_repair_cost_measurement --locked -- \
  --ignored --nocapture --test-threads=1
```

| Correct workflow request | Elapsed | JSON request | JSON stdout | Binary request | Binary response |
| --- | ---: | ---: | ---: | ---: | ---: |
| repair context | 399,271 ns | 373 B | 3,993 B | 98 B | 1,274 B |
| refine | 8,639,555 ns | 680 B | 485 B | 154 B | 154 B |
| run | 343,576 ns | 167 B | 150 B | 63 B | 40 B |
| **total (3 CLI invocations/round trips)** | **9,382,402 ns** | **1,220 B** | **4,628 B** | **315 B** | **1,468 B** |

Largest correct-workflow JSON response and context response were both 3,993 B; compact receipt was
485 B. Separate observations: blockers 408,448 ns / 207 B request / 386 B stdout; deterministic
invalid edit 426,873 ns / 627 B / 325 B; diff page 371,639 ns / 223 B / 1,317 B; workspace summary
287,200 ns / 188 B / 478 B. The invalid scenario had one rejected edit before success. Exact
production binary sizes for those four responses were 127 B, 141 B, 346 B, and 162 B respectively.

These are byte and round-trip measurements, not token measurements. No model was invoked and no
model-token conversion is claimed. CLI process startup dominates the correct refinement observation;
the measurement intentionally includes it because that is the external agent boundary.

### Scan-query cost

`query_performance_measurement` warms each direct derived query once and samples it 31 times in a
release test. Median is sorted sample 16 and p95 is sample 30. JSON result bytes are compact
`serde_json` encoding of the typed result and exclude envelope/framing. The implementation uses full
scans and no index/cache.

```sh
cargo test --release query_performance_measurement --locked -- \
  --ignored --nocapture --test-threads=1
```

| Workload/query | Nodes | Median | p95 | Result bytes |
| --- | ---: | ---: | ---: | ---: |
| scalar workspace summary | 11 | 60 ns | 61 ns | 282 B |
| scalar body | 11 | 170 ns | 280 ns | 1,057 B |
| scalar incoming uses | 11 | 220 ns | 290 ns | 372 B |
| scalar repair context | 11 | 922 ns | 1,412 ns | 3,799 B |
| scalar adjacent diff | 11 | 681 ns | 751 ns | 293 B |
| scalar non-adjacent diff, revision 0 to 2 | 11 | 1,032 ns | 1,423 ns | 1,295 B |
| scalar four-item validated query batch | 11 | 1,483 ns | 2,415 ns | 5,515 B |
| first 256 of body after 3,000 added operations | 3,011 | 12,734 ns | 14,558 ns | 49,546 B |
| repair context in that body | 3,011 | 180,018 ns | 205,497 ns | 8,713 B |
| repair context with 3,000 unrelated packages | 3,011 | 16,621 ns | 17,653 ns | 3,796 B |
| incoming uses with 3,000 unrelated packages | 3,011 | 3,226 ns | 3,336 ns | 372 B |

The non-adjacent row is a follow-up run of the same 31-sample release harness after adding explicit
revision-0-to-2 coverage; the other rows retain the prior campaign observation. The full-scan trend
is visible but remains sub-millisecond at 3,011 nodes on this microbenchmark. Reversal condition:
add one narrow derived index only after representative repeated agent workloads
show scan cost material to end-to-end latency, retain scan differential tests as oracle, and prove
exact invalidation. Do not infer asymptotic or application performance from these fixtures.

### Current scalar product-path rerun

The retained scalar test was rerun without replacing the old baseline: startup 11,714.256 us,
workspace creation 10,267.075 us, durable transaction 8,054.705 us; request-wall summary median/p95
10.029/13.595 us; run 8.936/10.570 us; compile 0.390/0.621 us; execute 0.070/0.100 us; restart
median/p95 5,341.233/5,413.289 us (11 samples); artifact 501 B.

### Build, dependencies, and binaries

Fresh target directories avoided cleaning repository state. Current fresh release build took
21.790 s and 34 MiB, compared with retained 6.665 s / 7.9 MiB. A separate fresh full test took
6.556 s and 119 MiB, compared with 2.122 s; 70 active tests passed and four manual tests were
ignored. That fresh test preceded test-only measurement refinement and the final focused boundary
hardening; the final cached full boundary passed with 76 active tests and four ignored manual tests. An unchanged incremental release observation was 0.023 s.

Direct normal dependencies increased from 3 to 5; unique normal `cargo tree` packages from 10 to 21.
The two direct additions are `serde` with only `derive,std` and `serde_json` with only `std`, both
with default features disabled. Resolved versions were 1.0.229 and 1.0.151. The 11-package normal
transitive delta is serde/core/derive, serde_json, itoa, memchr, zmij, proc-macro2, quote, syn, and
unicode-ident. Licenses are MIT or Apache-2.0 compatible with this Apache-2.0 project (memchr also
offers Unlicense; unicode-ident also carries Unicode-3.0). Their current named consumer is strict,
closed, exhaustively tested machine JSON projection. A local JSON parser/serializer would duplicate
Unicode, numeric, escaping, recursion, duplicate/unknown-field, and streaming-output security work
at higher maintenance and review risk.

| Release binary | Retained baseline | Current | Delta |
| --- | ---: | ---: | ---: |
| `lkjscript` | 418,416 B | 1,401,952 B | +983,536 B (+235.1%) |
| `lkjscriptd` | 806,920 B | 921,600 B | +114,680 B (+14.2%) |

The client size and fresh-build regressions are accepted campaign costs for the generic strict JSON
boundary, not performance wins. Reversal condition: reconsider serde feature use or projection
placement only if distribution/build constraints become material while retaining identical strict
coverage and one typed authority.

### Durability, mutation, and remaining baselines

Compact default receipts are preflighted independently of full diff size, and HEAD2 remains under
16 KiB even at maximum 64 selected bindings (the focused test asserts it is under 4 KiB). A
moderate 200-create transaction proves receipt size remains selected-projection bounded. Persistence
still clones the full snapshot, materializes the full semantic diff during preparation, and rewrites
a full canonical artifact; the scalar transaction/restart observations above do not justify a
journal, database, or incremental store.

The final deterministic malformed-boundary release smoke completed 10,000 cases with seed 1 in
0.02 s of reported test time (release compilation excluded):

```sh
LKJSCRIPT_MUTATION_SEED=1 LKJSCRIPT_MUTATION_CASES=10000 \
  cargo test --release boundary_mutation_smoke --locked -- \
  --ignored --nocapture --test-threads=1
```

It mutates artifact, binary protocol, and JSON byte corpora deterministically. Typed transaction
atomicity is exercised by a separate deterministic generated-sequence test in the normal suite; it
is not part of the seed/case byte-mutation loop. The smoke is explicitly not coverage-guided
fuzzing. A future coverage-guided harness remains warranted for the three byte decoders; this
bounded smoke does not support a fuzz-coverage claim.

Full snapshot cloning/recomputation, full diff materialization, full artifact rewrite, and retained
full history remain deliberate baselines. Reverse them only with representative workload evidence,
an unchanged semantic oracle, deterministic artifacts/receipts, and durability failure-injection
evidence.
