# Callable Baseline-JIT Benchmark

## Purpose

Retain the decision-grade Linux x86-64 comparison of the same release binary's
reference VM, forced baseline JIT including compilation, internal generated-code
execution after installation, and automatic function-entry tiering.

## Status

**Current** standard-library-only harness for the allocation-free scalar
baseline subset. The committed result is linked from the experiment registry.
It is not an OSR benchmark: `main` has one long invocation, while its repeated
`scalar-step` calls may tier under `auto`.

## Workload And Oracle

The default workload is
[`src/examples/jit-scalar/main.lkjscript`](../../../src/examples/jit-scalar/main.lkjscript):
100,000 F64 loop iterations with 100,000 direct calls. It writes no program
output and returns the computed F64 value. The harness independently evaluates
the same IEEE-F64 recurrence in Python and requires exact returned bits from
every VM, forced, and auto process. Forced samples additionally require native
entry, zero VM fallback, and zero compilation failures. Auto samples require a
successful later-call native transfer. Any process output, stderr other than
one metrics record, nonzero exit, malformed metric, or oracle mismatch aborts
the run.

## Protocol

Build and verify before timing, then run:

```sh
cargo build --workspace --release --locked
python3 meta/benchmarks/jit/benchmark.py
```

Defaults are four randomized warmups and 31 randomized measured samples per
variant using recorded seed `0x4c4b4a534d455452`. Lower counts are rejected.
Each process is timed with `time.monotonic_ns`; `/proc/<pid>/status` is polled
for `VmRSS`. Every warmup and measured sample, order, exact internal phase
metrics, code-object/accounting statistics, exit status, environment, hashes,
and median/MAD/nearest-rank-p95/min/max summaries are retained in
[`results/callable-baseline-jit-linux-x86_64.json`](results/callable-baseline-jit-linux-x86_64.json).
No sample is discarded.

The release runtime emits metrics only when `LKJSCRIPT_METRICS` is present, as
one `LKJSCRIPT_METRICS <json>` stderr line. `LKJSCRIPT_METRICS_FILE` instead
writes that line to an explicit file. Both are separate from verbose
`LKJSCRIPT_JIT_DIAGNOSTICS`; ordinary execution remains silent.
