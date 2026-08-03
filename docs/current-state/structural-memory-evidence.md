# Structural Memory Substrate Evidence

## Status

**Current for the safe runtime substrate and deterministic structural-value
cutover.** HIR memory authority, verified SSA, validated bytecode, evaluator,
VM, forced baseline, and forced proof execution select compact structural roots
for dynamic strings, paths, deterministic products/enums, recursive enums,
copy-leaf lists, structural results, destinations, projections, and cleanup.
Builtin storage is removed; capture-free functions and symbols use artifact
IDs. The later zero-family cutover removed the migration registry and collector;
this file retains earlier substrate commands as historical evidence.

## Implemented Facts

- `StructuralRuntime` internally issues process-unique runtime identities and
  exact class/slot/nonzero-generation domain keys. Safe callers cannot forge an
  identity or mutate runtime state outside a typed store. Generation changes
  before reuse, exhausted slots retire, and release capacity is preflighted.
- Typed roots bind domain, class, generation, layout, and semantic identity; safe constructors cannot fabricate keys.
- Production structural aggregate objects store one validated flat image with
  local node identifiers, node records, field ranges, and byte ranges rather
  than nested semantic values. Import, validation, clone, observation, merge,
  and release use bounded iterative work; a 12,000-level test covers conversion,
  clone, export, and release without host-stack recursion.
- Copy products construct, project, update, and return through flat images without
  collector service. Bytecode/native share the exact plan/name identity and scalar signatures.
  VM copy variables are not a native generic witness ABI.
- Every interned HIR type has a platform-bound canonical exact-facts witness independently
  reconstructed by the verifier over the resolved type and declaration set.
  Concrete regular recursive enums derive SCC closure; transformed, wrapped, or
  affine recursion fails. The narrow prepared vertical now binds residual
  witnesses and package provenance; broader operations remain Experimental in
  the [polymorphic evidence](memory/polymorphic-value-plane-evidence.md).
- Acyclic products closed over copy lists, scalar leaves, and selected region
  products use bounded records under canonical plan/name identities. All four tiers
  construct, project, update, call, and bulk-reset them. Native dispatch is
  noncollecting: no stack map, root, barrier, or collection call exists. Process
  codecs and main returns reject keys; see [evidence](memory/invocation-region-product-evidence.md).
- Copy-leaf lists use capacity-32 invocation segments; retained tails share
  immutable entries and no engine allocates pairs. Prepends retain allocation
  limits and bulk teardown. Flat snapshots reject unreachable tables and cycles;
  no runtime key or pair crosses boundaries.
- `StructuralRootTable` projects a typed root to one compact session-private
  64-bit slot/generation key. Entries retain exact runtime, layout, semantic
  type, owner state, and stale-safe shared or exclusive loans. Move, drop,
  sealed release, and static unregistration reject live loans, advance the slot
  generation before reuse, and return the typed root to its domain authority;
  the table never decides liveness or serializes a key.
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
  region-owner traffic is identical for 8 and 2,048 nodes; payloads carry no counts.
- Weak sealed roots never retain. Upgrade is generation/layout/type checked.
  Structured borrows block final release; cascading release follows dependency and side-drop ledgers.
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
  retain only reachable canonicalized symbol text. The authoritative HIR plan
  and its independent verifier now require those artifact values to use static
  trivial storage and byte-vector to use deterministic unique storage; none may
  consume a tracing-family registration.

## Focused Evidence

Environment: Linux x86-64, locked Rust workspace, revision-10 integration tree
based on `1517f17f70c8222378c9123179e79db7b380f0e6`. Every command below exited
zero on that integration tree.

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
cargo +nightly miri test --locked -p lkjscript-core --lib \
  gc::tests::collection::collection_preserves_nested_graph_and_reports_exact_counters
cargo +nightly miri test --locked -p lkjscript-vm --lib \
  run::unique::tests::trap_cleanup_releases_owner_and_exclusive_loan_once
MIRIFLAGS='-Zmiri-disable-isolation' cargo +nightly miri test --locked \
  -p lkjscript-vm --lib \
  run::tests::teardown::trap_and_exit_preserve_primary_outcomes_during_emergency_resource_cleanup
cargo +nightly miri test --locked -p lkjscript-native --test plan_validation \
  suite::runtime_abi::failure_cleanup_calls_are_typed_and_independently_verified
CARGO_TARGET_DIR=target/sanitizer-address RUSTFLAGS='-Zsanitizer=address' \
  ASAN_OPTIONS='detect_leaks=1:halt_on_error=1' cargo +nightly test \
  --workspace --lib --target x86_64-unknown-linux-gnu
CARGO_TARGET_DIR=target/sanitizer-leak RUSTFLAGS='-Zsanitizer=leak' \
  LSAN_OPTIONS='exitcode=23' cargo +nightly test --workspace --lib \
  --target x86_64-unknown-linux-gnu
CARGO_TARGET_DIR=target/sanitizer-thread RUSTFLAGS='-Zsanitizer=thread' \
  TSAN_OPTIONS='halt_on_error=1' cargo +nightly test -Zbuild-std \
  --workspace --lib --target x86_64-unknown-linux-gnu
```

Focused tests cover cross-runtime rejection, internal cycles, bounded ordinary
and sealed cascades, aggregate reverse drop failures, stale roots, live-loan
rejection, atomic seal failure, weak upgrade, pool retirement and deterministic
destruction, cyclic ECS graphs, pending remote-release authority, worker-queue
cleanup, and epoch exhaustion with no partial mutation.

## Compact Root-Table Evidence

Environment: Linux x86-64, locked Rust workspace, starting revision `90c6058`.
Commands actually run for the compact table, HIR authority repair, and final
zero-registry guard:

```text
cargo test --locked -p lkjscript-core --all-targets
cargo clippy --locked -p lkjscript-core -p lkjscript-contracts --all-targets -- -D warnings
cargo test --locked -p lkjscript-compiler --all-targets
cargo clippy --locked -p lkjscript-compiler --all-targets -- -D warnings
cargo test --locked -p lkjscript-xtask no_tracing
cargo test --locked -p lkjscript-xtask tracing_ratchet
cargo run --locked -p lkjscript-xtask -- check-sources
cargo run --locked -p lkjscript-xtask -- check-docs
cargo run --locked -p lkjscript-xtask -- structure check
cargo run --locked -p lkjscript-xtask -- quiet verify
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
CARGO_TARGET_DIR=target/lkjscript/miri cargo +nightly miri test --locked \
  -p lkjscript-core --test structural_roots --test structural_root_sharing
CARGO_TARGET_DIR=target/lkjscript/sanitizers/address \
  RUSTFLAGS='-Zsanitizer=address' RUSTDOCFLAGS='-Zsanitizer=address' \
  cargo +nightly test --locked -p lkjscript-core \
  --test structural_roots --test structural_root_sharing \
  --target x86_64-unknown-linux-gnu
CARGO_TARGET_DIR=target/lkjscript/sanitizers/leak \
  RUSTFLAGS='-Zsanitizer=leak' RUSTDOCFLAGS='-Zsanitizer=leak' \
  cargo +nightly test --locked -p lkjscript-core \
  --test structural_roots --test structural_root_sharing \
  --target x86_64-unknown-linux-gnu
CARGO_TARGET_DIR=target/lkjscript/sanitizers/thread-std \
  RUSTFLAGS='-Zsanitizer=thread' RUSTDOCFLAGS='-Zsanitizer=thread' \
  cargo +nightly test -Zbuild-std --locked -p lkjscript-core \
  --test structural_roots --test structural_root_sharing \
  --target x86_64-unknown-linux-gnu
CARGO_TARGET_DIR=target/lkjscript/cross cargo build --locked \
  -p lkjscript-host -p lkjscript-database --target wasm32-wasip1
```
The table tests cover compact-key category separation, wrong runtime/layout/type,
duplicate owner rejection, shared and exclusive conflict, live-loan release
rejection, move/drop state, root and loan slot reuse, generation retirement,
capacity failure without partial state, sealed region-level leases, and empty
completion. At this evidence checkpoint, the final no-tracing gate was
implemented but inactive while the migration registry was nonempty. Address, leak, and thread sanitizers and
Miri passed the focused table tests. Rust nightly does not provide an undefined
sanitizer, `cargo-fuzz` and repository fuzz harnesses are absent, and the WASI
probe built but could not execute because `wasmtime` is unavailable.
## Explicit Limits
- The structural contract digest is
  `9ba894d9d214c84ec286ecfb11df5da52b570c5d5d650ad2151559d582300d38`.
- Pool borrowing is currently bounded by Rust references inside the safe core;
  cross-call runtime loan slots remain a backend integration target.
- No general sealed compact-image representation exists beyond the narrow
  prepared sealed-region route.
- No per-node precise reference count is implemented or selected.
- At this substrate checkpoint the no-RC comparison was untested; revision 17
  later selected coarse sealed ownership and retained the comparison. General
  region/pool selection, undefined sanitizer, and fuzzing remain untested.
- This substrate checkpoint alone did not establish the later whole-runtime
  no-tracing claim; the final zero-family evidence does.
