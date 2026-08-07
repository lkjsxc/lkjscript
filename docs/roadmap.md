# Roadmap

**Role: planned ordering and intent only.** This document owns no implemented fact and no normative
semantic rule. Current capability is in [`status.md`](status.md); intended contracts are in
[`spec/`](spec/).

## Now

1. **Address measured local scale costs without restoring quotas.** Start with preparation,
   bytecode validation, and peak memory exposed by the retained 16,385-call/borrow-scope harness.
   Profile repeated scans, whole-program clones, duplicate identity/serialization work, and
   unconditional representation construction before adding parallelism.
2. **Record a representative post-cutover runtime baseline.** The one-shot baseline-native attempt
   plus VM fallback is the only product path. Extend the retained build/binary evidence with
   repeated startup, execution, peak-memory, and generated-code measurements across equivalent
   scalar, branch, call, structural, collection, ownership, failure, and host workloads. Reverse a
   runtime choice only on equivalent evidence.
3. **Implement the first semantic-workspace vertical.** Build one in-memory immutable typed snapshot
   supporting a module with `main`, ordinary functions, primitives, calls, bindings, conditionals,
   and one incomplete-expression state. Provide edit-stable logical IDs, stale-revision rejection,
   atomic rename/replace/fill transactions, type/reference/hole queries, deterministic import and
   rendering, semantic diffs, and direct compilation without a text round trip.
4. **Complete stack-safety evidence for the selected local architecture.** Convert or otherwise
   prove the remaining recursive Semantic Source, transaction, runtime structural-value,
   serialization, and specialization paths under deep generated tests.

## Next

1. **Cut text over to importer/projection status.** Remove syntax-shaped source authority and the
   bootstrap transaction compiler round trip once the semantic vertical owns compilation. Do not
   retain parallel text and semantic compiler paths.
2. **Add measured incremental recomputation.** Compare a small dependency-aware cache with a mature
   query framework on edit latency, invalidation precision, retained memory, cycle handling,
   cancellation, and debugging cost before choosing an implementation.
3. **Reassess retained representation and crate boundaries.** Use profiles and Cargo evidence to
   merge any remaining crate or representation whose separate ownership does not justify compile,
   runtime, safety, or maintenance cost. Keep the direct local product and one active architecture.
4. **Refine untrusted request policy when an untrusted product exists.** Semantic Source already has
   coarse request policy and reusable execution supports explicit limited policy. Do not rebuild
   daemon/process framing or multi-tenant admission machinery speculatively.

## Later

1. Add persistence, crash recovery, collaboration, or distributed storage only after measurements of
   retained scale, concurrent writers, and recovery needs justify them.
2. Add a daemon or warm semantic service only after the local semantic snapshot, transaction, query,
   and direct compiler input are complete and measurements show a process boundary is worthwhile.
3. Expand package, service, database, scheduler, network, GUI, web, game, and other platform products
   through the semantic model, capability system, and selected production runtime. Do not restore
   the deleted Phase 2 components as compatibility layers.
