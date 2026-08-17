# Evidence-gated roadmap

This roadmap contains only future gates. Current behavior belongs to `docs/status.md`; accepted
contracts belong to `docs/spec/`; measurements and reversals belong to `docs/performance.md`.

## 1. Prove or reject reusable package artifacts

Problem: application artifact v1 closes one run-only lifecycle but deliberately does not support
independently released reuse, imports, private tests, or multiple exports.

Gate:

- retain a second application that consumes one shared semantic unit released independently;
- compare one duplicated application closure with an exact immutable package graph;
- define package coordinate, content identity, user version, exports, dependency constraints,
  provenance, import/remap, copy, fork, vendoring, and conflict behavior as separate domains;
- preserve nominal identity without inference from paths, names, hashes, or structure;
- keep workspace history, application entry/profile, idempotency, aliases, and caches out of package
  bytes;
- decode package and graph bytes under exact limits and corruption tests; and
- delete package scaffolding if reuse does not beat duplicated closure on product and maintenance
  cost.

Oracle: the two applications and their independent behavior cases pass from transferred immutable
content while workspace, application, and package facts remain distinct. Reversal: retain run-only
applications until real reuse pays for package identity and resolution.

## 2. Derive or narrow the workspace machine contract

Problem: the workspace catalogue still occupies 153,227 source bytes and manually duplicates DTO
field facts, although its digest, workbench help, context/document binding, and diagnostic clients
remain active consumers.

Gate:

- implement the same representative request, response, structured error, and nested variant with
  local manual metadata, `macro_rules!`, and a disposable derive/IDL candidate;
- preserve strict duplicate/unknown rejection and explicit cross-field validators;
- measure duplicate facts, source opened, expanded review, diagnostics, clean/incremental build,
  binary size, and focused Miri time;
- compare global projection with command-local help and dependency-closed roots; and
- delete the displaced catalogue or every losing generator completely.

Oracle: exhaustive strict-codec samples and help/schema agreement. Reversal: keep manual local
ownership when generation obscures invariants or saves less total cost than it adds.

## 3. Re-evaluate application authoring economy

Problem: application build cases are exact but supplied as versioned JSON, and only one application
workflow has deterministic byte/process measurements. No fresh provider trial covers the combined
document-and-application surface.

Gate:

- freeze create, test-add, failing-test repair, body refactor, declaration replacement, build,
  inspect, transfer, run, and corrupt-artifact tasks with independent outcomes;
- compare current bracketed document plus application JSON with one isolated equal-capability
  source-like proposal and, if repeated edits justify it, one narrow typed application action;
- add application/test context summaries only when repeated discovery or repair calls require them;
- compare one-shot processes with the existing direct session before designing script batching;
- record action/observation bytes, processes, failures, corrections, files/source opened, elapsed
  time, and provider telemetry only when exposed; and
- keep one grammar and delete every losing parser/help path.

Oracle: equal revisions, test set, artifact logical content, results, and corruption diagnosis.
Reversal: retain current exact surfaces when compact alternatives do not improve semantic success or
correction depth.

## 4. Revalidate managed bytes across distributions

Problem: the ownership route has a large absolute win on the retained 512-octet append shape but
still costs planner, verifier, handle-store, compiler, interpreter, Miri, and failure-state surface.

Gate:

- generate application inputs across empty, tiny, typical, maximum, concat, slice, sharing, call,
  loop, trap, and result-materialization shapes;
- compare current planning, allocate-new owned bytes, safe shared immutable values, and an invocation
  arena only where each is serious;
- keep logical fuel, frames, cells, visible/retained bytes, objects, and output limits identical;
- measure allocations, copies, peaks, RSS where supported, compile/plan/verify/run/materialize time,
  source, tests, Miri, sanitizer scope, and binary size; and
- leave one production route, deleting planner/handles/modes if a simpler route matches application
  bounds.

Oracle: exact result/trap/fuel/resource differential. Reversal: current complexity survives only
while repeatable absolute application benefit remains material.

## 5. Measure executable caching before adding a format

Problem: standalone application run validates and recompiles semantic content on every invocation,
but current startup and compile observations do not justify serialized IR or bytecode.

Gate:

- measure process startup, artifact read/decode, semantic validation, closure, lowering, IR
  verification, and first instruction separately across repeated short-lived invocations;
- prototype at most one target-neutral verified Core IR cache or compact bytecode only after compile
  or dispatch dominates;
- bind it to exact semantic digest, compiler/backend identity, target, policy, and runtime contract;
- reverify every untrusted load and keep semantic compile plus interpreter as differential oracle;
  and
- require material improvement in at least two of startup, repeated compile, dispatch, or transfer
  size before retaining a second executable format.

Oracle: deterministic semantic results, traps, resources, and origins. Reversal: delete the cache
when semantic compile remains negligible or verifier/version cost dominates.

## 6. Scale storage and queries with application history

Problem: full snapshots and full scans remain simple and correct, but no large application/test
history distribution establishes their long-term cost.

Gate:

- vary durable entities, function bytes, release cases, application closure, history length, and
  repeated builds;
- measure workspace/application bytes separately, duplicate semantic bytes, transaction and decode
  peaks, cold restart, history validation, query visits, closure discovery, and corruption traversal;
- prototype only the strongest delta/checkpoint, immutable-object, database, or narrow-index
  candidate after at least two thresholds are crossed;
- specify atomic publication, reconstruction, retention, interruption-safe compaction/GC,
  digest-domain separation, and filesystem attacks; and
- preserve full snapshot/scan reconstruction as the oracle until direct cutover.

Oracle: byte-canonical reconstruction and identical accepted semantics. Reversal: retain snapshots
and scans if added recovery/trust cost exceeds measured application benefit.

## 7. Introduce one capability-secure effect only for a real application

Problem: pure typed and byte-stream invocation cannot perform partial external actions, while the
current runner is deliberately not a sandbox.

Gate:

- select a retained application that cannot externalize interaction through complete input/output;
- define explicit typed permission, order, timeout, cancellation, partial-action, idempotency,
  retry, audit, crash, serialization, and deterministic fake-host behavior;
- model external resource consume/close separately from immutable managed memory;
- establish worker isolation and denial-of-service boundaries before untrusted host effects; and
- delete the effect if authority or cleanup cannot be made exact.

Oracle: fake deterministic host plus failure injection at every lifecycle edge. Reversal: pure
application adapters remain the default and ambient host access remains forbidden.
