# Evidence-gated roadmap

This roadmap contains only future gates. Current behavior belongs to `docs/status.md`; accepted
semantics belong to `docs/spec/`; measurements and reversal conditions belong to
`docs/performance.md`.

## 1. Derive the executable contract from one field owner

Problem: the machine catalogue is now isolated from JSON codecs, but record fields and some shape
facts are still manually assembled. A new semantic form can still require distant matching edits.

Gate:

- choose ordinary Rust declarative metadata, a small derive, or a checked IDL only after implementing
  the same representative operation/query/error in disposable alternatives;
- preserve strict unknown-field/variant rejection and explicit semantic invariants;
- generate dependency-closed schema fragments, help cards, and document facts deterministically;
- fail verification on stale output;
- measure source opened, incremental/clean build time, proc-macro/build-script surface, binary size,
  and debugging cost;
- delete the manual copied owner if a candidate wins.

Oracle: strict codec samples and schema/help agreement remain exhaustive. Reversal: retain the local
manual catalogue if derivation saves little duplication or obscures accepted shapes.

## 2. Reduce maintenance process and proposal cost

Problem: direct Engine removed socket lifecycle, but the eight-revision agent corpus still launches
81 CLI processes and the primary declaration migration document is 9,100 bytes. The process target
and migration-byte target were missed.

Gate:

- prototype an agent-aware bounded session using the existing Engine and document contracts;
- compare explicit per-command publication with a script containing multiple exact commands;
- prototype apply-and-refresh only if measured follow-up context/diff calls dominate;
- preflight every returned delta before publication and include it in replay semantics;
- run one fresh sealed document-v1 agent trial only after deterministic gates;
- record exact provider token classes when exposed; do not rerun the raw control unnecessarily;
- retain no alternate session grammar or hidden client state.

Oracle: identical revisions, rejections, receipts, historical runs, and command audit. Reversal:
delete batching/deltas if they merely move bytes into larger responses or weaken failure diagnosis.

## 3. First-class tests and package artifact boundary

Problem: external Rust/Python tests are effective but do not travel with semantic programs, and a
workspace revision is not a publishable dependency unit.

Gate:

- identify one real two-package application and its independent behavior oracle;
- define immutable package identity/version/content domains without using storage digests as entity
  IDs;
- define manifest exports, dependencies, compiler inputs, tests, permissions, and provenance;
- decide entity preservation for publish, import, copy, fork, and vendoring;
- decode an untrusted package under exact bounds and reject missing, corrupt, wrong-schema, and
  foreign-domain references;
- keep workspace idempotency, history, aliases, and caches out of package bytes;
- compile/run an exact package graph and expose first-class test results in task context and review.

Oracle: workspace and package reconstructions yield equal accepted semantics while their metadata
domains remain distinct. Reversal: keep tests external and defer packages if the application does
not justify the semantic surface.

## 4. Storage and query scaling

Problem: current full snapshots are small, but reopen decodes all retained history and exact queries
scan the selected snapshot.

Gate:

- build deterministic scale corpora varying body size, durable entities, history length, incoming
  uses, and shared package content;
- measure artifact bytes, duplicate bytes, transaction peak memory, restart, resident memory, query
  scans, and corruption traversal;
- prototype at most the strongest candidate: delta/checkpoint, immutable objects/persistent tree, or
  embedded transactional store;
- require improvement in at least two retained dimensions such as growth, restart, branch fit,
  package reuse, query locality, or transaction allocation;
- specify publication, full reconstruction, retention, compaction/GC interruption, digest-kind
  separation, and filesystem attacks before selection;
- add narrow derived indexes only for measured scan hotspots and retain full-scan differentials.

Oracle: byte-canonical full reconstruction and semantic validation against the snapshot path.
Reversal: retain snapshots/scans if the added recovery and trusted surface outweighs measured wins.

## 5. Branches and parallel agent candidates

Problem: one authority-owned monotonic allocator is correct and simple, but unpublished parallel
entity creation and semantic merge are not modeled.

Gate:

- start from a retained parallel-agent task, not a generic collaboration framework;
- define immutable candidate parentage, authority status, bounds, cancellation, abandonment,
  publication, recovery, and exact query/run behavior;
- compare branch-qualified counters, authority-assigned random IDs, and explicit merge remapping;
- represent conflicts at durable entities and structural bodies rather than text offsets or storage
  hashes;
- define continuity maps explicitly and reject ambiguity;
- model-check the small candidate/publication state machine when practical.

Oracle: deterministic merge/rejection and identity non-reuse under generated interleavings.
Reversal: keep one-writer exact-base transactions until a real task wins on total correction/cost.

## 6. Managed-value revalidation with a second consumer

Problem: ownership-optimized bytes show a small retained benefit but carry compiler, verifier, and
runtime surface. One value class is weak architecture evidence.

Gate:

- select a real application requiring immutable text, a sequence, or another managed value;
- compare current ownership planning, invocation arena, safe `Arc` values, and typed-tree execution;
- measure compile/runtime source, verification tests, allocations, copies, peak retained bytes, fuel,
  escape behavior, and cleanup;
- preserve the allocate-new semantic oracle;
- delete the ownership planner and reuse path if benefit becomes marginal;
- do not add tracing without real cycles or expose memory choreography to authors.

Oracle: exact result/trap/fuel/resource differential across all modes. Reversal: complexity must stay
proportionate to representative wins.

## 7. Capability-secure effects and resources

Problem: pure programs cannot yet perform useful host work, and the local Engine is not a deployment
runtime or sandbox.

Gate:

- choose one narrow retained host-effect application;
- define explicit permission values, structured outcomes, order, cancellation, timeout, partial
  action, idempotency, retry, audit, isolation, crash behavior, and deterministic tests;
- model resource-owning values separately from ordinary immutable memory;
- require deterministic consume/close/cleanup on success, rejection, cancellation, and traps;
- establish a worker/sandbox threat model before executing untrusted host effects;
- keep deployment topology separate from workspace-authority topology.

Oracle: fake deterministic host plus failure injection for every lifecycle edge. Reversal: reject the
effect if its permission or partial-action contract cannot be made explicit.

## 8. Portable executable substrate

Problem: Core IR plus interpreter is a trustworthy bootstrap, not evidence for a final performance
tier. Only Linux x86-64 is verified.

Gate:

- obtain representative package/application compile and execution profiles;
- evaluate compact bytecode only if it improves dispatch, serialization, caching, or backend
  layering enough to justify a second derived format;
- retain the interpreter as the semantic oracle;
- bind every executable artifact to exact package graph, target, compiler/backend identity, policy,
  and memory contract;
- add cross-platform storage/client/build checks before support claims;
- isolate and validate any future native/unsafe boundary.

Oracle: differential behavior, deterministic traps, bounded resources, and reproducible builds.
Reversal: do not add JIT/AOT/native code from aspirational performance alone.
