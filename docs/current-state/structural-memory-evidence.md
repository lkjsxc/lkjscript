# Structural Memory Substrate Evidence

## Status

**Current only for the safe internal runtime substrate and resource-plane
adapter.** No source, HIR, SSA, bytecode, VM, native, or JIT structural family
selects these domains yet. Builtin heap storage is removed and capture-free
functions are inline prototype IDs; seven traced representation families remain.

## Implemented Facts

- `StructuralRuntime` internally issues process-unique runtime identities and
  exact class/slot/nonzero-generation domain keys. Safe callers cannot forge an
  identity or mutate runtime state outside a typed store. Generation changes
  before reuse, exhausted slots retire, and release capacity is preflighted.
- Typed roots bind domain, root class, slot generation, layout identity, and
  semantic type identity. Safe constructors cannot fabricate a live key.
- Ordinary typed regions use bounded aligned `Vec<T>` chunks and a separate
  fallible large-object path. Internal edge records may cycle. Release follows
  only child and side-drop ledgers; it does not inspect payload values.
- Region reset changes root generation. Child ownership is a checked forest,
  admitted only when its aggregate release work fits the configured bound.
  Aggregate drop-failure storage is reserved before mutation and reports use
  exact 64-bit totals. General shared dependencies use sealed regions.
- Sealed builders are private and unique. Batch seal validates a deterministic
  dependency graph and every transitive release-work bound before one atomic,
  allocation-free commit. Failure returns only still-valid builders. Published
  roots use checked non-atomic region-level owner counts.
- Weak sealed roots never retain. Upgrade is generation/layout/type checked.
  Structured borrows block final release. Cascading final release follows only
  dependency and side-drop ledgers.
- Typed pools bind pool, slot, generation, layout, semantic type, and Rust type.
  Stale and wrong-pool IDs fail before access; exhausted slots retire; iteration
  and destruction use ascending slot order. Cycles use non-owning typed IDs.
- `StructuralOwnerHomeTable` maps complete domain keys to generation-safe
  `DataOwnerId`s without truncation. Remote release consumes a fresh no-loan
  proof, enters a non-transferable pending state, retains epoch authority across
  drain, and completes teardown atomically. Empty worker queues are removed.
- Resource-plane generation exhaustion now retires IDs before reuse. Owner proof
  epoch exhaustion fails before changing home or loan state.

## Focused Evidence

Environment: Linux x86-64, Rust locked workspace, starting revision
`0d961d43efa944583375758e30f249472ba96f39`.

Commands actually run after implementation:

```text
cargo test --locked -p lkjscript-core --all-targets
cargo clippy --locked -p lkjscript-core --all-targets -- -D warnings
cargo test --locked -p lkjscript-resource --all-targets
cargo clippy --locked -p lkjscript-resource --all-targets -- -D warnings
cargo test --locked -p lkjscript-contracts --all-targets
cargo clippy --locked -p lkjscript-contracts --all-targets -- -D warnings
cargo run --locked -p lkjscript-xtask -- structure check
cargo run --locked -p lkjscript-xtask -- structure audit
cargo run --locked -p lkjscript-xtask -- quiet verify
cargo run --locked -p lkjscript-app --bin lkjscript -- memory traced --json
cargo miri --version  # unavailable for installed stable toolchain
```

Focused tests cover cross-runtime rejection, internal cycles, bounded ordinary
and sealed cascades, aggregate reverse drop failures, stale roots, live-loan
rejection, atomic seal failure, weak upgrade, pool retirement and deterministic
destruction, cyclic ECS graphs, pending remote-release authority, worker-queue
cleanup, and epoch exhaustion with no partial mutation.

## Explicit Limits

- The structural contract digest is
  `86e15c020b2f93eae0d278a272139c37acd9668d77487db3277783b246db4b4a`.
- Pool borrowing is currently bounded by Rust references inside the safe core;
  cross-call runtime loan slots remain a backend integration target.
- No sealed compact image is implemented.
- No per-node precise reference count is implemented or selected.
- The no-RC candidate comparison, HIR/SSA operations, execution-tier migration,
  live-family ratchet decrements, sanitizers, and fuzzing remain untested in
  this slice. Miri is unavailable in the installed stable toolchain.
- No collector-free-runtime or no-tracing-runtime claim follows from this work.
