# Roadmap

**Role: planned ordering and intent only.** This document owns no implemented fact and no normative
semantic rule. Current capability is in [`status.md`](status.md); intended contracts are in
[`spec/`](spec/).

## Now

1. **Remove the next measured borrow-call scale costs without restoring quotas.** Block-entry
   bytecode validation and generic prepared-identity work are complete. The retained matrix now
   shows approximately fourfold HIR-analysis and bytecode-lowering time, plus superlinear VM time,
   as calls double. Profile those exact paths for repeated scans or reconstruction; the VM's
   per-step linear cleanup-range search is already one concrete lead. Do not add caching or
   parallelism before repairing the simplest demonstrated cause.
2. **Record a representative post-cutover runtime baseline.** The one-shot baseline-native attempt
   plus VM fallback is the only product path. Extend the retained build/binary evidence with
   repeated startup, execution, peak-memory, and generated-code measurements across equivalent
   scalar, branch, call, structural, collection, ownership, failure, and host workloads. Reverse a
   runtime choice only on equivalent evidence.
3. **Complete stack-safety evidence for the selected local architecture.** Convert or otherwise
   prove remaining recursive transaction and runtime structural-value paths under deep generated
   tests. Keep concise workspace projection iterative and fallible as its semantic slices expand.
4. **Expand the implemented workspace vertical without adding another authority.** Add declaration
   and node create/delete/move, local-storage construction, generic calls, matches, and broader
   incomplete states on the retained identity/transaction/query/projection model.

## Next

1. **Add measured incremental recomputation.** Compare a small dependency-aware cache with a mature
   query framework on edit latency, invalidation precision, retained memory, cycle handling,
   cancellation, and debugging cost before choosing an implementation.
2. **Reassess retained representation and crate boundaries.** Use profiles and Cargo evidence to
   merge any remaining crate or representation whose separate ownership does not justify compile,
   runtime, safety, or maintenance cost. Keep the direct local product and one active architecture.
3. **Define untrusted request policy only when an untrusted product exists.** Reusable execution
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
