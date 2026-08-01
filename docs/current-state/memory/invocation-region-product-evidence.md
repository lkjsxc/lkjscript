# Invocation-Region Product Evidence

[Authority](../../decisions/memory/authoritative-memory-plan.md)

## Status

**Historical intermediate evidence; the exact invocation-region slice remains
Current on Linux x86-64.** The zero-family evidence supersedes this checkpoint's
whole-runtime limitation.

## Exact Slice

A nonrecursive product is selected when it transitively contains a selected
copy-leaf list and every field is `unit`, `bool`, `i64`, `f64`, or an acyclic
selected region product. Producer and independent verifier reconstruct `OrdinaryRegion`, `RegionHandleCopy`, no
root projection, a Current ordinary-region destination, and process-codec
ineligibility. Structural-image products, symbols, owners, recursive/cyclic
region products, and unsupported list witnesses do not enter this route.

Evaluator, VM, forced baseline JIT, and forced proof JIT construct, project,
immutably update, call, and bulk-reset the same value semantics. Bytecode and
native references carry the canonical memory-plan/name identity. Main return,
foreign-arena keys, malformed identities, and unsupported field routes reject.
Native invocation-region dispatch emits no root map, safepoint publication,
collector call, or barrier. Region and list allocations retain aggregate,
allocation, and estimated heap-byte budgets.

## Evidence

Environment: Linux x86-64 locked workspace based on
`75de064aa417ef700a5f33c2ab908d2ce720a1e1`.

Commands run after implementation:

```text
cargo test --locked -p lkjscript-app --test jit_engines \
  regions::nested_region_product_and_list_graph_is_collector_free_in_all_engines
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo run --locked -p lkjscript-xtask -- structure check
cargo run --locked -p lkjscript-xtask -- check-sources
cargo run --locked -p lkjscript-app --bin lkjscript -- memory traced --json
grep -R -n -E '\b(Rc|Arc)<|Atomic(Usize|U64)|fetch_(add|sub)' \
  crates/lkjscript-core/src/structural/region_product \
  crates/lkjscript-core/src/structural/segmented_list \
  crates/lkjscript-vm/src/run/product.rs crates/lkjscript-jit/src/heap/products
```

The focused cross-tier fixture includes list construction, region-product
construction, immutable update, a nested region product, exact calls/projections,
forced collection configuration, and baseline/proof execution without fallback.
It reports zero legacy allocations, collections, roots, barriers, collector
runtime invocations, and collecting safepoints; region metrics report four
records and six fields across both nested layers. The ownership-count audit
produced no matches. The
tracing registry is now exactly `enum`; unsupported product closures reject
instead of retaining a tracing fallback.

`check-docs` and `quiet verify` remain intentionally unpassed only on
`LKJ-PLATFORM-REVISION` until all public cutover commits are squashed into the
single revision-11 integration commit. Docker, retained workloads, performance,
and final no-collector acceptance remain untested for this intermediate slice.
