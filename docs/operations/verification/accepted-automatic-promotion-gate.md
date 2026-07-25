# Verification: Accepted Automatic Promotion Gate

[Authority](../verification.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Accepted Automatic Promotion Gate

This gate is an **Accepted Implementation Selection**, not Current and not yet
run. Focused tests must preserve the existing threshold-64 baseline behavior,
then prove exact `BaselineCandidate`/`BaselineCompiling`/`BaselineNative` and
`OptimizingCandidate`/`OptimizingCompiling`/`OptimizingPending`/
`OptimizingNative`/`Disabled` transitions. At exact optimizing threshold N, the
test must show synchronous proof/lowering/W^X installation, invocation of the
captured baseline object on that entry, and optimized publication only on a
later entry.

Tests must reject mismatched opaque function/object/tier tokens, stale or
invalidated selection, a second pending object, a second attempt in one epoch,
attempts beyond the total bound, unbounded mappings/optimizer work/certificates/
metadata, and reference-signature auto entry. Structured failure keeps baseline
current and records suppression; an explicit newer epoch invalidates pending or
current optimizing selection back to baseline and permits at most one bounded
retry. Stale mappings remain owned and non-selectable until session drop. Main
stays VM, scalar auto entry is unchanged, internal generated reference helpers
may still call/allocate, and forced tiers remain fallback-free.

The retained performance run must use one clean locked release build,
deterministic randomized ordering, at least four warmups and 31 samples per
auto baseline-only and 64/256/1,024/4,096 optimizing-threshold case, unchanged
forced sentinels, and allocation/reference correctness. It checks exact oracle,
streams, states/tokens/objects, proof, W^X, attempts/suppressions, invalidations,
entries/bytes/times, and limits. Mechanical adoption is at least 1.10x median
process speedup, improvement greater than twice combined MAD, p95 at most 5%
worse, compilation repaid before completion, historical forced scalar native
and process medians at most 5% worse, and no repeated attempt/fallback. The
largest candidate within twice combined MAD of the fastest passing process
median wins; otherwise the rejection is retained and optimizing stays disabled.
## Performance Evidence

A benchmark is decision-grade only with a declared baseline, environment,
correctness oracle, randomized repetitions, dispersion, and adoption threshold.
Use [../vision/experiments.md](../../vision/experiments.md). The current
single-shot C script is diagnostic only.

The retained callable scalar gate is:

```sh
cargo run --locked -q -p lkjscript-xtask -- quiet verify
cargo build --workspace --release --locked
python3 meta/benchmarks/jit/benchmark.py
```

It rejects fewer than four warmups or 31 samples per VM/forced/auto variant,
randomizes with a fixed recorded seed, checks exact F64 bits and stream silence,
polls `/proc` RSS, requires fallback-free forced native entry and successful
auto later-call entry, and retains every phase/sample. Results at selected
threshold 64 and alternatives 1/1,024 live under
`meta/benchmarks/jit/results/`. Implementation commit `025cbb2` measured 46.146x
native execution, 37.829x forced process wall, and 1.653x auto process wall over
same-commit VM; the full environment, dispersion, costs, pre-JIT diagnostic,
and limitations are in [Experiment C4](../../vision/experiments.md#c4-callable-scalar-baseline-jit-adopted).
## Current Native-JIT Gates

Focused forced-native tests prove an installed W^X code object, actual generated
main and callee entries, direct relocatable native calls, versioned PollV1
calls, nonzero counts, no fallback, and exact evaluator/VM/native scalar values
or structured outcome categories. Forced unsupported semantics and native
resource failures are engine errors rather than VM fallback. Auto tests use a
low deterministic threshold and prove compilation at one call is used only by
later calls while unsupported code remains VM-correct and retry-suppressed.

The CLI implements `vm`, `auto`, `baseline-jit`, and forced
`optimizing-jit`; ordinary `run` defaults to `auto` at 64 function entries,
while explicit `vm` remains deterministic and either forced JIT mode fails
rather than downgrading.
Tests check both selections. Machine diagnostics and low-overhead metrics are
separate, stderr/file-only, opt-in, and silent during normal execution. Metrics retain exact outcome bits,
compile/HIR/effect/SSA/bytecode/native/install/first-entry/first-call/VM/native/
engine times, tier states and failures/fallbacks, entries/direct calls/PollV1,
actual optimization phase/pass counts, aggregate optimization work,
instruction growth/removal, estimated certificate/optimization metadata bytes,
code/metadata/cache peaks, allocations/deterministic estimated object
bytes/collections/estimated peak live heap, roots, distinct attempted and
successful heap runtime calls, barriers, peak native frame depth, and transition
counts. The selected automatic slice additionally requires exact enables/
thresholds, epoch, attempts/failures/suppressions, baseline entries and elapsed
time before first optimized entry, tier object IDs/tokens/code bytes, W^X/proof,
stale invalidations, current/pending selection, and retained mapping/attempt/
work/certificate/metadata limit facts. Collection pause distribution is not currently measured.
Forced optimizing tests additionally require a retained bounded certificate,
exact rewrite counts, `Tier::Optimizing`, W^X installation, nonzero optimizing
entries, zero baseline entries/objects, zero VM fallback, smaller generated
code for the declared repeated-expression workload, and exact evaluator/VM/
baseline/optimizing values or structured outcomes. Allocation-graph attempts,
successes, bytes, collections, and returned values remain equal between forced
baseline and optimizing execution. Budget and unsupported failures are visible
engine errors. `auto` remains baseline-only. Owned-buffer/lexical-reference,
Handle/host paths, native/VM reference transitions, OSR, broader proof passes,
and background compilation are outside the current subsets. Automatic
optimizing promotion is also outside Current behavior, but its next gate is the
Accepted Implementation Selection above. Host-independent GC references/allocation and
recursive SCCs are Current only in forced mode; auto remains conservative. The containing
host-independent allocation commit based on `0daa7a0` passed
focused core/native/sys/JIT/VM/app tests, strict affected Clippy, separate
docs/tree/source checks, `quiet verify` with 182 unit/integration tests plus one
compile-fail doctest, a locked release build, scalar/hello/Brainfuck smokes, and
a forced allocation-graph metrics smoke. Docker, performance, and full
Brainfuck Mandelbrot were not run. Broader host-capability/native-transition
and performance evidence remain separate gates.

The containing forced first optimizing commit, based on `cd4eee2`, passed the
locked 209-test workspace suite plus compile-fail doctest, strict workspace
Clippy, separate docs/tree/source checks, `quiet verify`, locked release build,
and forced scalar/allocation/optimizing smokes recorded in
[Current State](../../current-state.md). The optimizing workload emitted 2,788
versus baseline 3,405 code bytes and entered optimizing code 10,001 times with
zero baseline entry or fallback. This is operation/code-size evidence, not a
1.20x runtime-performance result. Docker and performance sampling were not run.
## Rule

A command that did not run did not pass. Historical success is not evidence for
a later commit.
