# Performance evidence

No performance leadership claim is made. These are bootstrap baselines whose purpose is to expose
costs before optimization.

## Environment

- date: 2026-08-14;
- code state: reset working tree based on `1f4da233367e0cd282c1e5e1c35b6f73a19880ad` (the first
  retained post-reset commit is recorded in the next evidence update);
- host: `devbox`, AMD Ryzen 9 9955HX, 20 logical CPUs visible, 32 GiB memory;
- OS: Linux 7.0.0-29-generic x86-64, glibc 2.39;
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`;
- mode: release for runtime measurements;
- workload: one 10-node source-free workspace whose entry computes `40 + 2`;
- oracle: typed result must equal `i64(42)` and artifact round-trip must be byte-identical.

## Build and test baseline

A fresh target directory avoided deleting or reusing repository build state:

```sh
fresh_target=$(mktemp -d /tmp/lkjscript-fresh-target.XXXXXX)
TIMEFORMAT='fresh_release_build_elapsed_s=%3R'
time CARGO_TARGET_DIR="$fresh_target" cargo build --workspace --release --locked
```

Observed elapsed time: 6.422 s. The fresh release directory occupied 7.9 MiB.

A separate fresh target ran the complete test boundary:

```sh
fresh_target=$(mktemp -d /tmp/lkjscript-fresh-test.XXXXXX)
TIMEFORMAT='fresh_full_test_elapsed_s=%3R'
time CARGO_TARGET_DIR="$fresh_target" \
  cargo test --workspace --all-targets --all-features --locked
```

Observed elapsed time: 1.934 s; 16 tests passed in that measured tree. Later correctness hardening
added tests; current verification totals belong to `docs/status.md`, and these build-time figures
were not relabelled as measurements of the later tree. `/usr/bin/time` was unavailable, so retained
maximum-RSS evidence is not yet available.

## Product-path scalar baseline

The retained ignored integration test starts the real foreground daemon binary in a new temporary
directory and sends production protocol requests. It performs one warm-up, samples summary and run
31 times, shuts down, then samples workspace restart 11 times. Percentiles use nearest-rank
ordering. Request wall time includes a new Unix-socket connection and one frame each way but not a
new client process. Internal compile/execute timings come from the daemon response and exclude IPC.

```sh
cargo test --release --test semantic_vertical product_path_performance_baseline -- \
  --ignored --nocapture --test-threads=1
```

| Measurement | Median | p95 | Samples |
| --- | ---: | ---: | ---: |
| workspace query, request wall | 7.384 us | 9.428 us | 31 |
| run, request wall | 7.053 us | 8.997 us | 31 |
| direct SPG validation/lowering/Core IR verification | 0.551 us | 0.802 us | 31 |
| interpreter execution | 0.060 us | 0.130 us | 31 |
| daemon restart with one retained workspace | 3,219.264 us | 3,294.475 us | 11 |

Single observations: initial daemon readiness 4,299.315 us; durable workspace creation 9,867.977 us;
durable bootstrap transaction 8,057.953 us. The revision-1 artifact was 501 bytes.

This workload is a bootstrap microbenchmark, not representative application evidence. The suspected
cost is synchronous full-artifact durability, not validation or execution. Reversal condition:
retain full snapshot cloning and replacement until representative edits show material latency or
memory cost; add no cache or incremental store from this scalar result alone.

## Agent-interface baseline

`bootstrap_agent_request_cost_is_bounded_and_reproducible` derives sizes from the production
encoder. The program construction is one transaction with 11 typed operations and a 253-byte frame.
The first create/construct/summary/node/run workflow uses five round trips; request frame sizes are
253 bytes for construction, 39 for summary, 64 for expanded-node lookup, and 63 for run.

The reset checkout has 34 active non-build files, 20 Rust files across one Cargo package, and the
seven maintained documents required by repository policy; the audited tree had 1,596 tracked files
and eleven packages. This file count is the navigation-fan-out baseline, not a quality claim.

Model token input/output was not measured because the daemon API has no model integration and this
milestone did not run a controlled model task. Phase 5 must measure tokens and failed edits on
controlled create, inspect, repair, and run tasks before claiming agent-cost improvement.
