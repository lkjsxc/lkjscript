# S3 Structural Scale And Complete Graph

## Status

**Experimental, predeclared before implementation measurements.** Baseline failures are observed; candidate results and
selection remain unset.

## Question

Can the compact flat structural image, verified immutable call borrowing, exact tail transfer, and an in-memory complete
repository graph remove the accidental 4,096 ceilings without regressing small values, memory safety, deterministic
reclamation, graph completeness, or bounded query output?

## Baseline

- Repository implementation baseline: `006693f4cb8d5d7fb15f4f7fe54688df89c2a382`.
- Policy-only mission commit: `0b0611b9b0c6e1819f26ac8a6a1e58dc4859c053`.
- Platform revision: 19.
- Linux `7.0.0-27-generic`, x86-64, AMD Ryzen 9 9955HX, 32 GiB reported RAM.
- Rust `1.96.0`; Cargo `1.96.0`.
- Baseline graph: 4,096 retained nodes, 9,708 retained edges, `truncated=true`.
- Baseline Brainfuck direct diagnostic: typed `structural limit exceeded: Domains` before output.
- Independent oracle: 6,240 bytes, SHA-256
  `83a0aac65090b3b5e85c22337afac39d8ac17bfd88675f044b33bd55ca0c351b`.

## Candidates

Structural representation:

- A: existing contiguous flat node, field-cell, and payload vectors with exact preflight;
- B: fixed-capacity segmented flat storage; and
- C: hybrid contiguous small/medium and segmented large images.

Call/lifetime behavior:

- preserve exact memory-plan `borrow-shared` owner references across nonescaping calls;
- release dead copy owners before VM tail-frame replacement; and
- keep native recursion when native tail transfer is not separately proven.

Graph:

- A: complete in-memory canonical graph with typed exhaustion;
- B: sorted producer runs and content-addressed shards.

The immediate candidate safety maxima are 16,384 graph nodes and 65,536 graph edges. They are accepted only if the
complete current graph uses less than half of each maximum. Exceeding that condition falsifies the candidate rather than
automatically moving the threshold.

## Workloads

- tiny scalar and one-node structural values;
- 4,095, 4,096, 4,097, 16,384, and 65,536 nodes;
- balanced, wide, and deep-but-valid shapes;
- independent field-cell and payload-byte limits;
- build, projection, clone, equality, export, codec where Current, and release;
- allocation/overflow failure before publication;
- 8,192 sequential publish/drop cycles and explicit live-capacity exhaustion;
- immutable direct-call recursion with clone/domain metrics;
- Brainfuck smoke, direct diagnostic, correctness, and folded full workload when time permits;
- clean repository graph build twice and focused context/impact queries.

## Metrics

Primary correctness metrics are exact value/output equality, typed failure, graph node/edge completeness, deterministic
identity, zero fallback, and zero final roots, loans, domains, reservations, destinations, views, or release backlog.

Performance metrics are wall time, CPU time where available, peak RSS where available, allocations, allocated/copied
bytes, live/peak domains, roots, and objects, clone bytes, borrowed calls, release work, graph work, and serialized
query bytes, and affected native code size where observable.

Use one release warmup and at least three measured samples for expensive end-to-end workloads when practical. Focused
microtests use deterministic repeated assertions rather than claiming stable wall-clock results.

## Acceptance And Falsification

Select the flat image if 65,536 nodes complete without native-stack recursion, partial publication, or disproportionate
tiny-value regression and no candidate demonstrates a measured need for indirection. Defer segmentation rather than
shipping two unproved production paths.

Borrow/tail behavior passes only if the Brainfuck domain failure disappears without raising `max_domains`, direct
immutable call SSA contains no physical structural copy, all implemented tiers agree, and teardown is empty.

The graph passes only if its successful result covers every accepted producer record, exceeds 4,096 nodes, is
byte-identical across repeated builds, retains closed endpoints, and fails atomically at exact plus-one work, byte,
node, and edge bounds. A bounded query must expose exact completion state and cannot claim complete impact after budget
exhaustion.

Reject on wrong output, stale/wrong-runtime acceptance, fallback, leaked capacity, unchecked arithmetic,
partial graph publication, identity nondeterminism, or small-value regression above 10% in a controlled repeated release
measurement. A noisy or unavailable measurement is recorded as unavailable, not a pass.

## Correctness Oracle And Cleanup

Structural values compare against iterative semantic export and exact type/layout identity. Brainfuck compares byte for
byte against the pinned independent C oracle. Graph builds compare canonical JSON bytes and endpoint closure. Every test
requires deterministic release or rollback before observing success.

## First Candidate Result

The 16,384-node graph maximum is **Rejected**. Complete-or-error construction observed 9,492 canonical nodes before
publication. That is 57.9% of the candidate maximum and violates the predeclared below-50% headroom condition. No graph
or identity was published. The result does not authorize moving this threshold; S4 predeclares a replacement candidate.

## Not Yet Measured

Peak RSS tooling, sanitizer overhead, native branch/indirection counters, all candidate representation timings, full
folded Brainfuck timing, and clean-versus-incremental graph equivalence are not yet measured.
