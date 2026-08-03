# Sealed Semantic DAG Rehydration Evidence

## Status

**Experimental safe-core prerequisite; not Current execution-tier behavior.**
This evidence covers one source-invisible adapter from an already validated,
key-free `SemanticDagSnapshot` to one coarse-owner sealed region. It does not
provide source `sealed`, executable-witness-fact binding, compiler selection,
evaluator/VM/native execution, provider integration, or persistent sessions.

## Exact Implementation

Implementation commits:
`b763c16bfe85a3fcb9099b6a98870d86697d1dd3` and
`db5f994bf66146b1d22ed2869ea88ba5500db71d`.
Platform revision remains `14`; no registered source, artifact, wire, runtime,
host, or provider contract changed.

`SealedSemanticDagRuntime::rehydrate` requires the exact expected root and a
bounded, sorted, duplicate-free caller-supplied set of semantic type, layout,
and payload-kind tuples. Every node must have exact membership before the
runtime creates a builder. `rehydrate_validated_return` adds a narrower path:
it derives the exact return representation, `StructuralTypeId` graph, product
field shape, and copy scalar/static leaves from a `ValidatedChunk`. It admits
only immutable structural types and rejects affine roots, resource-marked
fields, unique/resource/legacy routes, enums, bytes, byte vectors, missing
returns, forged identities, and shape disagreement before allocation. It does
not yet bind `InstalledMemoryWitness` executable facts.

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

## Rejected Witness-Fact Shortcut

A post-`a77a3572` candidate that accepted hand-authored sealed witness flags was
removed. At these prerequisite commits, compiler policy emitted unique
single-owner products and bytecode lacked complete executable authentication.
Revision 17 later added the narrow authenticated producer, verifier, placement,
and prepared route without restoring caller-selected flags. The rejected
shortcut remains negative evidence.

## Focused Coverage

The focused integration target has 11 passing tests, and four validated-return
unit tests pass. Coverage includes:

- mixed product-list-product sharing and exact round-trip identity;
- canonical multi-chunk string, path, and bytes export;
- unresolved type-set and object-limit rejection before builder allocation;
- deterministic mid-build chunk exhaustion with zero live domains afterward;
- wrong-runtime borrow ending that returns the loan token;
- final release rejection under a live loan that returns the owner token; and
- 8-versus-2,048-node products with one owner/dependency release-planning unit
  in both cases while reclaimed cell counts scale explicitly;
- exact validated return/`StructuralTypeId` shape derivation and round trip; and
- preallocation rejection of missing, forged, affine, resource-marked, and
  field-shape-mismatched validated metadata.

## Exact Commands

Environment: Linux `7.0.0-27-generic` x86-64, Rust
`1.96.0 (ac68faa20 2026-05-25)`, Miri
`0.1.0 (9f36de775b 2026-07-19)`.

The following commands exited zero at the first implementation commit:

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
The following additional commands exited zero at the validated-shape commit:

```text
cargo test --locked -p lkjscript-core validated_structural_return -- --nocapture
cargo run --locked -p lkjscript-xtask -- quiet verify
CARGO_TARGET_DIR=target/lkjscript/miri-validated-sealed-dag cargo +nightly miri test \
  --locked -p lkjscript-core --lib \
  validation::tests::structural::validated_structural_return_derives_exact_sealed_dag_shape \
  -- --exact --nocapture
```

The focused binder reported four passed tests, canonical verification completed
in 13.7 seconds, and Miri reported one passed test in 1.02 seconds. Three
pre-existing atomic `fetch_update` deprecation warnings and no error occurred.
Both reproducible Miri target directories were removed after recording compact
evidence.

## Untested At These Prerequisite Commits

- release-workspace and Docker gates after this prerequisite;
- ASan, LSan, and TSan after this prerequisite;
- malformed in-memory cell injection unavailable through the safe public API;
- `InstalledMemoryWitness` fact binding and process decode-to-runtime binding;
- validated enum and independently shareable bytes admission;
- compiler, evaluator, VM, baseline JIT, and proof JIT integration;
- source sealing, persistent sessions, provider IPC, and state transactions;
- non-Linux portability.
