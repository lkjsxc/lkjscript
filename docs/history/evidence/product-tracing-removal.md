# Product Tracing Removal Evidence

[Tracing ratchet authority](../../decisions/memory/tracing-ratchet.md)

## Status

**Historical intermediate evidence; product absence remains Current.** At this
checkpoint the `product` family was removed while `enum` still used the
migration collector. The final zero-family evidence supersedes that remaining
runtime description.

## Closed Cutover

Every accepted product has one verified deterministic route. Copy and immutable
products use flat structural images; selected list-bearing and nested products
use acyclic invocation-region records. A product without structural or region
metadata rejects during HIR planning or independent SSA/bytecode verification.
Unknown generic parameters remain specialization blockers and caller-destination
requirements; they do not authorize a product tracing fallback.

`HeapObj::Product`, `ReferenceType::Product`, traced product allocation/access/
update, collector traversal, owned snapshot/debug/wire support, and test-only
construction are absent. Removed owned-object wire tag `1` rejects rather than
aliasing the retained enum tag. VM and native product operations are region-only;
structural products use structural instructions and services. Enum graphs retain
collector, root, mutation rollback, reachable snapshot, codec, and exact native
stack-map evidence.

## Evidence

Environment: Linux x86-64 locked workspace based on
`1adb82a6238e0cdf7dde3483afae2528075b0dc1`.

Commands run after implementation:

```text
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo run --locked -p lkjscript-xtask -- check-sources
cargo run --locked -p lkjscript-xtask -- structure check
cargo run --locked -p lkjscript-app --bin lkjscript -- memory traced --json
```

The workspace reports 60 passing test-suite results. The tracing command reports
exactly `enum -> Enum`. Source audits find no Rust occurrence of
`HeapObj::Product` or `ReferenceType::Product`. Focused adversarial evidence
covers rejected HIR blockers, unannotated SSA product construction, all three
non-region product bytecodes, removed wire tag `1`, foreign/cyclic region
metadata, and exact nested region identities. Forced baseline and proof product
fixtures continue to report zero collector allocations, roots, barriers,
collector calls, and collecting safepoints.

`check-docs` and `quiet verify` remain intentionally unpassed only on
`LKJ-PLATFORM-REVISION` until all public cutover commits are squashed into the
single revision-11 integration commit. Docker acceptance, retained workloads,
Miri, performance evidence, and final no-collector acceptance remain untested
for this intermediate one-family slice.
