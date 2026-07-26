# C8: Persistent Native Image Cache Candidate Rejected

[Authority](../experiments.md)

## Status

**Rejected.** This capsule is historical evidence and cannot promote the
candidate into Current code.

## Question

Would a bounded local persistent cache of canonical verified native images
improve repeated Linux x86-64 startup enough to justify a Current integration?

## Candidate

Commit `90aae65fcc8d458be27af75da8ed51b4e518d8df` implemented the complete
candidate. It keyed images by the full contract, package, source, verified SSA,
profile, tier, backend, policy, and target identity. Lookup decoded and
validated bounded canonical `InstallableImage` bytes before fresh W^X
installation. Optimizing proof construction and checking remained mandatory.
Publication used contained same-user storage, staged sync, atomic rename, and
directory sync. The cache was disabled by default.

The candidate also included corruption, stale-identity, concurrent publication,
resource-limit, proof-rerun, W^X, and zero-fallback tests. The Git commit retains
the implementation and exact test trailers.

## Predeclared Gate

The accepted candidate contract required at least 30 randomized samples for
disabled cold runs, enabled cold misses, and warm hits across scalar,
allocation, Brainfuck, editor, and SQLite workloads. Adoption required at least
20% lower time to first native entry and at least 10% lower end-to-end p50 on
three representative workloads. Cold-miss p50 and p95 regressions were capped
at 10% and 5%, warm RSS regression at 5%, and break-even at five executions.
Failure required removal from Current code or retention only as rejected
history.

## Evidence

The locked release campaign ran 30 samples for each of the 15
workload/condition combinations with five warmups and deterministic randomized
interleaving. The retained result is
[`native-image-cache-selection.json`](../../../meta/benchmarks/jit/results/native-image-cache-selection.json).
It records the candidate commit, environment, commands, source and artifact
hashes, every sample, p50/p95, RSS, cache status, and native-entry metrics.

Scalar, allocation, and Brainfuck each produced one exact cache artifact and 30
of 30 warm hits. Editor and SQLite produced no eligible native artifact because
their exercised entries remained VM-only under Current native type limits.

| Workload | Warm wall change | Warm first-native change | Cold p50 | Cold p95 | Break-even |
| --- | ---: | ---: | ---: | ---: | ---: |
| scalar | 0.94% slower | 190.27% slower | 17.00% slower | 18.82% slower | never |
| allocation | 2.15% slower | 133.54% slower | 45.82% slower | 44.46% slower | never |
| Brainfuck | 0.12% faster | 31.67% slower | 16.92% slower | 33.09% slower | 137 runs |

Editor and SQLite had zero cache lookups or hits. Their wall changes therefore
do not provide cache-benefit evidence.

## Decision

Reject and remove the candidate. Canonical native lowering is already faster
than bounded disk lookup and image decode for the eligible Current workloads;
durable publication makes cold misses materially worse. No representative
workload passed both warm-hit thresholds, all eligible workloads failed the
cold-miss thresholds, and break-even failed.

The result does not reject content-addressed caching permanently. A future
candidate requires materially larger native objects or a measured lookup and
publication design that passes a newly accepted predeclared gate. It must not
restore this implementation as a compatibility path.
