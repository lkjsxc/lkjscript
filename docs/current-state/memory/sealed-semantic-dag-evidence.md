# Sealed Semantic DAG Rehydration Evidence

## Status

**Experimental safe-core prerequisite; not Current execution-tier behavior.**
This evidence covers one source-invisible adapter from an already validated,
key-free `SemanticDagSnapshot` to one coarse-owner sealed region. It does not
provide source `sealed`, installed-witness binding, compiler selection,
evaluator/VM/native execution, provider integration, or persistent sessions.

## Exact Implementation

Implementation commit:
`b763c16bfe85a3fcb9099b6a98870d86697d1dd3`.
Platform revision remains `14`; no registered source, artifact, wire, runtime,
host, or provider contract changed.

`SealedSemanticDagRuntime::rehydrate` requires the exact expected root and a
bounded, sorted, duplicate-free caller-supplied set of semantic type, layout,
and payload-kind tuples. Every node must have exact membership before the
runtime creates a builder. The supplied set is not yet derived from a validated
installed witness table.

A complete preflight converts nodes, local child IDs, and payload bytes into
fixed-size cells, checks object, byte, edge, and checked-arithmetic bounds, and
then initializes one private region. Semantic DAG edges become internal region
edges. Publication uses the existing atomic sealed-region transition.
Allocation, edge, and seal failures publish no owner and return the unchanged
snapshot. The adapter's dropless rollback removes a private builder without an
allocating cleanup path.

One explicit region borrow exports a fresh key-free snapshot without owner
traffic. Export requires contiguous canonical auxiliary ranges, complete cell
coverage, exact byte-chunk counts and lengths, zero padding, and successful
snapshot revalidation. Failed borrow ending and failed final release return the
original loan or owner token.

Independent lifetime divergence performs one checked region retain. Owner and
dependency release planning does not follow semantic edges. Physical final
reclamation still frees every region chunk and is not claimed node-count
invariant. No cell has an owner count, runtime key, dense witness slot, pointer,
provider token, resource, or affine payload.

## Focused Coverage

The focused integration target has 11 passing tests. New coverage includes:

- mixed product-list-product sharing and exact round-trip identity;
- canonical multi-chunk string, path, and bytes export;
- unresolved type-set and object-limit rejection before builder allocation;
- deterministic mid-build chunk exhaustion with zero live domains afterward;
- wrong-runtime borrow ending that returns the loan token;
- final release rejection under a live loan that returns the owner token; and
- 8-versus-2,048-node products with one owner/dependency release-planning unit
  in both cases while reclaimed cell counts scale explicitly.

## Exact Commands

Environment: Linux `7.0.0-27-generic` x86-64, Rust
`1.96.0 (ac68faa20 2026-05-25)`, Miri
`0.1.0 (9f36de775b 2026-07-19)`.

The following commands exited zero at the implementation commit:

```text
cargo test --locked -p lkjscript-core --test semantic_dag_snapshot
cargo clippy --locked -p lkjscript-core --all-targets --all-features -- -D warnings
cargo run --locked -q -p lkjscript-xtask -- structure check
cargo run --locked -q -p lkjscript-xtask -- check-sources
cargo run --locked -p lkjscript-xtask -- quiet verify
CARGO_TARGET_DIR=target/lkjscript/miri-sealed-dag cargo +nightly miri test \
  --locked -p lkjscript-core --test semantic_dag_snapshot \
  sealed_rehydration::validated_dag_rehydrates_and_round_trips_through_one_coarse_region \
  -- --exact --nocapture
```

Focused tests reported 11 passed. Canonical `quiet verify` completed in 13.8
seconds. Miri reported one passed test in 0.53 seconds; three pre-existing
atomic `fetch_update` deprecation warnings were emitted and no error occurred.
The reproducible Miri target directory was removed after recording this compact
evidence.

## Explicitly Untested

- release-workspace and Docker gates after this prerequisite;
- ASan, LSan, and TSan after this prerequisite;
- malformed in-memory cell injection unavailable through the safe public API;
- installed-witness closure derivation and process decode-to-runtime binding;
- compiler, evaluator, VM, baseline JIT, and proof JIT integration;
- source sealing, persistent sessions, provider IPC, and state transactions;
- non-Linux portability.
