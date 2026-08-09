# Roadmap

**Role: planned ordering and intent only.** This document owns no implemented fact and no normative
semantic rule. Current capability is in [`status.md`](status.md); intended contracts are in
[`spec/`](spec/).

## Now

1. **Add declaration deletion and movement coherently.** Define identity survival and local-binding
   removal/compaction so edits never orphan program-wide bindings; complete transaction, query,
   projection, dependency, compiler, and failure-atomic behavior in the same cutover.

## Next

1. **Add generic semantic calls and type arguments.** Preserve structured nominal/type-parameter
   identity, exact instantiation, effects, ownership, diagnostics, and imported convergence without
   exposing compiler-dense IDs or rendering source.
2. **Add unresolved, ambiguous, conflict, and recovery states deliberately.** Keep each state
   inspectable and editable, with truthful legal next actions and an explicit completeness blocker;
   never lower a fabricated executable fallback.
3. **Measure full recomputation under real edit workloads.** Record edit/query latency, allocation,
   retained memory, and invalidated work before deciding whether a narrow cache is justified. Add no
   query framework merely because full recomputation is currently simple.
4. **Reassess retained representation and crate boundaries.** Use profiles and Cargo evidence to
   merge any remaining crate or representation whose separate ownership does not justify compile,
   runtime, safety, or maintenance cost. Keep the direct local product and one active architecture.
5. **Define untrusted request policy only when an untrusted product exists.** Reusable execution
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
