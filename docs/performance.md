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

## Verification critical path

**Measured decision: keep Cargo's native runner, move excess owned-cleanup scale work out of the
default boundary, and make retained Cargo-cache invalidation source-correct.** The hypothesis was
that test execution, rather than compilation, dominated the warm native verification sequence and that the default 1,024-owned-argument fixture repeated work
beyond the representation boundaries it protects. The candidate had to preserve every asserted
width, ownership, cleanup, evaluator, VM, native-decline, and resource-failure outcome; retain the
1,024 case as a locked-release stress; reduce the three-sample median native boundary by at least
15%; and remain smaller than a runner or scheduling change. The reversal condition is a missed defect
at the retained larger geometry, failure of the default fixture to cross one of its checked
boundaries, changed test selection outside this fixture, hidden failure output, or loss of the
repeatable material benefit.

The unchanged commands were Rustfmt, workspace all-target/all-feature Clippy, workspace all-target/
all-feature tests, and the locked workspace release build in their documented serial order. The
baseline was clean commit `353236260c5d388ca7dbd3379f8822a49e6c9544`; the candidate used that
base plus dirty diff SHA-256
`b422c4111cf94e47750c62340f16cf839594e35de75acc667032a43deda4c8e2`. Environment: `devbox`, Linux
`7.0.0-27-generic` x86-64, AMD Ryzen 9 9955HX, 20 available logical CPUs, 32 GiB RAM, `powersave`
governor, ZFS checkout, Rust/Cargo 1.96.0, warm Cargo artifacts, and no Rust/Cargo override. Three
serial warm samples were retained per cell; p95 below is the maximum of three and only tail
orientation. Monotonic process wall time and child CPU were recorded directly. Approximate peak
process-tree RSS used a 50 ms `/proc` poll because GNU `/usr/bin/time` was unavailable; it is not
unique physical memory.

A warm `cargo test --no-run` took 0.104 s with empty output, while the corresponding first full test
command took 141.381 s. The serial libtest targets included 26.60 s, 7.01 s, 3.90 s, 65.26 s, 9.24 s,
and 24.14 s runs; compilation was therefore not the selected bottleneck. `cargo-nextest` was not
installed, but adding or pinning it was unnecessary: one generated fixture accounted for about
46% of the full test wall time. The retained Docker baseline corroborated the shape: its verified
stage spent 66.37 s in the same target, while all post-build product checks and smokes together took
about 2 s before the timing-only benchmark. Bounded smoke parallelism was lower leverage and was not
implemented.

The first container candidate exposed a correctness defect rather than a speed result. The source in
the image contained the new ten-test/two-ignored layout, but the successful `cargo test` output still
ran the old nine-test/one-ignored layout and its 1,024 default case. BuildKit retained Cargo target
artifacts whose mtimes were newer than `COPY`-preserved changed source mtimes, so Cargo reused a stale
test executable. The Docker build now refreshes Cargo input mtimes whenever its source layer changes.
Only Cargo inputs enter the release-build stage; product package/source, benchmark, and smoke inputs
enter the derived verified stage, and unconsumed policy, README, documentation, and Compose copies
were removed. This makes a Rust change rebuild the affected cached artifacts while a smoke-script-only change no longer
invalidates the release build. The fixed retained run visibly executed ten tests with two ignored and
the 640 case in 21.41 s.

The candidate adds a 640-owned-argument default case through the exact existing helper and marks the
unchanged 1,024 case ignored for locked-release stress. The default still dynamically proves byte-
wide local/place crossings, a main bytecode stream above `u16::MAX`, more than 65,535 logical cleanup
actions, near-linear physical cleanup storage, evaluator/VM outcome `7`, preferred-route outcome,
and call-unentered resource-failure cleanup. A 512 probe was rejected because it did not cross the
bytecode-length boundary; 640 was the first retained passing probe after that failure. The 1,024
ignored release case passed unchanged with 3.85 s in-process execution. No product code, documented
verification command, runner, dependency, feature, target, or concurrency changed; the Docker stage
inputs and cache-validity step changed only to remove unconsumed invalidation and prevent stale
executable reuse.

| Cell | Wall median (p95) | User CPU median | Approx. peak RSS median | Success output / stderr |
| --- | ---: | ---: | ---: | ---: |
| Baseline four-command native boundary | 142.335 s (142.698) | 201.665 s | 446.7 MiB | 5,188 B / 0 B |
| Retained default fixture | 98.245 s (98.990) | 158.180 s | 431.7 MiB | 5,052 B / 0 B |

The selected boundary fell by 44.089 s, or 31.0%; user CPU fell 21.6%, and approximate peak RSS did
not increase. The affected test-target median fell from 64.75 s to 21.17 s while retaining eight
default tests and moving only the larger equivalent geometry behind the existing ignored-release
policy. This meets the predeclared threshold without concurrent heavyweight commands, extra CI
compute, port/file isolation work, or a new maintenance dependency. One mixed-cache exact retained
container orientation fell from 233.069 s to 195.083 s (16.3%); its verified command stage fell from
160.2 s to 112.3 s (29.9%). The release-build cells were not cache-equivalent (64.1 s versus 75.9 s),
so these single container samples corroborate the test-work shape but do not establish a general
container speedup. Keep the smaller default while its dynamic boundary assertions and focused larger
stress both pass. Reconsider a mature runner only if independent target execution remains dominant
after direct fixture simplification and measured latency justifies installation, output, filtering,
doctest, CI, and Docker costs.

Raw native baseline and candidate result documents remain ignored with SHA-256 values
`5838129b7d2686f637cb639ffd8ec9a5e5dacdbe0f6662c011c2ca096add6459` and
`65de2d6d0973925eeae5c0b6820a74e34bd79ed07f6dc0c8a6279f8b000cec23`; recorded environment metadata
has SHA-256 `fde9249b138681e94640de6d3b44af4b711198759ccfb88908b2a9998d90a129`.
The exact baseline, stale-candidate, and fixed-candidate container stdout logs have SHA-256 values
`524240a2d7b677b5347373b5d34950a18ba2dbbb26e1e791da437fcc79dc29b8`,
`8815f94c372dcd2de3455c93694516c56e10e4e3b92a211c81041fe445567cac`, and
`a9e91e3a6348e17677b67a60678e3c9bea40eda270e6933bf5d0744827eddbf3`.

## Semantic-workspace recomputation boundary

**Measured decision: retain the local metadata correction and keep full recomputation for semantic
edits.** The falsifiable hypothesis was that a small source-free edit in a wider mixed workspace
would cause predictable whole-program work without a hidden superlinear cliff, while complete
compilation rather than transaction/query/projection work would dominate the agent check loop. A
candidate required at least 10% representative-medium end-to-end benefit or 20% in a clearly dominant
phase, exact semantic equivalence, and a mechanism smaller than the removed work. The reversal
condition remains any semantic, identity, diff, diagnostic, blocker, ownership/cleanup, pagination,
old-snapshot, or failure-atomicity divergence. No incremental graph, cache, daemon, persistence layer,
transaction planner, or second semantic authority was added.

The retained harness is
[`workspace-recompute.py`](../meta/scripts/workspace-recompute.py) plus the ignored locked-release
sampler and fast exactness tests in
[`recompute_measurement.rs`](../crates/lkjscript-compiler/src/workspace/recompute_measurement.rs).
One release test binary constructs every fixture through semantic transactions, times fixture setup
separately, performs one real revision-publishing edit, issues structured queries and a selected
projection, compiles directly, and intentionally enters the VM only for exact outcome confirmation.
The local v2 JSON is measurement output, not a language or service protocol. The corpus is:

| Workload | Exact selected operation and outcome |
| --- | --- |
| W0 scalar | one-node `main`, replace `7` with `8`, direct entity/node facts, projection, compile, VM |
| W1 incomplete metadata | refine one `i64` main-hole goal amid 16/128/512 complete scalar helpers; inspect context, diagnostics, legal constructors, and incomplete compile rejection |
| W2 imperative | replace the bound in a 12-node mutable counted loop amid 16/128/512 helpers; old/new VM `100`/`101` |
| W3 ownership/return | replace a two-node early-return subtree under a live byte-vector owner after a shared borrow; retain drop/end-borrow obligations; VM `7`/`9` |
| W4 nominal/match | create a product and enum, edit one exhaustive payload-match arm, query stable member/payload/match facts; VM `42`/`43` |
| W5 mixed generic | edit the value argument of an exact `Copy`-witnessed generic call in a middle callable, with equal helper halves around it plus a product, enum, counted loop, and payload match; VM `10`/`11` |
| W6 lifecycle | create and complete a function, rename it, reject a stale revision atomically, delete a middle independent function, relocate a later private binding, and preserve survivor/node identities and old VM `5` |
| W7 incomplete recovery | introduce a real typed hole, inspect blockers/context/constructors, reject compilation before lowering, refill the same root, and run `7`/`9` |

W1 geometry is 17/129/513 entities and nodes. W2 is 18/130/514 entities and
28/140/524 nodes with a fixed 12-node affected root. W5 is 33/145/529 entities and
42/154/538 nodes; its affected generic-call root remains two nodes and the submitted draft one node.
The ignored W5 stress uses 2,048 helpers, 2,065 entities, and 2,074 nodes. These are workload
parameters, never language-validity limits.

The recorded commands were:

```sh
meta/scripts/workspace-recompute.py \
  --label accepted-representative-narrow --sizes 16,128,512 \
  --samples 5 --warmups 1 --refinement-mode narrow \
  --decision retain-local-refinement-and-full-recomputation-for-semantic-edits \
  --output target/workspace-recompute/accepted-representative-narrow.json
meta/scripts/workspace-recompute.py \
  --label accepted-w1-full-control --workloads W1 --sizes 16,128,512 \
  --samples 5 --warmups 1 --refinement-mode full \
  --decision same-binary-full-recomputation-control \
  --output target/workspace-recompute/accepted-w1-full-control.json
meta/scripts/workspace-recompute.py \
  --label accepted-stress-w5-2048 --workloads W5 --sizes 2048 \
  --samples 1 --warmups 1 --refinement-mode narrow \
  --decision ignored-release-complexity-shape-only \
  --output target/workspace-recompute/accepted-stress-w5-2048.json
```

Each representative and full-control cell discards one fresh-process warm-up and retains five
fresh-process samples with one test thread; the generated stress cell retains one sample after one
warm-up. The driver preserves exact stdout/stderr bytes and hashes beneath
`target/workspace-recompute/<label>-raw/`, validates required schema, geometry, outcome, correctness,
timing, deterministic-work, and sample-count facts, and refuses
source, binary, or worktree movement during a run. The three recorded documents have SHA-256 values
`b60e0ec5fd114a9df84d80c5dab898e8ca8d41e69ff182dfdaa3540e1cd78b6d`,
`23b72a34c374ad3ab7fe956e993da88a4662074fe4b0bcb59a8d7a121014583f`, and
`4a1565e10542fb3708e657da204f0ec824f19c86733366ea88d08883e478d7e9`.
All used base commit `79cb09b3c0eb3316994a43e38c7123e06f7b105c`, recorded dirty-worktree digest
`770895103a10ed74616bc767723f75c6c9d42f8df0a4e38fc9d3a7a8ef9bf63a`, and the same release
binary digest `f548fec3fdac87ba7cf367baec58f0969a92e9488922ec5312c7a905fdd6b4fe`.
The dirty state comprised the task-supplied `AGENTS.md` replacement and the measured implementation,
harness, ignore, and documentation changes; that exact measured state is identified by digest in raw metadata. A later complete
repeat was excluded as a whole after an isolated 50.86 ms W5/512 transaction and broad host slowdown;
its undiscarded successful samples remain outside Git for audit rather than being selectively edited.

Environment: host `devbox`, Linux `7.0.0-27-generic` x86-64, AMD Ryzen 9 9955HX, 20 available
logical CPUs, 32 GiB RAM, `powersave` governor, Rust/Cargo 1.96.0, Python 3.12.3, release test
profile, warm dependencies/artifacts, and no recorded Rust/Cargo performance override. Starting load
averages were 3.78/5.69/5.10 for the representative matrix. Timers are monotonic and exclude Cargo
and process startup. Median is exact integer arithmetic; nearest-rank p95 is the maximum of five and
only tail orientation. Durations are medians with p95 in parentheses; `check` is edit plus selected
inspection/projection plus compilation, while `+VM` adds only semantic outcome confirmation. Each
old-snapshot compile/run is an untimed semantic oracle outside these operation sums.

| Workload | Helpers | Transaction | Queries / projection | Compile | Check (p95) | +VM | Outcome |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| W0 | 0 | 9.1 us (9.8) | 1.5 / 5.8 us | 0.304 ms | 0.321 ms (0.339) | 0.345 ms | `7` -> `8` |
| W1 narrow | 512 | 7.0 us (9.9) | 60.9 / 12.6 us | 4.3 us reject | 84.7 us (129.1) | same | incomplete |
| W2 | 16 | 86.5 us (130.1) | 7.6 / 10.2 us | 0.982 ms | 1.096 ms (1.175) | 1.226 ms | `100` -> `101` |
| W2 | 128 | 0.394 ms (0.431) | 16.5 / 9.3 us | 3.095 ms | 3.522 ms (3.855) | 3.652 ms | `100` -> `101` |
| W2 | 512 | 1.616 ms (1.630) | 30.9 / 12.4 us | 10.890 ms | 12.554 ms (13.915) | 12.744 ms | `100` -> `101` |
| W3 | 0 | 37.2 us (39.4) | 2.0 / 10.9 us | 0.554 ms | 0.607 ms (0.653) | 0.638 ms | `7` -> `9` |
| W4 | 0 | 85.4 us (264.1) | 4.1 / 14.9 us | 0.607 ms | 0.710 ms (1.337) | 0.746 ms | `42` -> `43` |
| W5 | 16 | 0.242 ms (0.255) | 12.3 / 15.2 us | 1.150 ms | 1.406 ms (1.531) | 1.435 ms | `10` -> `11` |
| W5 | 128 | 0.553 ms (0.591) | 16.0 / 15.5 us | 2.356 ms | 2.906 ms (3.285) | 2.952 ms | `10` -> `11` |
| W5 | 512 | 1.851 ms (2.991) | 32.1 / 20.1 us | 6.375 ms | 8.280 ms (10.151) | 8.385 ms | `10` -> `11` |
| W6 | 0 | 26.7 us (41.2) delete | 3.6 / 10.7 us | 0.327 ms | 0.529 ms (0.643) | 0.549 ms | `5` -> `5` |
| W7 | 0 | 9.0 us (9.4) fill | 7.8 / 7.0 us | 0.281 ms | 0.316 ms (0.380) | 0.341 ms | `7` -> `9` |

The same-binary W1 control makes the retained correction reproducible without a detached source
variant. At 16/128/512 helpers, full-path versus narrow transaction medians were respectively
44.90/3.57 us, 268.37/3.25 us, and 1.153 ms/6.98 us. Check medians were 58.67/17.66 us,
294.45/30.83 us, and 1.227 ms/84.75 us: 3.3x, 9.6x, and 14.5x improvements. Projection digests,
revisions, blockers, diagnostics, queries, diffs, and public identities matched through the focused
differential test. At 512 helpers the full control cloned 512 functions, 512 bindings, and 513
nodes; compacted and inferred 513 roots; built 513 entity/node records; and examined 1,026 old/new
entity and node records. Every count is zero on the narrow route because only hole records,
diagnostics, allocator staging, net diff, and publication remain. Repeated refinements now report
one base-to-final diff per hole, and a net no-op reports no semantic diff.

Full recomputation remains predictable. W2 transaction work at 512 helpers clones 524 nodes,
compacts/infers 513 roots, builds 514 entities/524 nodes, and examines exactly 1,028 entity plus
1,048 node old/new records. Its phase medians were 98.5 us clone, 204.7 us compaction, 321.4 us
complete validation, 296.0 us index build, and 311.7 us reconciliation. W5/512 reports the analogous
one-pass counts for 538 nodes and 517 roots; clone was 112.2 us, compaction 260.2 us, validation
381.1 us, index build 396.1 us, and reconciliation 301.6 us. No repeated per-entity phase or clone
cost dominates the check loop. W2 compile instead spends 5.780 ms in canonical normalization and
2.072 ms in memory planning; W5 compile spends 2.456 ms in memory planning and 1.192 ms in SSA
construction. The 2,074-node W5 stress took 6.717 ms transaction and 38.808 ms check: versus W5/512,
4x nodes caused 3.6x transaction and 4.7x check work. This single stress sample is complexity-shape
evidence, not an application benchmark.

Direct reads remain secondary: at 512 helpers W1/W2/W5 query medians were 60.9/30.9/32.1 us and
selected projections were 12.6/12.4/20.1 us. W2 scans/sorts 1,032 candidates for three pages; W5's
two selected bodies cause 1,076 node inspections for three emitted nodes, but the absolute projection
cost is 0.3% of its check loop and does not justify another index. Selected projection output is
3,683 bytes/3 lines for W1, 1,206 bytes/14 lines for W2, and 1,027 bytes/18 lines for W5. The default
driver emits one 96-byte path line and empty stderr; each opt-in libtest sample emits 193 stdout bytes
in six lines plus one JSON marker line. These are byte/line measurements, not provider-token or
billing evidence.

The 10 ms `/proc` poll is approximate process-tree RSS, not unique physical or retained-snapshot
memory; short cells can finish between polls. At the meaningful 512-helper cells, median/p95 RSS was
20.16/20.46 MiB W1, 24.44/24.74 MiB W2, and 32.59/32.75 MiB W5. W1 narrow/full was
20.16/20.46 versus 20.41/20.88 MiB, showing no material process-peak change. Exact retained snapshot
bytes, allocator counts, allocation counts, and allocated bytes remain Unknown; every relevant
sample retained and verified an old snapshot, but RSS also includes fixture, binary, compiler, raw
buffers, and VM state.

**Decision:** keep the narrow all-`RefineHole` path and full recomputation for every
semantic-program-changing or mixed transaction. Compilation dominates representative complete
checks, whole-program transaction passes have the intended one-pass shape, and indexed queries and
selected projections are small in absolute time. Reopen the local correction if equivalent W1 loses
its material benefit or any semantic/failure-atomic fact diverges. Reopen broader recomputation when
a representative semantic edit makes transaction/query/projection work dominate the local loop,
exact retained memory becomes material, or a present consumer establishes a larger or different
snapshot lifetime. Current evidence does not justify general incrementality, a warm service,
retained HIR, another index, or a second correction. Other hosts/targets, exact allocations, broader
query density, and application-scale steady-state behavior remain unmeasured.

## Local agent-loop output boundary

**Measured output-volume correction; timings are orientation only.** The hypothesis was that an
effect-free product check, already-silent Rustfmt, and Cargo's other native quiet modes could remove
irrelevant successful output without weakening package/compiler validation, selected
targets/features, exit status, or failure
detail. The reversal condition is any hidden nonzero status or diagnostic, divergent check/run
compile semantics, or no material successful-output reduction. No wrapper, cache, daemon, or new
command runner was added.

One sample per cell was recorded at base `a4739a41cb816ba1e346b95c1af5015431ccf9be` before and in
the dirty implementation tree after the cutover. Environment: `devbox`, Linux
`7.0.0-27-generic` x86-64, AMD Ryzen 9 9955HX, 20 available logical CPUs, 32 GiB RAM,
`rustc 1.96.0 (ac68faa20 2026-05-25)`, and Cargo 1.96.0. CLI samples used the warm debug binary;
the focused test was warm in both cells. The aggregate suite used warm dependency/debug artifacts,
but the after release build rebuilt changed code, so its wall time is not comparable. Raw stdout,
stderr, and JSON summaries remain ignored under `target/agent-loop-measurements/`.

| Workload | Before stdout / stderr bytes (lines) | After stdout / stderr bytes (lines) | Status | Wall orientation |
| --- | ---: | ---: | --- | ---: |
| Product `check` of effectful hello | unavailable; command failure 0 / 34 (0 / 1) | 0 / 0 (0 / 0) | 1 before; 0 after | n/a / 1.005 s |
| `package check` success | 41 / 0 (1 / 0) | 0 / 0 (0 / 0) | 0 / 0 | 1.000 / 1.011 s |
| Focused CLI contract test, normal / `--quiet` | 163 / 158 (6 / 2) | 113 / 0 (5 / 0) | 0 / 0 | 0.035 / 0.036 s |
| Four-command full host boundary | 75,781 / 3,548 (946 / 40) | 5,152 / 0 (190 / 0) | all four 0 / all four 0 | 171.979 / 240.514 s |
| Malformed-source human failure | 0 / 162 (0 / 2) | 0 / 151 (0 / 2) | 1 / 1 | 0.010 / 0.010 s |

The full boundary after the cutover was exactly:

```sh
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
```

Compared with the prior non-quiet presentation of the same semantic set, successful aggregate
output fell from 79,329 to
5,152 bytes (93.5%) and from 986 to 190 lines (80.7%) even though the after suite contains the new
check tests. Rustfmt deliberately remains non-quiet because `--quiet --check` hides formatting
diffs. The other native quiet modes still emit complete failures: a deliberate temporary
`compile_error!("quiet failure evidence probe")` returned 101 with the source location, offending
line, marker, and Cargo summary in 297 stderr bytes / 7 lines; its full log is retained in the raw
measurement directory and the probe source was removed. The malformed program preserved
`LKJ-SRC-UNMATCHED-MARKER`, logical path, primary range, message, and related range; machine mode
returned one 445-byte JSON value on stdout, empty stderr, and status 1. Successful JSON check is one
43-byte value. These are byte/line reductions, not provider tokenization or billing measurements.

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
committed, so this baseline is **pre-reset orientation only**. The representative selected-product
baseline later in this document supersedes it.

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

## Source-free semantic-workspace vertical

**Structural and stack-safety evidence, not a latency benchmark.** The hypothesis was that honest
source-free nominal declarations, generic function declarations, immutable and mutable lexical
locals, ordered imperative control including early return, byte-vector ownership, exhaustive enum
payload matches, and exact generic calls could converge with imported semantics and compile directly
without hidden source/HIR authority, source loading, parsing, repeated enum-declaration/CFG scans, or
recursion on expression, pattern, local, match, or public type depth. The equivalent workloads were
the imported and transaction-created forms of:

- `identity(value: i64) -> i64` plus `main() -> identity(42)`;
- a two-field product constructed, bound, and projected;
- a two-variant enum constructed, bound, and tested;
- a two-variant enum constructed and exhaustively matched, with one stable payload binding;
- a byte vector thawed from bytes, shared-borrowed for a call, then moved and observed;
- a mutable `i64` counted from 0 to 100 by ordered sequence, assignment, `while`, and canonical
  `less-than`;
- the retained ownership-control program, which allocates and borrows one byte vector before an early
  return of `7`; and
- imported and source-free generic identity declarations and calls, including exact ordered binders,
  builtin bounds, nested binder-bearing types, auto witnesses, and exact structured type arguments;
  and imported repeated-bound and explicit-implementation calls.

Selection required equal normalized stable entity kinds/types, containment, references,
dependencies, node kinds/types/effects, canonical match-plan shape, compiler outcomes, selected
memory-obligation kinds, the main bytecode stream, exact VM values or traps, and cleanup behavior.
Product/enum names are normalized only in the test observation because imported names retain module
qualification; nominal identity
stays workspace-local and structured. Match provenance is intentionally not equal: imported plans
retain real source origin and source-free plans retain semantic origin.

Retained deterministic counters, indexes, and assertions show:

- selected source-free create/fill/compile paths invoke source loading and the parser zero times;
- incomplete compilation visits zero memory-plan, SSA, and bytecode phases;
- scalar, product, enum, immutable/mutable local, counted-loop, ownership, early-return,
  exhaustive enum-payload-match, and equivalent generic-call source-free/imported paths have equal
  selected semantic observations and production outcomes; the counted-loop path returns exactly 100
  with zero source-load/parser invocations. Imported and source-free ownership-control snapshots have
  equal normalized node/type/effect/containment/reference/dependency facts, memory-obligation kinds,
  main bytecode, and VM result `7`; the source-free create/fill/compile portion performs zero source
  loads and parses and contains exactly one whole-value drop obligation;
- generic declaration and call edits invoke source loading and parsing zero times; source-free
  declarations allocate stable binder entities in declaration order, publish exact bounds and nested
  binder types, derive the same auto witnesses as source import, expose exact instantiated
  parameters/results/effects, and survive rename, deletion, recreation, and unrelated
  function/product/implementation compaction without changing surviving public binder or witness
  identity;
- index construction performs exactly one root-address lookup per semantic node for retained nested
  `if` geometries at depths 32, 64, and 128 and for the nested-match fixture; enum, variant,
  enum-field, and match relation lookup uses prebuilt identity maps rather than scanning declarations
  per expression; the nested-match fixture also records exactly three pattern-lowering node visits
  per two-arm match (one `Some` aggregate, its wildcard field, and one `None` aggregate); a generated
  1,028-node wide sequence with 512 assignments records exactly one lowering-order visit and one
  scope visit per draft node and compiles/executes on a 128 KiB worker stack; separate fresh 1-use
  and 2,001-use stable-target drafts record the same one-time callable location-scan count, while
  exact map lookups grow from 2 to 2,001 rather than rescanning the retained body per use;
- the retained same-sequence movement fixture permutes 2,048 direct scalar child blocks on a 128 KiB
  worker stack and records exactly 2,048 examined blocks and 2,049 explicitly relocated nodes,
  including the sequence. The ordinary full path records one semantic clone, compaction, effect
  inference, index build, and reconciliation; every public node identity survives and the final
  program compiles and executes. The ignored 20,000-level moved-subtree fixture exercises the same
  iterative relocation and destruction path. These exact work counts establish linear selected-root
  traversal shape and stack safety, not a latency or broad throughput claim;
- callable/local deletion uses one retained-order binding/plan pass, one iterative expression/pattern
  rewrite per surviving root, and one parent/child-ordinal survivor reconciliation pass; it does not
  rescan the whole program for each binding and retains no dead binding or plan per edit. Focused
  tests assert dense binding and retained-plan placement, compact per-root slots, and declaration-
  ordered dense places after removal or insertion before a survivor. The existing modest small-stack
  local/match tests execute the same compaction path on every staged transaction;
- code structure and exact semantic assertions, rather than a latency claim or nominal-specific work
  counter, show that nominal deletion builds each concrete product, enum-vector, and implementation
  relocation map once. Final dependency validation builds one borrowed product-name lookup and
  iteratively visits surviving stored types, expressions, and patterns; it does not rescan declarations
  once per requested deletion or products once per type. Focused tests delete two earlier products and
  enums, assert exact dense retained placement, preserve public entity/node and stable nominal/member/
  layout identities, compile and execute surviving product and enum operations, and compare
  order-reversed dependency-closed diffs. Imported fixtures exercise product patterns and a
  two-product/two-implementation program: they compact surviving `ProductId`/`ImplId` values, remap an
  explicit generic witness, execute unchanged, and invoke source loading and parsing zero times after
  import;
- the selected product path runs both imported and source-free ownership-control forms through one
  baseline-native entry with no VM execution; each allocates and drops exactly one unique owner,
  returns `7`, reports no cleanup failure, and ends with zero live owners, live loans, release backlog,
  stale/forged failures, or teardown failures. Imported and source-free typed loops likewise take the
  baseline-native path and return `42`. Their affine-continue counterparts each allocate and drop
  exactly three iteration-local owners, return `3`, and end with no live owner, loan, release backlog,
  or cleanup failure. Compiler convergence tests additionally compare normalized loop/control HIR,
  private nearest targets, node types/effects, memory obligations, bytecode, evaluator/VM results,
  parser/load counters, and cleanup behavior;
- targeted return-value-child and ordinary-value-to-return replacement preserves targeted and
  unaffected ancestor IDs, iteratively changes enclosing sequence, mutable-local, `let`, and
  all-return match results to `never`, refreshes the existing match plan once for the affected root,
  keeps old snapshots executable, and rejects a non-final return without publication. Hole
  introduction tombstones the removed return value, and refill preserves the hole/root identity.
  Foreign, stale, and same-transaction-deleted callees nested beneath return reject without
  publication or changed deterministic retry allocation. An unrelated outstanding function hole
  does not permit publication of an unreachable expression after an inserted return. A source-free
  function also moves one affine byte vector through return to its caller, where a borrow observes
  the value before canonical caller cleanup. Return edits in programs without match plans add no
  separate match scan;
- an affine mutable-local regression rejects live overwrite atomically, then moves and releases the
  old byte vector, reinitializes the ended place, moves and releases the replacement, returns `4`,
  and completes VM teardown without retained owners or loans;
- deterministic instrumentation on a 128 KiB stack shows both valid 256-level typed-loop lowering and
  an invalid 512-level out-of-context control draft perform exactly one validation and one lowering
  visit per flat draft node. The invalid edit republishes neither snapshot nor allocator state;
- one ignored locked-release fixture completes a 20,000-level nested-`if` draft beneath one early
  return (60,002 expression nodes), a second completes 20,000 alternating immutable/mutable lexical
  locals (40,001 expression nodes), a third completes 20,000 nested semantic enum matches (80,001
  expression nodes and 20,000 canonical plans), and a fourth completes 20,000 nested typed loops
  (60,000 expression nodes); each includes staged lowering, semantic clone, dense lifecycle
  compaction, identity reconciliation, complete-HIR/ownership/match derivation, memory planning, SSA,
  bytecode, VM execution, projection where selected, deepest-branch hole introduction and
  legal-constructor control-context query in the nested-`if` and smaller deep-loop fixtures, and
  destruction on a 128 KiB worker stack; separate type-only fixtures exercise
  published `SemanticType` construction, clone, equality, hashing, display, transaction validation,
  conversion, query, projection, and destruction and creation-only `DeclarationType` construction,
  clone, equality, debug, local-binder resolution, stable publication, signature query, projection,
  and destruction at 20,000 levels on that stack without duplicating the full pipeline geometry;
- bytecode emission derives nonowned structural values once per function by propagating predecessor
  edges, replacing a whole-CFG scan for every emitted structural load/store; the generated match
  fixture asserts exactly one collection per SSA function and one deterministic visit per CFG edge;
  and
- the small scalar test asserts one post-completeness lowering invocation, so incomplete revisions
  enter none and the selected complete revision enters the production compiler boundary once.

Reproduce the focused convergence and locked-release stack fixtures with:

```sh
cargo test --locked -p lkjscript-compiler \
  workspace::movement_tests::deep_moved_subtree_and_wide_sibling_permutation_are_stack_safe_and_identity_stable -- --exact
cargo test --locked -p lkjscript-app \
  engine::tests::moved_independent_owners_execute_and_clean_up_in_baseline_native -- --exact
cargo test --locked --release -p lkjscript-compiler \
  workspace::movement_tests::twenty_thousand_level_moved_subtree_is_stack_safe \
  -- --ignored --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::source_free_main_return_is_queryable_and_executes_without_source_work -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::invalid_return_drafts_are_structured_atomic_and_retry_stable -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::unreachable_return_replacement_rejects_while_an_unrelated_hole_remains -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::source_free_return_moves_an_affine_value_to_its_caller -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::return_replacement_recomputes_control_ancestor_types -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::return_call_references_reject_foreign_stale_and_deleted_entities_atomically -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::return_replacement_refreshes_match_arm_and_result_types -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::source_free_early_return_executes_cleans_up_and_matches_imported_semantics -- --exact
cargo test --locked -p lkjscript-app \
  engine::tests::source_free_early_return_enters_native_and_cleans_unique_owner_once -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::source_free_imperative_counted_loop_executes_and_matches_imported_semantics -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::imported_and_source_free_typed_loop_and_nested_continue_converge -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::affine_break_payload_and_continue_cleanup_match_imported_ownership -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::legal_loop_control_constructors_are_exact_contextual_and_paginated -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::deep_flat_loop_control_is_linear_and_stack_safe -- --exact
cargo test --locked -p lkjscript-app \
  engine::tests::imported_and_source_free_typed_loops_use_the_same_product_path -- --exact
cargo test --locked -p lkjscript-app \
  engine::tests::imported_and_source_free_continue_cleanup_match_in_the_product_path -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::wide_imperative_draft_visits_each_node_once_on_a_small_stack -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::wide_stable_assignment_targets_use_one_callable_location_scan -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::affine_mutable_reinitialization_moves_and_cleans_up_exactly_once -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::imported_nominal_local_and_ownership_programs_converge -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::source_free_byte_vector_borrow_then_move_executes_and_cleans_up -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::imported_and_source_free_enum_payload_matches_converge -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::product_deletion_cascades_fields_compacts_dense_ids_and_preserves_survivors -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::enum_deletion_cascades_members_and_preserves_stable_nominal_layout_identity -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::imported_product_deletion_cascades_implementation_and_remaps_surviving_witness -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::imported_product_pattern_and_value_survive_earlier_product_compaction -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::source_free_generic_function_creation_is_exact_and_executes_without_source_work -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::imported_and_source_free_generic_declarations_converge_through_bytecode_and_vm -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::source_free_generic_declaration_uses_imported_trait_and_explicit_implementation -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::malformed_generic_declarations_are_structured_atomic_and_retry_stable -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::source_free_generic_binders_survive_rename_compaction_and_follow_deletion -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::imported_generic_signature_and_explicit_workspace_call_are_exact_and_execute -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::generic_binder_identities_survive_compaction_and_follow_function_lifecycle -- --exact
cargo test --locked -p lkjscript-compiler \
  workspace::tests::unsatisfied_bound_reports_the_exact_binder_when_traits_repeat -- --exact
cargo test --locked --release -p lkjscript-compiler \
  workspace::tests::twenty_thousand_level_generic_declaration_types_are_stack_safe \
  -- --ignored --exact
cargo test --locked --release -p lkjscript-compiler \
  workspace::tests::twenty_thousand_level_semantic_type_operations_are_stack_safe \
  -- --ignored --exact
cargo test --locked --release -p lkjscript-compiler \
  workspace::tests::twenty_thousand_level_source_free_compile_execute_and_drop_on_small_stack \
  -- --ignored --exact
cargo test --locked --release -p lkjscript-compiler \
  workspace::tests::twenty_thousand_source_free_locals_compile_execute_and_drop_on_small_stack \
  -- --ignored --exact
cargo test --locked --release -p lkjscript-compiler \
  workspace::tests::twenty_thousand_level_source_free_enum_matches_compile_execute_and_drop_on_small_stack \
  -- --ignored --exact --test-threads=1
cargo test --locked --release -p lkjscript-compiler \
  workspace::tests::twenty_thousand_level_source_free_loop_control_is_stack_safe \
  -- --ignored --exact
```

On the development host, the imperative-control cutover's locked-release 20,000-local invocation,
now alternating immutable and mutable declarations and including removal/recompaction of every local
on a 128 KiB stack, reported 1.18 seconds for the test body after a 1 minute 21 second release
rebuild. The final return-extended 60,002-node nested-`if` invocation reported 19.78 seconds with
warm release artifacts. The binding-lifecycle cutover's final locked-release 20,000-match invocation
reported 307.41 seconds for the test body with warm release artifacts (307.469 seconds process wall);
it is deliberately an ignored maximum-geometry correctness fixture, not a representative latency
target. The final 20,000-level typed-loop fixture reported 10.84 seconds with warm release artifacts;
it remains stack-safety and semantic-convergence evidence rather than a representative latency
workload. These single noisy samples are orientation, not product-latency claims or gates. No allocator count,
peak RSS, retained memory, edit/query latency distribution, or representative application throughput
was
measured. Reverse or redesign the implementation if equivalent observations diverge, an incomplete
path enters a compiler phase, lookup work ceases to track semantic work, selected depth fails on the
small stack, structural-local emission resumes repeated CFG scans, cleanup changes, or broader
authorship requires two mutable semantic representations. Future measurements should extend these
retained product operations rather than revive the deleted text publication or wire service.

## Owned structural-value stack and work correction

**Measured generated-boundary correction, not representative application performance.** The
hypothesis was that the safe owned `SemanticValue` tree could not contain cycles, so
`OwnedValue::from_structural` spent quadratic work scanning every active ancestor, while derived
Debug/equality and recursive symbol rewriting still consumed native stack. Equivalent before/after
semantics were one generated alternating product/enum chain whose base product contains unit, Bool,
I64, F64-bits, string, path, bytes, byte-vector, and static-symbol leaves. Focused correctness tests
check exact node/field/byte metrics and leaf contents; timing samples use the same generator and one
named operation. No depth or work limit was added.

The retained fixture is
[`owned_structural_depth.rs`](../crates/lkjscript-core/tests/owned_structural_depth.rs). Internal
phase timing excludes fixture construction and result destruction except for the named construction
and destruction phases. Each sample used a locked release profile, shared Cargo caches/artifacts, a
fresh test process, and a 128 KiB worker stack. The first invocation in a matrix could compile the
test binary, but compilation occurs before the internal phase clock. Five validation samples and
three samples for each other phase were collected at depths 1,024, 2,048, 4,096, 8,192, and 20,000.
Nearest-rank p95 is the maximum and
is orientation only. Reproduce a sample with:

```sh
LKJSCRIPT_STRUCTURAL_DEPTH=20000 \
LKJSCRIPT_STRUCTURAL_OPERATION=validation \
cargo test --locked --release -p lkjscript-core --test owned_structural_depth \
  owned_structural_scale_sample -- --ignored --exact --nocapture --test-threads=1
```

The before samples used detached product code at `f7707560fd036924555d945010d35aeea365b327`
plus the same validation operation copied from the then-untracked sampler. The after tracked Rust
diff against that commit had SHA-256
`a1289ca27a6c11aa56c9380678c28025e21b885c1ce3b0c5289b9c6259f16405`; the retained core sampler
and VM fixture had SHA-256 values
`7cbbf36946488049811d5620fd99597c2234d32bb718871ff36b222ed7335c38` and
`f0280247bcf9912c3392aa3c7958595a215f9539a7189c5c8062b3afd485b883`. Documentation and the
user-supplied `AGENTS.md` replacement are excluded from those values. Raw output remains ignored
under `target/structural-scale/`. Environment: `devbox`, Linux `7.0.0-27-generic` x86-64, AMD Ryzen
9 9955HX, 20 available logical CPUs, 32 GiB RAM, `rustc 1.96.0 (ac68faa20 2026-05-25)`, and Cargo
1.96.0.

| Depth | Validation before median (p95), us | Validation after median (p95), us |
| ---: | ---: | ---: |
| 1,024 | 148.509 (245.261) | 4.829 (5.400) |
| 2,048 | 905.272 (981.475) | 7.444 (8.636) |
| 4,096 | 2,667.966 (2,812.047) | 13.736 (14.207) |
| 8,192 | 8,518.837 (10,017.445) | 24.877 (34.575) |
| 20,000 | 44,921.690 (49,781.487) | 56.817 (62.237) |

For 19.53x depth, the before median grows 302.48x and the after median 11.77x. At depth 20,000 the
median falls 790.64x. More importantly, the retained structural assertion is exact: depth 20,000
has 20,010 nodes, 20,009 fields, 25 aggregate leaf bytes, and 40,044 units of validation work;
node, field, and byte charges are each visited once. That structural result, not microsecond timing,
is the linear-work correctness evidence.

Post-correction endpoint medians show the other retained tree operations have the expected bounded
or near-linear shape. Validation uses five samples; the other rows use three.

| Operation | Depth 1,024 median, us | Depth 20,000 median, us | Growth |
| --- | ---: | ---: | ---: |
| iterative construction | 27.832 | 752.775 | 27.05x |
| owned validation/conversion | 4.829 | 56.817 | 11.77x |
| runtime image publish/export | 99.537 | 1,855.219 | 18.64x |
| clone | 50.685 | 896.556 | 17.69x |
| fallible equality | 3.207 | 67.908 | 21.17x |
| trait equality | 3.667 | 69.320 | 18.90x |
| symbol canonicalization | 7.905 | 97.583 | 12.34x |
| destruction | 14.307 | 257.835 | 18.02x |
| bounded Debug | 2.074 | 2.384 | 1.15x |

Before the correction, Debug completed at depth 128 with 25,187 output bytes but overflowed the 128
KiB stack at depth 256; trait equality completed at 2,048 and overflowed at 4,096; symbol rewriting
completed at 1,024 and overflowed at 2,048. Afterward every sampled operation completed at depth
20,000 on the same stack, and Debug emitted 174 bytes at every measured depth. A separate generated
VM fixture returns a 20,000-level alternating result; baseline native reports a typed recursive
call-graph stack decline before entry, and the VM constructs, exports, clones, compares, and destroys
the result once.

The correction removes the impossible tree ancestry/cycle state, gives semantic payload/value/
children bounded Debug and one shared iterative equality algorithm, makes symbol rewriting
iterative, and makes the SSA/VM result oracle compare full owned values rather than bounded Debug
text. Keep it while `SemanticValue` remains an exclusively owned safe tree. If a future unsafe,
serialized, shared, or graph producer is introduced, validate cycles at that real boundary or use
`SemanticDagSnapshot`; do not restore a depth-linear ancestry scan inside the owned tree. Reverse
trait or traversal changes for an equality, allocation-failure, cleanup, or malformed-value
semantic regression, not for compatibility with recursive debug output.

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
runtime baseline. Product metrics identify `execution_path=baseline-native|vm-fallback`, one
nullable typed decline, whether native entry began, package/compiler and native/VM timings,
published installed-artifact measurements, and selected explicitly saturating runtime observations.
Unavailable native sections are `null`. Threshold, automatic-transition, public engine, and tier
fields are absent. Reverse the
choice if equivalent representative measurements show the group preflight or baseline maintenance
cost outweighs its scalar benefit.

### Representative selected-product baseline and package correction

**Measured on one host; not a cross-machine performance promise.** The hypothesis was that
`lkjscript run` verified the complete package once in the application and then rebuilt and verified
it again in the compiler importer. If true, removing only the application pass should eliminate
about one package-validation duration outside the compiler timer while preserving the same validated
program, selected execution path, result, host effects, and failure behavior. The correction is one
required-package compiler entry point: it verifies the root manifest, lock, selected module, source
identities, target, and typed capability grants once and carries that value through import and
`compile_snapshot`. It rejects missing packages and ungranted bytecode capabilities; it does not
weaken package validation.

[`meta/scripts/runtime-baseline.py`](../meta/scripts/runtime-baseline.py) retains the product harness.
It accepts only the exact retained `lkjscript.metrics` schema and contract identity before reading
measurements. It covers scalar loop/branch, direct calls, redundant scalar work, products/lists,
enum matching,
unique ownership and cleanup, an entered checked failure, larger benchmark and Mandelbrot programs,
a nested polymorphic package, bytes/hash, two filesystem paths, and direct SQLite. Every sample
checks exit status, exact outcome, stdout SHA-256, expected path/native-entry state, and applicable
file or SQLite effects; the current ownership sample also checks exact allocation/drop counts and
zero live owners, loans, release backlog, and teardown failures. Host-effect paths are arguments
beneath one harness-owned mode-0700 temporary directory, never predictable shared `/tmp` files. One
validated fresh-process warmup precedes five measured fresh processes per workload. The command is:

```sh
meta/scripts/runtime-baseline.py --label single-package-current-5 --samples 5 --warmups 1
```

The paired retained binary used `--no-build` with explicit commit, worktree, profile, and build-command
attestations. Both records have identical workload-input hashes. Environment: `devbox`, Linux
`7.0.0-27-generic` x86-64, AMD Ryzen 9 9955HX, 20 available of 32 configured logical CPUs, 32 GiB
RAM, and `rustc 1.96.0 (ac68faa20 2026-05-25)`. The workspace release profile uses LTO, one codegen
unit, and symbol stripping. The harness removes Cargo/Rust profile, wrapper, target, incremental,
and flag overrides and records the resolved tool/cache locations; none of those overrides was set
for the final runs. Nearest-rank p95 is the maximum of five and is orientation only.

The RSS column is 1 ms `/proc` process-tree polling. It may miss short-lived or final peaks, may
report zero for a very short run, and may double-count shared pages; it is not unique physical
memory. Total allocation count and bytes were unavailable. Raw JSON remains ignored at
`target/runtime-baseline/`; current and before-record SHA-256 values are respectively
`14bd924e2e55e542fc988bc3105ac3d781c3c6655c5ea850c3291e53d5099829` and
`259a8a3713928b7bc1e2c5364e06d1faffc0994738373201d804de1390c9a060`.

| Workload | Path | Before process median (p95) | After process median (p95) | Median change | Approx. RSS before / after |
| --- | --- | ---: | ---: | ---: | ---: |
| scalar loop | native | 223.891 ms (230.342) | 117.154 ms (119.386) | -47.7% | 28.5 / 28.7 MiB |
| scalar calls | native | 228.297 ms (231.909) | 118.912 ms (123.996) | -47.9% | 28.6 / 28.7 MiB |
| scalar redundancy | native | 232.161 ms (245.890) | 120.109 ms (121.371) | -48.3% | 28.6 / 28.7 MiB |
| product/list | native | 225.791 ms (233.253) | 115.250 ms (122.912) | -49.0% | 28.1 / 28.0 MiB |
| enum match | native | 225.253 ms (232.096) | 115.840 ms (120.333) | -48.6% | 28.1 / 28.0 MiB |
| ownership/control | native | 226.031 ms (234.901) | 118.470 ms (121.404) | -47.6% | 28.6 / 28.0 MiB |
| checked failure | native | 228.876 ms (233.960) | 115.233 ms (120.347) | -49.7% | 28.4 / 28.0 MiB |
| hello | VM | 227.112 ms (230.096) | 117.346 ms (118.467) | -48.3% | 27.9 / 27.9 MiB |
| bench | VM | 1,031.528 ms (1,042.922) | 927.697 ms (932.748) | -10.1% | 28.7 / 28.9 MiB |
| Mandelbrot | VM | 281.486 ms (284.133) | 179.831 ms (183.688) | -36.1% | 28.7 / 28.9 MiB |
| polymorphic transport | VM | 27.241 ms (28.481) | 15.334 ms (15.502) | -43.7% | 7.9 / 7.6 MiB |
| bytes/hash | VM | 224.523 ms (227.179) | 116.693 ms (118.585) | -48.0% | 28.0 / 28.0 MiB |
| bulk bytes/filesystem | VM | 229.357 ms (233.664) | 117.863 ms (124.644) | -48.6% | 28.1 / 28.2 MiB |
| durable filesystem | VM | 234.965 ms (244.029) | 125.469 ms (130.196) | -46.6% | 28.7 / 28.9 MiB |
| SQLite | VM | 250.768 ms (260.286) | 141.029 ms (145.785) | -43.8% | 29.4 / 29.6 MiB |

The compile medians remain in the same range: for example, scalar loop changed from 108.776 ms to
113.010 ms and bench from 112.481 ms to 113.422 ms. The new package-validation timer owns 111.627 ms
and 111.903 ms of those respective medians. In contrast, median process time outside the reported
compiler and execution intervals fell from 114.633 ms to 2.781 ms for scalar loop and from
115.966 ms to 3.692 ms for bench. Across root-package workloads, this outside interval fell from
112.956-118.916 ms to 2.353-3.692 ms; for the smaller nested package it fell from 14.257 ms to
1.934 ms. That work-shape result, rather than one timing cell alone, supports deletion of the
duplicate pass. The execution-heavy bench improves only 10.1%, as expected; approximate RSS is
effectively unchanged.

Current metrics classify native fallback without a broad duplicate label. Hello, bench, bytes/hash,
and the host workloads decline at lowering with `unsupported-type`; Mandelbrot declines with
`unsupported-operation` for `WriteByte`; the nested polymorphic package declines with
`unsupported-signature`. All declines occur before entry and execute the validated VM program once.
The checked-failure workload records `baseline-native` and native entry before the expected checked
division trap. Engine tests independently prove entered failure does not retry and a generated
65-function group crossing the current private 64-function native tuning boundary declines with
`backend-verification`, executes the VM exactly once, and returns `42`.

Published installed-artifact observations for the seven native workloads are:

| Workload | Functions | Code bytes | Mapped bytes | Median first native entry | Peak native stack bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| scalar loop | 2 | 3,240 | 4,096 | 0.432 ms | 864 |
| scalar calls | 2 | 3,143 | 4,096 | 3.916 ms | 832 |
| scalar redundancy | 3 | 6,178 | 8,192 | 2.172 ms | 1,424 |
| product/list | 5 | 4,609 | 8,192 | 0.241 ms | 528 |
| enum match | 1 | 3,029 | 4,096 | 0.285 ms | 688 |
| ownership/control | 1 | 1,383 | 4,096 | 0.188 ms | 272 |
| checked failure | 1 | 731 | 4,096 | 0.168 ms | 112 |

Each row has one published object. Declined attempts report artifact data as unavailable rather than
claiming generated work was zero. Runtime event counters are explicitly saturating. These selected
service observations are not total allocator measurements.

Three fresh-target locked release builds measured by
[`meta/scripts/build-footprint.py`](../meta/scripts/build-footprint.py) had a 57.330 s median and
57.388 s p95; immediate no-op rebuilds had a 24.182 ms median and 24.568 ms p95. All three produced
the same 6,324,552-byte binary with SHA-256
`e52ca11faa3a6796d526f17a89f161a65e6dbe7340fcef27d85c59cb6e7e01d8`. That digest also matches the
runtime binary. The retained before binary was 6,304,576 bytes, so the final instrumentation and
correction add 19,976 bytes (0.317%). No paired fresh-build before sample was retained, so no
cold-build improvement claim follows. Raw build JSON remains ignored with SHA-256
`3e5d9debb40f01d0be867131a3202ac2643284f22669a7d028b06934b5836868`.

Keep the single-pass package path while it remains fail-closed and removes one whole validation pass.
Reverse it for a manifest, lock, source-identity, target, capability, mutation, or publication
correctness regression, or if repeated equivalent product profiles no longer show eliminated work
and its API cost outweighs the benefit. Do not restore application-side preverification merely for
compatibility. The still-required package verification now dominates small-run compile/startup time;
profile that pass before attempting another runtime optimization. First user-visible output timing,
total allocations/bytes, semantic edit/query latency, retained warm-process memory, other targets,
and broader application throughput remain unmeasured.
