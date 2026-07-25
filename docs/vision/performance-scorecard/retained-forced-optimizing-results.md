# Performance Scorecard: Retained Forced Optimizing Results

[Authority](../performance-scorecard.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Retained Forced Optimizing Results

The clean `cc967ff7e6f57a3225ae974d64ced6039ed8e9ae` locked release protocol
retained four warmups and 31 measured samples for same-commit forced baseline
and forced optimizing execution of `src/examples/jit-optimizing`, plus the same
count for the forced-baseline scalar sentinel, in one deterministic randomized
interleaving. A separate reference VM required exact returned I64 `3333`;
normal streams were silent. Every sample and order, phase metric, peak RSS poll,
tier/object/proof/code fact, and median/MAD/p95/min/max distribution is retained
in the adopted
[adopted result][adopted-result].

| Forced optimizing workload metric | Baseline | Optimizing |
| --- | ---: | ---: |
| native execution median / MAD / p95 | 1.999889 / 0.010469 / 2.092424 ms | 0.670029 / 0.002174 / 0.686310 ms |
| process wall median / MAD / p95 | 3.565363 / 0.035817 / 4.145873 ms | 2.411023 / 0.014387 / 2.609285 ms |
| polled peak RSS median / MAD / p95 | 4,148 / 40 / 4,224 KiB | 4,048 / 40 / 4,144 KiB |
| generated code / retained metadata | 13,656 / 15,953 B | 2,424 / 3,281 B |
| native entries | 10,001 baseline | 10,001 optimizing |

Optimizing native execution was 2.984780x faster. Its exact 1,329,860 ns median
improvement exceeded twice the combined native MAD, 25,286 ns. Process wall
improved from 3,565,363 to 2,411,023 ns, a 1.478776x speedup. The optimizing
case retained 72 checked-I64 GVN records, 2,816 estimated certificate bytes, 35
actually executed optimizing passes, zero baseline entries or VM fallback, and
verified W^X. Median optimization, lowering/encoding, and installation were
0.248297, 0.034676, and 0.045806 ms. The corresponding baseline lowering and
installation medians were 0.081343 and 0.057638 ms.

The mechanically complete verdict is **Adopted** for forced first-tier
performance: every exact, stream, tier, proof, W^X, allocation, speed, noise,
and scalar criterion passed. Same-commit scalar native median was 7,982,586 ns
versus retained 7,647,935 ns, ratio 1.043757; process wall was 9,207,038 versus
9,372,036 ns, ratio 0.982395. The recovery followed folding the mandatory entry
poll into ABI-2 frame registration, removing a separate runtime transition
without weakening polling or proof checks.

The earlier `063668e08b92a97a2feae8397ff0d634887bd0b6` result remains visible in
[rejected result][rejected-result].
Its optimizer-local 2.930761x result passed, but scalar native was 8,182,742
versus 7,647,935 ns, ratio 1.069928, so the complete verdict was **Rejected**;
scalar process wall passed at 9,340,049 versus 9,372,036 ns, ratio 0.996587.
The sentinel includes compiler/runtime evolution and does not attribute either
regression or recovery to optimizing passes. The adopted run preserves rather
than reinterprets this negative evidence.

One untimed allocation-graph metrics execution returned exact I64 `1`, entered
optimizing code three times, allocated seven objects, collected six times,
reported 14 attempted/14 successful heap calls and maximum three roots, and had
zero baseline entry/fallback with verified W^X. This is only a forced first-tier
boundary on one CPU, not a general language score. Automatic optimizing
promotion remains disabled and unmeasured; no OSR, deoptimization, or
speculation capability or measurement is claimed.
## Predeclared Automatic Proof-Promotion Gate

This gate is selected but has not run. The future automatic-optimizing control
is CLI-opt-in and disabled by default; existing auto baseline threshold 64 and both forced
tiers remain unchanged. A clean locked release protocol deterministically
randomizes at least four warmups and 31 unremoved samples per case for:

- auto baseline-only;
- auto optimizing thresholds 64, 256, 1,024, and 4,096 exact baseline entries
  of the promotion root;
- unchanged forced-baseline and forced-optimizing sentinels;
- allocation/reference correctness with exact roots and no reference VM/native
  entry transfer.

Every run checks the exact independent oracle and stream expectations; exact
state, epoch, token, object, and tier entries; proof/certificate verification;
W^X; stale invalidation/selection; attempts and suppressions; code/metadata/
mapping limits; and zero fallback. The Nth baseline entry must compile and
install synchronously while invoking captured baseline code, with the first
optimized entry occurring only later.

A threshold passes only with at least 1.10x median process speedup over auto
baseline-only, median improvement greater than twice the sum of both MADs,
nearest-rank p95 no more than 5% worse, measured compile/install cost repaid by
workload completion, exact correctness, no repeated attempt/fallback, and forced
scalar native/process medians no more than 5% above retained 7,647,935 ns and
9,372,036 ns. Select the largest passing threshold whose process median differs
from the fastest passing candidate by no more than twice the sum of their MADs.
If none passes, retain every sample and the rejection and keep automatic
optimizing disabled. A passing result is not Current default policy until a
later implementation/documentation adoption change.
## Claim Policy

Claims are category- and tier-specific: for example, “lowest median cold
startup in this published set” or “baseline JIT breaks even after N calls under
this cache budget.” A geometric mean may summarize a declared suite but never
replaces per-workload results. Regressions, failed compilation, VM fallbacks,
rejected candidates, and unfavorable categories remain visible.

[adopted-result]: ../../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64.json
[rejected-result]: ../../../meta/benchmarks/jit/results/optimizing-jit-linux-x86_64-rejected-scalar-regression.json
