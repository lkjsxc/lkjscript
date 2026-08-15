# Performance evidence

No performance leadership claim is made. These are bootstrap baselines whose purpose is to expose
costs before optimization.

## Nominal-data campaign environment

Measurements were retained on 2026-08-15 from the dirty campaign working tree based on starting
commit `99d7ca5bbdac6bcf90fdd64721c13df1342ef67a` on `main`: `devbox`, Linux
7.0.0-29-generic x86-64, AMD Ryzen 9 9955HX (32 logical CPUs visible), 32 GiB memory,
`rustc 1.96.0 (ac68faa20 2026-05-25)`, and Cargo 1.96.0. The cgroup memory ceiling was
34,359,738,368 B, CPU quota was unlimited, shell stack limit was 8 MiB, and virtual memory was
unlimited. Runtime and schema harnesses used optimized release binaries. The machine schema has seven sections and canonical digest
`983614734f16b5d2095279fb5e958814e839caaa7aa25a5a6963cfca44795e2d`.
`/usr/bin/time` is unavailable, so maximum RSS is unmeasured. No model was invoked; bytes are not
model tokens or API cost, and no token or performance-leadership claim is made.

## Closed machine-schema projections

The retained byte harness is:

```sh
cargo test --release --lib machine::tests::schema_projection_byte_measurements_are_retained \
  --locked -- --nocapture
```

It serializes compact result JSON without a newline and the production framed binary
`Response::DescribeSchema` with request ID 1. The six-section projection requests
`semantic_types_and_nodes`, `nominal_declarations`, `transactions_and_expressions`,
`queries_and_repair`, `runtime_and_run`, and `errors_and_limits` together.

| Projection | Compact result JSON | Framed binary response | Local / daemon round trips |
| --- | ---: | ---: | ---: |
| old audited full (prior schema authority; unequal workload) | 21,516 B stdout | 9,166 B | 0 / 1 |
| current manifest | 739 B | 736 B | 0 / 1 |
| current six nominal sections | 86,009 B | 86,006 B | 0 / 1 |
| current full | 126,888 B | 96,083 B | 0 / 1 |
| current known-digest unchanged | 105 B | 48 B | 0 / 1 |

The old row is retained only as a separately labelled historical observation and is not an equal-schema
regression comparison. `lkjscript schema` computes locally; daemon `DescribeSchema` returns the same
projection in one request/response. The current nominal interaction harness below measures enveloped
manifest/six-section/unchanged requests and responses through the daemon.

After one warm-up per command, Python `time.monotonic_ns` measured 31 new release CLI processes per
projection. Percentiles use nearest rank; stdout includes the CLI newline.

```sh
target/release/lkjscript schema
target/release/lkjscript schema \
  --section semantic_types_and_nodes --section nominal_declarations \
  --section transactions_and_expressions --section queries_and_repair \
  --section runtime_and_run --section errors_and_limits
target/release/lkjscript schema --full
target/release/lkjscript schema --full --known-digest \
  983614734f16b5d2095279fb5e958814e839caaa7aa25a5a6963cfca44795e2d
```

| Local projection | Stdout | Samples | Median | p95 |
| --- | ---: | ---: | ---: | ---: |
| manifest | 740 B | 31 | 882,409 ns | 1,142,849 ns |
| six nominal sections | 86,010 B | 31 | 1,173,126 ns | 1,342,766 ns |
| full | 126,889 B | 31 | 1,201,760 ns | 1,511,603 ns |
| unchanged | 106 B | 31 | 852,934 ns | 980,233 ns |

## Nominal Reading/Input application

The retained real generic-CLI measurement is:

```sh
cargo test --release --test agent_repair_json nominal_agent_interaction_cost_measurement \
  --locked -- --ignored --nocapture --test-threads=1
```

It uses the production daemon and launches the strict generic CLI for each measured request. One
transaction creates Reading/Input plus seven functions, requests 18 selected bindings, expands to 97
canonical nodes, and publishes an incomplete revision. One intentional type error is followed by an
identity-preserving product refinement, layout and diff queries, Run, typed shutdown, restart, and a
retained-node query. Typed shutdowns are lifecycle requests and are explicitly excluded from agent
round trips.

| Request | JSON request | JSON stdout | Binary request | Binary response | CLI + daemon wall |
| --- | ---: | ---: | ---: | ---: | ---: |
| schema manifest | 110 B | 817 B | 17 B | 736 B | 1,697,093 ns |
| six schema sections | 272 B | 86,087 B | 31 B | 86,006 B | 2,343,348 ns |
| known digest unchanged | 354 B | 183 B | 63 B | 48 B | 1,624,797 ns |
| workspace creation | 69 B | 325 B | 15 B | 120 B | 11,220,107 ns |
| structured nominal creation | 8,297 B | 1,256 B | 1,517 B | 658 B | 8,975,464 ns |
| Reading repair context | 376 B | 6,556 B | 98 B | 2,298 B | 1,365,880 ns |
| invalid identity-keyed repair | 882 B | 329 B | 221 B | 142 B | 849,348 ns |
| valid identity-keyed repair | 935 B | 488 B | 237 B | 154 B | 8,626,779 ns |
| Reading layout | 266 B | 741 B | 85 B | 292 B | 483,539 ns |
| semantic diff | 224 B | 1,547 B | 69 B | 464 B | 519,407 ns |
| main Run | 234 B | 156 B | 83 B | 40 B | 459,544 ns |
| retained refined-hole query after restart | 247 B | 548 B | 81 B | 185 B | 447,061 ns |
| **total** | **12,266 B** | **99,033 B** | **2,517 B** | **91,143 B** | **38,612,367 ns** |

There are 12 measured CLI invocations and daemon round trips: 11 successful semantics and one expected
semantic error. Cold daemon readiness was 5,358,601 ns and restart readiness was 5,422,811 ns.
Revision-1 and revision-2 artifacts are 4,213 B and 4,256 B; HEAD is 268 B. The Reading layout oracle
is size 16, alignment 8, and two runtime cells. The measured main Run returned `i64(42)` with
52,298 ns compile/lower/verify and 11,471 ns interpreter execution. These timing rows are one
observation each, not distributions.

The retained repeated harness is:

```sh
cargo test --release --test agent_repair_json nominal_reading_performance_measurement \
  --locked -- --ignored --nocapture --test-threads=1
```

It performs one warm-up and 31 measured generic-CLI Run requests per route.

| Measurement | Samples | Median | p95 |
| --- | ---: | ---: | ---: |
| main request wall | 31 | 414,680 ns | 510,490 ns |
| main compile/lower/verify | 31 | 19,987 ns | 60,103 ns |
| main interpreter execution | 31 | 3,186 ns | 10,851 ns |
| nominal Input match request wall | 31 | 423,266 ns | 533,664 ns |
| nominal Reading output request wall | 31 | 414,580 ns | 498,878 ns |

The typed oracles are main `42`, sample payload `5`, and returned Reading value `9`. A separate single
layout query took 445,448 ns and reasserted size 16/alignment 8/two cells. A single restart took
5,341,389 ns and reasserted the exact retained layout; no median or p95 is claimed for either single
observation.

### Fresh build, test, binary, and boundary evidence

Fresh targets were newly allocated under `/tmp`; no Cargo target was cleaned or reused:

```sh
FRESH_RELEASE_TARGET=/tmp/lkjscript-final2-release.lrOvgc
FRESH_TEST_TARGET=/tmp/lkjscript-final2-test.MCHhFp
TIMEFORMAT='fresh_release_build_elapsed_s=%3R'
time CARGO_TARGET_DIR="$FRESH_RELEASE_TARGET" cargo build --workspace --release --locked
TIMEFORMAT='fresh_full_test_elapsed_s=%3R'
time CARGO_TARGET_DIR="$FRESH_TEST_TARGET" \
  cargo test --workspace --all-targets --all-features --locked
du -sk "$FRESH_RELEASE_TARGET" "$FRESH_TEST_TARGET"
stat -c '%n %s' "$FRESH_RELEASE_TARGET/release/lkjscript" \
  "$FRESH_RELEASE_TARGET/release/lkjscriptd"
TIMEFORMAT='incremental_release_build_elapsed_s=%3R'
time cargo build --workspace --release --locked
```

The final fresh release build took 56.476 s and occupied 52,207 KiB. Its client was 3,779,472 B and
daemon 2,245,680 B. The separate fresh full test took 17.948 s, occupied 457,561 KiB, and reported 166
active passes with eight ignored manual measurement/smoke tests. The unchanged incremental release
build took 0.024 s; repository release binaries had the same sizes. The temporary targets were removed
only after sizes were recorded. Compared with the separately retained structured and reset baselines
below, these are accepted capability/build regressions, not equal-work performance claims.

The full boundary specifically passed malformed Core aggregate/switch rejection, exhaustive selected-arm
execution, strict malformed JSON and protocol rejection, exact aggregate copy fuel, selected-large-arm
fuel exhaustion, entry/callee/live-cell exhaustion, nominal restart, exact cycle-participant
selection, and iterative deep match/type traversal tests. `examples/nominal-match/run.sh` additionally
proves an overflowing arm is lazy when
unselected and traps when selected through the public Run path. The final deterministic boundary command was:

```sh
LKJSCRIPT_MUTATION_SEED=1 LKJSCRIPT_MUTATION_CASES=10000 \
  cargo test --release --lib campaign_tests::boundary_mutation_smoke --locked -- \
  --ignored --nocapture --test-threads=1
```

It passed one test in 0.03 s and printed `seed=1 cases=10000`; this is bounded deterministic mutation
testing, not coverage-guided fuzzing.

## Structured pure-program campaign (retained older baseline)

### Environment and method

Measurements were retained on 2026-08-15 from the final campaign tree based on
`dc541eb3ebb7a54006e8057d0f76b0596cf012e4`: `devbox`, Linux 7.0.0-29-generic x86-64,
Rust/Cargo 1.96.0, AMD Ryzen 9 9955HX, 32 GiB memory. Runtime harnesses use release binaries and one
warm-up before reported samples. Oracles are typed `i64(5050)`, `i64(0)`, `i64(55)`,
finite-recursion `i64(1)`, and `execution_fuel_exhausted`. Percentiles use nearest rank. Shell
`time` measured fresh commands because `/usr/bin/time` is unavailable; maximum RSS therefore
remains unmeasured.

The retained real generic-CLI interaction harness is:

```sh
cargo test --release --test agent_repair_json structured_agent_interaction_cost_measurement \
  --locked -- --ignored --nocapture --test-threads=1
```

It launches the production CLI for every request. The one structured creation has 6 public
transaction items, requests 4 explicit bindings, and expands to 36 canonical nodes. Revision-1 and
revision-2 artifacts are 1,584 B and 1,601 B. The complete measured sequence uses 11 CLI
invocations/daemon round trips: schema discovery, workspace creation, structured creation, repair
context, invalid repair, valid repair, semantic diff, three runs, and one retained query after
restart.

| Request | JSON request | JSON stdout | Binary request | Binary response | CLI wall |
| --- | ---: | ---: | ---: | ---: | ---: |
| schema discovery (then-default full) | 67 B | 21,516 B | 15 B | 9,166 B | 655,402 ns |
| workspace creation | 68 B | 324 B | 15 B | 120 B | 10,098,864 ns |
| structured creation | 3,106 B | 660 B | 527 B | 266 B | 8,934,375 ns |
| repair context | 375 B | 5,472 B | 98 B | 1,761 B | 440,939 ns |
| invalid repair | 375 B | 310 B | 85 B | 137 B | 430,550 ns |
| valid repair | 577 B | 487 B | 136 B | 154 B | 9,040,735 ns |
| semantic diff | 224 B | 1,215 B | 69 B | 356 B | 477,839 ns |
| main run | 234 B | 157 B | 83 B | 40 B | 411,544 ns |
| direct `-3` run | 258 B | 151 B | 92 B | 40 B | 318,298 ns |
| direct `11` run | 258 B | 153 B | 92 B | 40 B | 301,657 ns |
| restart retained query | 246 B | 503 B | 81 B | 161 B | 385,344 ns |
| **total** | **5,788 B** | **30,948 B** | **1,293 B** | **12,241 B** | **31,495,547 ns** |

Daemon cold readiness was 6,429,787 ns and restart readiness with the retained workspace was
4,294,382 ns. These are byte, process, and round-trip measurements, not model-token or API-cost
measurements. That campaign's then-default full schema discovery is intentionally included because
the runtime-generated schema is the agent's authoritative vocabulary; the current default is the
compact manifest measured above.

The retained repeated product-path harness is:

```sh
cargo test --release --test agent_repair_json structured_product_path_performance_measurement \
  --locked -- --ignored --nocapture --test-threads=1
```

| Measurement | Median | p95 | Samples |
| --- | ---: | ---: | ---: |
| daemon cold start | 5,315,642 ns | 5,334,448 ns | 11 |
| workspace creation, generic CLI wall | 10,313,819 ns | 10,988,907 ns | 31 |
| structured incomplete commit, generic CLI wall | 9,015,217 ns | 9,959,452 ns | 11 |
| nested repair context, generic CLI wall | 353,384 ns | 459,944 ns | 31 |
| main request wall | 297,600 ns | 490,022 ns | 31 |
| main compile/lower/verify | 3,927 ns | 15,068 ns | 31 |
| main interpreter execution | 14,898 ns | 17,633 ns | 31 |
| direct parameterized run wall | 295,776 ns | 393,129 ns | 31 |
| finite recursion wall | 274,135 ns | 358,234 ns | 31 |
| controlled fuel exhaustion wall | 293,812 ns | 348,896 ns | 31 |
| daemon restart with retained workspaces | 5,327,895 ns | 6,374,162 ns | 11 |

CLI startup dominates sub-millisecond request rows. Main execution is materially larger than the old
scalar `42` interpreter micro-observation because it performs calls, a conditional, 101 loop
iterations, checked increments, and branch transfers; this is a different representative workload,
not a regression ratio for equal work.

### Fresh build, test, and binary observations

Fresh targets were separate `mktemp` directories; repository targets were not deleted or reused:

```sh
FRESH_RELEASE_TARGET="$(mktemp -d /tmp/lkjscript-structured-build.XXXXXX)"
FRESH_TEST_TARGET="$(mktemp -d /tmp/lkjscript-structured-test.XXXXXX)"
printf '%s\n' "$FRESH_RELEASE_TARGET" "$FRESH_TEST_TARGET"

TIMEFORMAT='fresh_release_build_elapsed_s=%3R'
time CARGO_TARGET_DIR="$FRESH_RELEASE_TARGET" cargo build --workspace --release --locked

TIMEFORMAT='fresh_full_test_elapsed_s=%3R'
time CARGO_TARGET_DIR="$FRESH_TEST_TARGET" \
  cargo test --workspace --all-targets --all-features --locked

du -sk "$FRESH_RELEASE_TARGET" "$FRESH_TEST_TARGET"
stat -c '%n %s' "$FRESH_RELEASE_TARGET/release/lkjscript" \
  "$FRESH_RELEASE_TARGET/release/lkjscriptd"
rm -rf -- "$FRESH_RELEASE_TARGET" "$FRESH_TEST_TARGET"

# Unchanged-worktree incremental release observation:
TIMEFORMAT='incremental_release_build_elapsed_s=%3R'
time cargo build --workspace --release --locked
stat -c '%n %s' target/release/lkjscript target/release/lkjscriptd
```

The final fresh release build was 28.519 s and 38,095 KiB. Its separate final fresh full test was
13.212 s and 249,828 KiB, with 121 active tests passing and 6 ignored manual tests. Unchanged
incremental release build was 0.033 s. Release binaries are `lkjscript` 1,786,936 B and
`lkjscriptd` 1,184,712 B.

Against the audited reset baseline, fresh release time increased from 6.665 s to 28.519 s and target
size from 7.9 MiB to about 37.2 MiB. Fresh test time increased from 2.122 s to 13.212 s. The client
binary increased from 418,416 B by 1,368,520 B (+327.1%); the daemon increased from 806,920 B by
377,792 B (+46.8%). Against the prior JSON/repair campaign observation, the client grew 384,984 B
(+27.5%), the daemon 263,112 B (+28.5%), and incremental release moved from 0.023 s to 0.033 s.
These are accepted capability/build regressions, not performance wins. Reversal conditions remain a
material distribution/build constraint plus an alternative that preserves strict JSON, generated
schema, structured semantics, and one executable route.

### Authoring baseline comparison

The audited scalar low-level baseline required 11 public transaction items for its small `40 + 2`
program. The new representative request uses 6 items (45.5% fewer) while defining three functions,
two parameters, calls, an `if`, a loop, and a nested hole. This item-count comparison is semantic
interaction evidence across different capabilities, not equal-work latency or wire evidence. Calls
and structured control were not expressible, so there is no honest production old wire measurement
for the complete representative program.

A disposable detached worktree at `dc541eb3` was temporarily instrumented and removed after measuring
the largest directly comparable old production subset used here: one package, one module, and three
zero-parameter functions, each with an explicit region, block, constant, return terminator, and body
attachment; `main` returns a constant `5050`. The old product had no parameters, calls, `if`, loop,
block arguments, or nested hole with which to encode the missing representative meaning. One release
sample through the real generic CLI/daemon produced:

| Old production subset dimension | Observation |
| --- | ---: |
| public transaction items | 21 |
| returned handles / created nodes | 17 / 17 |
| semantic nodes including initial workspace root | 18 |
| functions / regions / blocks | 3 / 3 / 3 |
| operation nodes / return terminators | 6 / 3 |
| function-body / package-entry attachments | 3 / 1 |
| compact JSON request / stdout response | 2,849 B / 1,210 B |
| framed binary request / response | 477 B / 630 B |
| one transaction CLI wall | 8,961,607 ns |
| revision artifact | 813 B |

The disposable procedure was `git worktree add --detach "$OLD_WT" dc541eb3`, append the temporary
focused test `temporary_old_low_level_representative_subset_measurement` to
`tests/agent_repair_json.rs`, run `cargo test --release --test agent_repair_json
 temporary_old_low_level_representative_subset_measurement --locked -- --nocapture
 --test-threads=1`, then run `git worktree remove --force "$OLD_WT"` and `git worktree prune`.
Temporary instrumentation and its build directory were removed; the table retains its complete typed
shape assumptions and observed dimensions without turning measurement code into an active old API.

The implemented structured creation uses 6 items, returns only 4 selected bindings, creates 36 nodes,
and measures 3,106/660 JSON request/response bytes, 527/266 binary request/response bytes,
8,971,224 ns one-transaction CLI wall, and a 1,584 B incomplete artifact. It therefore uses 71.4%
fewer public items while expressing materially more semantics. Its request is 9.0% larger in JSON and
10.5% larger in binary than the inexpressive old subset; its selected-binding response is 45.5%
smaller in JSON and 57.8% smaller in binary. Artifact and latency observations are different-workload
context, not equal-work regressions or wins.

No exact old binary dimension exists for the nested call/control scaffolding: the old production
binary codec has no variant tags or payload grammar for parameters, calls, `if`, loops, block
arguments, or structured holes. An "exact synthetic binary" would therefore require inventing a
non-production protocol and is intentionally not reported. The JSON structural model below is kept
explicitly synthetic because JSON can at least state named hypothetical records without claiming the
old decoder accepted them.

For structural comparison only, an exact synthetic compact-JSON encoder counted the representative
canonical shape as 36 created nodes: 1 package, 1 module, 3 functions, 2 parameters, 6 regions,
6 blocks, 2 block arguments, and 15 operations/terminators. Under the stated assumption that the old
shape needed one explicit create item per canonical node, three explicit function-body attachments,
and one entry selection, it produced 40 transaction items and 6,804 compact JSON bytes. The
synthetic payload used full field names (`create_canonical_node`, `local_handle`,
`canonical_node_kind`, `owner`, `owner_slot`), excluded the semantic payloads for calls/control that
the old product could not express, and omitted a versioned envelope. It is therefore a lower-bound
structural estimate, not old production bytes and not an old binary measurement.

The implemented structured request uses 6 semantic transaction items and 3,106 JSON bytes including
the versioned envelope and all call/control payloads: 85.0% fewer public items than the synthetic
shape and 54.4% fewer bytes than that deliberately incomplete structural lower bound. This is
semantic compression from implied regions/blocks/arguments/terminators and aggregate function
bodies, not abbreviated naming. No token saving is inferred.

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
6.556 s and 119 MiB, compared with 2.122 s. Those were historical intermediate campaign
observations; their test counts are superseded by the structured campaign boundary above. The
historical unchanged incremental release observation was 0.023 s.

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

Compact default receipts are preflighted independently of full diff size, and `LKJHEAD3` remains under
16 KiB even at maximum 64 selected bindings (the focused test asserts it is under 4 KiB). A
moderate 200-create transaction proves receipt size remains selected-projection bounded. Persistence
still clones the full snapshot, materializes the full semantic diff during preparation, and rewrites
a full canonical artifact; the scalar transaction/restart observations above do not justify a
journal, database, or incremental store.

The final deterministic malformed-boundary release smoke completed 10,000 cases with seed 1 in
0.03 s of reported test time (release compilation excluded):

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
