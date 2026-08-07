# Roadmap

**Role: planned ordering and intent only.** This document owns no implemented fact and no normative
semantic rule. Current capability is in [`status.md`](status.md); intended contracts are in
[`spec/`](spec/).

## Now

1. **Finish boundary-policy separation outside the ordinary local path.** Phase 1 is complete for
   trusted local source loading, compilation, bytecode validation, and execution: those paths are
   explicitly unrestricted and do not use compiler profiles or source/HIR/SSA count budgets.
   Classify and simplify the remaining daemon, process-frame, output/argument, platform-observation,
   and OS/ABI bounds. Keep only genuine external representation boundaries and a small explicit
   coarse policy for untrusted requests. Preserve typed exhaustion, cancellation safety, and atomic
   publication.
2. **Delete the losing internal runtime paths.** The app now synchronously prepares one scalar
   baseline-native reachable group before effects, runs it when preparation succeeds, otherwise
   runs the VM, and never falls back after entry. Public forced tiers, threshold, and auto controls
   are deleted. Remove the repeated-auto, forced-tier, and optimizing APIs retained only by current
   tests, then rerun representative measurements with peak-memory, generated-code, release-size,
   and target evidence.
3. **Address measured scale costs without restoring quotas.** Start with preparation, bytecode
   validation, and peak memory exposed by the retained 16,385-call/borrow-scope harness. Profile
   repeated scans, whole-program clones, duplicate identity/serialization work, and unconditional
   representation construction before adding parallelism or scheduler infrastructure.
4. **Simplify platform and crate topology around the selected path.** Use Cargo and measured
   boundaries to merge or remove daemon, process-cell, scheduler, resource-topology, contract, and
   platform components that do not justify independent ownership. Retain narrow unsafe, FFI,
   executable-memory, path, process, and SQLite boundaries.

## Next

1. **Implement the first semantic-workspace vertical.** Build one in-memory immutable typed snapshot
   supporting a module with `main`, ordinary functions, primitives, calls, bindings, conditionals,
   and one incomplete-expression state. Provide edit-stable logical IDs, stale-revision rejection,
   atomic rename/replace/fill transactions, type/reference/hole queries, deterministic import and
   rendering, semantic diffs, and direct compilation without a text round trip.
2. **Cut text over to importer/projection status.** Remove syntax-shaped source authority and the
   bootstrap transaction path once the semantic vertical owns compilation. Do not retain parallel
   text and semantic compiler paths.
3. **Add measured incremental recomputation.** Compare a small dependency-aware cache with a mature
   query framework on edit latency, invalidation precision, retained memory, cycle handling,
   cancellation, and debugging cost before choosing an implementation.
4. **Complete stack-safety evidence for the selected architecture.** Convert or otherwise prove the
   remaining recursive semantic-operation, transaction, runtime structural-value, serialization,
   and specialization paths under deep generated tests.

## Later

1. Add persistence, crash recovery, collaboration, or distributed storage only after measurements of
   retained scale, concurrent writers, and recovery needs justify them.
2. Integrate immutable semantic snapshots, compilation caches, and warm runtime state with a
   simplified daemon through narrow revision-labelled interfaces.
3. Expand package, service, database, network, GUI, web, game, and other platform capabilities
   through the semantic model, capability system, and selected production runtime.
