# Roadmap

**Role: planned ordering and intent only.** This document owns no implemented fact and no normative
semantic rule. Current capability is in [`status.md`](status.md); intended contracts are in
[`spec/`](spec/).

## Now

1. **Construct owned local values source-free.** Extend the existing authority with one complete
   local-binding and owned product/enum vertical: creation, move/borrow legality, expected types,
   cleanup on success/failure, projection, direct compilation, VM execution, imported convergence,
   and focused deep/failure tests. First characterize current imported semantics and retain only the
   smallest transaction surface needed; do not add another AST or a general incremental framework.

## Next

1. **Broaden semantic authorship by dependency-closed slices.** Add declaration deletion/movement,
   generic calls, matches, and the unresolved/ambiguous/recovery states only with their identity,
   transaction, query, projection, completeness, compiler, and failure-atomic behavior complete.
2. **Measure full recomputation under real edit workloads.** Record edit/query latency, allocation,
   retained memory, and invalidated work before deciding whether a narrow cache is justified. Add no
   query framework merely because full recomputation is currently simple.
3. **Reassess retained representation and crate boundaries.** Use profiles and Cargo evidence to
   merge any remaining crate or representation whose separate ownership does not justify compile,
   runtime, safety, or maintenance cost. Keep the direct local product and one active architecture.
4. **Define untrusted request policy only when an untrusted product exists.** Reusable execution
   supports explicit limited policy, but there is no semantic wire service. Do not rebuild framing,
   text publication, or multi-tenant admission machinery speculatively.

## Later

1. Add persistence, crash recovery, collaboration, or distributed storage only after measurements of
   retained scale, concurrent writers, and recovery needs justify them.
2. Add a daemon or warm semantic service only after the local semantic snapshot, transaction, query,
   and direct compiler input are complete and measurements show a process boundary is worthwhile.
3. Expand package, service, database, scheduler, network, GUI, web, game, and other platform products
   through the semantic model, capability system, and selected production runtime. Do not restore
   the deleted Phase 2 components as compatibility layers.
