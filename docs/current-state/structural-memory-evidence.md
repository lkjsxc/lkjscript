# Structural Memory Substrate Evidence

## Status

**Current only for the safe internal runtime substrate and resource-plane
adapter.** No source, HIR, SSA, bytecode, VM, native, or JIT structural family
selects these domains yet. Builtin storage is removed; capture-free functions
and symbols use inline artifact IDs. Six traced representation families remain.

## Implemented Facts

- Revisions `32374e9`, `e8de299`, `2d694a6`, `7e03b79`, and `5354ff9`
  implement the substrate and first monotonic tracing-family decrements.
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
- Unused builtin heap storage is removed. Capture-free functions use checked
  inline prototype IDs. Symbols use checked artifact IDs; returned snapshots
  retain only reachable canonicalized symbol text.

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
python3 meta/results/ai-authoring/validate.py meta/results/ai-authoring/results/*.json
cargo build --workspace --release --locked
./target/release/lkjscript run src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine baseline-jit src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine optimizing-jit src/examples/jit-optimizing/main.lkjscript
./target/release/lkjscript run --engine auto --auto-jit-threshold 2 src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/hello/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/mandel/main.lkjscript
python3 meta/benchmarks/brainfuck/benchmark.py --mode smoke --no-build
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/lkjedit-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/http-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/bulk-bytes-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/durable-files-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/sha256-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/sqlite-smoke.sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
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
- The no-RC candidate comparison, real region/pool HIR and backend selection,
  retained migration measurements for the six remaining families, sanitizers,
  and fuzzing remain untested. Miri is unavailable on the installed toolchain.
- No collector-free-runtime or no-tracing-runtime claim follows from this work.
