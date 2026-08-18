# Evidence-gated roadmap

This file contains only future gates. Current behavior belongs to `docs/status.md`, contracts to
`docs/spec/`, and decisions/evidence to `docs/performance.md`.

## 1. Measure weak-model release authoring economy

Problem: exact reusable releases now work through command-local JSON and bounded inspection, but no
fresh weak-model/provider trial measures dependency binding, private-target diagnosis, R1/R2
selection, or diamond repair.

Gate:

- freeze equal create, bind, update, inspect, corrupt, missing-dependency, nominal-confusion, and
  offline rebuild tasks with independent semantic oracles;
- compare current release/app requests plus inspection with at most one narrow typed action or
  task-scoped context candidate;
- reuse unchanged exact interfaces by release/inspection digest where that removes repeated bytes;
- record success, unintended semantic changes, corrections/depth, calls, processes, Engine opens,
  files opened, action/observation bytes, elapsed boundaries, and actual provider token classes only
  when exposed; and
- retain one surface and delete every losing grammar/action/help path.

Oracle: equal exact release IDs, application bytes, graph, tests, results, and bounded errors.
Reversal: current command-local contracts remain when a new surface does not improve complete-task
success or correction depth.

## 2. Derive or narrow the workspace machine catalogue

Problem: `contract.rs` remains a large manual owner, while release and application facts proved that
command-local contracts can avoid global catalogue growth.

Gate:

- implement one representative request/response/error/nested variant with current metadata,
  `macro_rules!`, and one disposable derive/IDL candidate;
- preserve strict duplicate/unknown rejection and explicit cross-field validation;
- measure duplicate facts, source opened, expanded review, diagnostics, clean/incremental build,
  binary size, and focused Miri cost;
- compare global roots with command-local help and dependency-closed projection; and
- delete either the displaced manual owner or every generator completely.

Oracle: exhaustive strict-codec samples and help/schema agreement. Reversal: retain explicit manual
ownership when generation obscures invariants or raises total build/debug/review cost.

## 3. Revalidate managed bytes across both shared-release applications

Problem: the planner materially wins the retained append workload, but its planner, verifier,
generation handles, compiler integration, interpreter paths, and Miri surface remain substantial.

Gate:

- generate empty, tiny, typical, maximum, concat, slice, sharing, call, loop, trap, nominal, and
  result-materialization cases across both reusable applications;
- compare current planning, allocate-new, safe shared immutable values, and an invocation arena only
  where each is serious;
- preserve identical fuel, frames, cells, visible/retained bytes, objects, and result limits;
- measure allocations, copies, peaks, compile/plan/verify/run/materialize time, source, tests, Miri,
  sanitizer scope, binary size, and RSS when available; and
- leave one production route, keeping a simple oracle and deleting losing modes.

Oracle: exact result/trap/resource differential. Reversal: delete managed planning when broader
end-to-end benefit is marginal or a simpler representation matches it.

## 4. Measure application executable caching before adding a format

Problem: application format 2 validates release bytes and recompiles on each process invocation,
but present 2.9–6.2 KiB bundles do not establish a compile/startup bottleneck.

Gate:

- separate process startup, file read, release decode, graph validation, flattening, lowering, Core
  verification, and first instruction across short-lived runs;
- prototype at most one target-neutral verified Core-IR cache only after compile dominates;
- bind it to exact graph digest, compiler/backend identity, target, policy, and runtime contract;
- reverify hostile cache bytes and keep semantic compile/interpreter differential; and
- require material improvement in at least two of startup, repeated compile, dispatch, or transfer
  size before retaining a second executable format.

Oracle: identical semantic results, traps, origins, and resources. Reversal: delete the cache if
semantic compilation remains negligible or verifier/version surface dominates.

## 5. Scale workspace storage, release bundles, and queries

Problem: full snapshots/scans and embedded application graphs are simplest, but no large history or
many-application distribution establishes their long-term storage and restart cost.

Gate:

- vary durable entities, body bytes, releases, graph width/depth/diamonds, application count,
  history length, and repeated builds;
- measure workspace bytes, embedded duplicate release bytes, decode peaks, cold restart, history and
  graph validation, query visits, closure work, and corruption traversal;
- prototype an immutable object store, delta/checkpoint store, or one narrow index only after at
  least two recorded thresholds cross;
- specify atomic publication, reconstruction, retention, interruption-safe compaction/GC, exact
  object keys, store-path mismatch, and symlink/path-race behavior; and
- preserve full snapshots/scans and explicit bundles as independent oracles until direct cutover.

Oracle: byte-canonical reconstruction and identical accepted semantics. Reversal: retain current
forms if recovery/trust/operational cost exceeds measured savings.

## 6. Add resolution, provenance, or authenticity only for a real consumer

Problem: exact local composition deliberately begins after selection and provides integrity but no
human constraint resolution, registry, freshness, provenance, authorization, or signatures.

Gate:

- name a retained consumer that cannot supply exact release artifacts directly;
- keep intent, candidate view, selected exact bindings, lock evidence, content identity,
  provenance, signature, roles, freshness, revocation, and authorization as distinct domains;
- make resolution finish before accepted release/application authority and remain reproducible
  offline from immutable results;
- threat-model mirrors, rollback, freeze, key compromise, ambiguous publication, and metadata
  expiry using TUF/in-toto/SLSA lessons without importing unnecessary ecosystem scale; and
- delete the resolver/trust prototype if explicit exact inputs remain simpler or trust cannot be
  independently verified.

Oracle: immutable exact builds ignore mutable latest state; adversarial metadata cannot change an
accepted graph. Reversal: no registry or signing infrastructure remains without that consumer.

## 7. Introduce one capability-secure effect only for a real application

Problem: pure typed and byte-stream invocation cannot perform partial external actions, while the
current process boundary is deliberately not a sandbox.

Gate:

- select a retained application that cannot externalize interaction through complete input/output;
- define explicit typed permission, acquire/use/consume/close, order, timeout, cancellation,
  partial action, idempotency, retry, audit, crash, and deterministic fake-host contracts;
- keep affine external-resource cleanup separate from immutable managed memory;
- establish worker isolation and denial-of-service boundaries before untrusted effects; and
- delete the effect if authority, failure recovery, or cleanup cannot be exact.

Oracle: deterministic fake host plus failure injection at every lifecycle edge. Reversal: pure
application adapters remain default and ambient host access remains forbidden.
