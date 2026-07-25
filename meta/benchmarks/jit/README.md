# Retained JIT Benchmarks

## Purpose

Retain decision-grade Linux x86-64 evidence for callable baseline JIT and the
forced first proof-based optimizing JIT without changing engine policy.

## Status

**Current** standard-library-only benchmark harness. The optimizing protocol is
a forced-tier performance gate only. Automatic optimizing promotion remains
disabled and is neither exercised nor measured by this protocol.

## Optimizing Workload And Oracle

The primary workload is
[`src/examples/jit-optimizing/main.lkjscript`](../../../src/examples/jit-optimizing/main.lkjscript).
It makes 10,000 calls containing deliberately duplicated checked I64 division.
A separate reference-VM process is the independent engine oracle and must return
exact I64 `3333`. Forced baseline and forced optimizing samples must return the
same exact value. Normal executions must have empty stdout and stderr; timed
executions opt into exactly one low-overhead `LKJSCRIPT_METRICS <json>` stderr
line.

The retained comparison also runs same-commit forced baseline on
[`src/examples/jit-scalar/main.lkjscript`](../../../src/examples/jit-scalar/main.lkjscript).
Its exact F64 recurrence is independently evaluated by Python. Both native and
process-wall medians are compared with the retained callable-baseline result,
with a 5% regression ceiling. This historical comparison is a sentinel rather
than an algorithm-equivalent optimizer comparison: compiler, metrics, native
ABI, stack checks, and workload-adjacent code have evolved since the retained
callable-baseline commit.

One forced optimizing execution of the allocation-graph fixture is retained as
a correctness/accounting check. It must return exact I64 `1`, enter optimizing
code with no baseline entry or VM fallback, and report successful heap calls,
allocations, roots, and W^X objects. It is not a timed workload.

## Optimizing Protocol

Run from the repository root:

```sh
python3 meta/benchmarks/jit/benchmark.py
```

The harness performs `cargo build --locked --workspace --release`, rejects
non-Linux/non-x86-64 hosts, and records the commit and dirty paths; OS/kernel,
CPU, RAM, Python, rustc, cargo, and git versions; binary and source sizes and
SHA-256 hashes; and the exact commands. The default deterministic seed is
`0x4c4b4a534f505449`.

After normal-stream and oracle checks, all three measured cases—optimizing
workload forced baseline, optimizing workload forced optimizing, and scalar
workload forced baseline—receive four warmups and 31 measured samples in one
deterministically randomized interleaving. Lower counts are rejected and no
sample is discarded. Process wall uses `time.monotonic_ns` around process
creation through collection. `/proc/<pid>/status` `VmRSS` is polled about every
0.5 ms and the largest observation is retained.

Every warmup, sample, and order entry is retained, including the complete
runtime metrics record. Summaries retain median, MAD, nearest-rank p95, minimum,
and maximum for process wall, peak RSS, execution/optimization/lowering/install
times, tier entries/objects, fallback, proof records/bytes/work/pass counts,
rewrite counts, and code/metadata/cache bytes. Exact per-sample tier, function,
object, proof, code-byte, and W^X invariants are checked before a result is
written.

The result schema is `lkjscript.optimizing-jit-benchmark.v1` and the retained
file is
[`results/optimizing-jit-linux-x86_64.json`](results/optimizing-jit-linux-x86_64.json).
The verdict is mechanically **Adopted** only when all of these predeclared gates
pass:

- optimizing native-execution median is at least 1.20x faster than same-commit
  baseline on the optimizing workload;
- the median improvement is greater than twice the combined MAD (combined MAD
  is the sum of the two MADs);
- all exact outcomes and stream checks pass;
- every optimizing sample has nonzero optimizing entries, zero baseline entries,
  zero VM fallback, verified W^X, and nonzero checked proof records;
- every forced baseline sample has nonzero baseline entries, zero optimizing
  entries, zero VM fallback, and verified W^X;
- current scalar forced-baseline native and process medians are each no more
  than 5% above the retained callable-baseline medians.

A failed gate produces a retained **Rejected** verdict; it does not permit an
optimizing performance claim or automatic promotion.

## Retained Result At `063668e`

The locked release run on the recorded AMD Ryzen 9 9955HX host retained all
four warmups and 31 samples per case. Same-commit optimizing-workload native
medians were 1.997375 ms baseline (MAD 0.016721 ms) and 0.681521 ms optimizing
(MAD 0.003567 ms), a 2.930761x speedup. The 1.315854 ms improvement exceeded
twice the combined MAD, 0.040576 ms. Process medians were 3.584969 and 2.457400
ms respectively. Optimizing retained 72 checked-I64 GVN proof records, 2,816
estimated certificate bytes, 2,724 code bytes, 10,001 optimizing entries, zero
baseline entries/fallback, and verified W^X; baseline retained 13,956 code bytes
and 10,001 baseline entries.

The overall verdict is **Rejected** because the historical scalar sentinel's
native median was 8.182742 ms versus retained 7.647935 ms, ratio 1.069928 and
therefore a 6.99% regression above the 5% ceiling. Its process median was
9.340049 ms versus 9.372036 ms, ratio 0.996587, so that separate gate passed.
This is evidence of a sentinel regression, not attribution to the optimizer:
the source remains identical but compiler, metrics, ABI, stack checks, binary,
and surrounding generated code evolved between commits. The forced optimizer's
workload-local speed/noise/correctness gates passed, but the predeclared
all-gates adoption rule rejects the performance claim and leaves automatic
promotion disabled and unmeasured.

The one allocation-graph check returned I64 `1` with 3 optimizing entries, 7
allocations, 6 collections, 14 attempted/14 successful heap calls, maximum 3
roots, 6 barriers, zero baseline entry/fallback, and verified W^X. The retained
JSON SHA-256 is
`3e4341ffab5c0cbd976b3dc228d24dfdd8ff135247b91caafb74f0a571e71cec`.

## Historical Callable Baseline

The earlier VM/forced-baseline/auto protocol and all randomized samples remain
retained in
[`results/callable-baseline-jit-linux-x86_64.json`](results/callable-baseline-jit-linux-x86_64.json),
[`results/auto-threshold-1.json`](results/auto-threshold-1.json), and
[`results/auto-threshold-1024.json`](results/auto-threshold-1024.json). The
compatible pre-JIT VM comparison remains in
[`results/pre-jit-c4-vm-comparison.json`](results/pre-jit-c4-vm-comparison.json).

The release runtime emits metrics only when `LKJSCRIPT_METRICS` is present.
`LKJSCRIPT_METRICS_FILE` writes the same line to a file. Both are separate from
verbose `LKJSCRIPT_JIT_DIAGNOSTICS`; ordinary execution remains silent.
