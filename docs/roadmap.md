# Roadmap

**Role: planned ordering and intent only.** This document owns no implemented fact and no normative
semantic rule. Current capability is in [`status.md`](status.md); intended contracts are in
[`spec/`](spec/).

## Now

1. **Add one source-free early-return vertical.** The retained
   `crates/lkjscript-app/tests/fixtures/ownership-control.lkjscript` program is the current consumer:
   direct return construction would remove its remaining source-free control blocker while reusing
   existing return typing, cleanup, and exactly-once ownership paths. Keep the draft flat, stop before
   unrelated `break`/`continue`, and change course if focused convergence shows a lower-layer defect.

## Next

1. **Measure full recomputation under representative imperative edits and queries.** Source-free
   mutable counted loops now complete an ordinary stateful construction/execution workflow; measure
   edit/query latency, allocations, retained memory, and invalidated work before choosing any
   incremental mechanism. Keep full recomputation if the evidence does not justify more machinery.
2. **Add unresolved, ambiguous, conflict, and recovery states deliberately.** Keep each state
   inspectable and editable, with truthful legal next actions and an explicit completeness blocker;
   never lower a fabricated executable fallback.
3. **Define one concrete public movement vertical only when a present owner/order use case exists.**
   Private identity-preserving compaction is complete but is not public movement. Do not add generic
   coordinates, paths, or tree-editing machinery for symmetry.
4. **Reassess retained representation and crate boundaries.** Use profiles and Cargo evidence to
   merge any remaining crate or representation whose separate ownership does not justify compile,
   runtime, safety, or maintenance cost. Keep the direct local product and one active architecture.

## Later

1. Add persistence, crash recovery, collaboration, or distributed storage only after measurements of
   retained scale, concurrent writers, and recovery needs justify them.
2. Add a daemon or warm semantic service only after the local semantic snapshot, transaction, query,
   and direct compiler input are complete and measurements show a process boundary is worthwhile.
3. Expand package, service, database, scheduler, network, GUI, web, game, and other platform products
   through the semantic model, capability system, and selected production runtime. Do not restore
   the deleted Phase 2 components as compatibility layers.
