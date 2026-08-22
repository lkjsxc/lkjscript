# Incremental validation summaries and full oracle

Date: 2026-08-22 UTC.

## Status

Accepted and partially implemented for meaning graph contract 4, semantic-summary contract 2, and
validator identity `lkjscript-semantic-validator-2`. Persisted summaries, a revision-bound reverse
index, an authenticated semantic certificate, and four local transaction classes are current.
General dependency-frontier validation is not implemented; all other changes retain complete
preparation.

## Implemented fact model

Each canonical module has an integrity-bound derived summary covering public signatures,
implementation/effect digests, tests, and typed dependency facts for imports, types, calls,
capabilities, components, ports, targets, and tests. Summary identity binds its exact module object,
package, graph contract, summary contract, and validator contract. A revision-bound
`ReverseDependencyIndex` records those summaries and reverse semantic roles.

Summary objects are persisted by content digest under `indexes/summary-objects`; the reverse index
is persisted as `semantic-dependencies.lkix` in the revision's disposable index generation. Every
revision core contains a revision-independent `semantic_certificate` digest of the complete exact
fact set. The files remain derived acceleration: missing or malformed bytes rebuild from canonical
modules. If the rebuilt certificate differs from the accepted revision, the repository is corrupt;
the cache cannot substitute different meaning.

The summary implementation also computes deterministic change classes and an invalidation
frontier. Those mechanisms are tested as fact/index behavior, but the frontier is not yet the
general publication validator.

## Current local preparation

Exactly four transaction classes may avoid complete candidate reconstruction, and only when the
request has no preconditions:

- `incremental_pure_body_slice` accepts eligible pure-function `ReplaceBody` operations. It loads
  the selected modules and their recursive local import dependencies, validates that slice,
  changes only the selected module objects and affected root paths, and emits tombstone-map deltas
  for nested identities removed by a structurally different replacement.
- `incremental_independent_module_create` accepts only independent empty-module creation. It
  validates the new modules and exact namespace/root changes without loading unrelated bodies.
- `incremental_module_rename` resolves modules through the persistent ID/name maps, changes the
  module objects and name-map paths, and validates the renamed modules plus their outgoing import
  dependencies. Exact-ID imports and targets are neither loaded nor rewritten as presentation
  consumers.
- `incremental_declaration_rename` resolves declarations through the revision-bound owner index,
  changes only owning modules and their summary entries, and does not load or rewrite exact-ID
  callers.

Each local path updates or removes the changed module summaries, updates the reverse index by
delta, computes the new certificate, and produces a prepared root delta. It also compares old and
new module owner projections to prepare exact-owner/name index contract 3: only touched
content-addressed buckets are rewritten and unchanged bucket digests are reused. Focused
differential tests compare the accepted logical root, validation result, and exact-index generation
with complete reconstruction.

Every preconditioned request, mixed request, declaration move, signature/type/effect/capability/
target/test/dependency change, and every other operation uses validation profile
`prepared_once_full_oracle`. That path reconstructs current logical meaning, clones the complete
root/module vectors, canonicalizes relations, and validates the complete candidate. It seeds the
new exact index from candidate values already in memory. A missing disposable exact or semantic
index makes local preparation widen to this complete path.

## Publication and full oracle

Both local and complete preparation produce one exact result bound to base root, result root, root
delta, changed modules, semantic diff, summary delta, reverse index, semantic certificate, and
validation facts. Local preparation additionally carries an optional exact-index delta with the
same bindings. Under the write lock publication rereads HEAD, rejects a stale base, replays and
checks the prepared delta and certificate, writes new immutable data, and does not repeat semantic
validation. Exact-index shards precede their manifest; because the index is disposable, failure to
install it does not block accepted authority.

Complete canonicalization, complete semantic validation, packed reconstruction, and deterministic
root rebuilding remain the correctness oracles. Current differential coverage is focused on the
admitted local classes and persistent-map properties; there is no retained long randomized
mutation sequence or claim that every invalidation class is implemented.

## Authority and failure rules

- The meaning graph and revision are accepted authority. Summary/index bytes and prepared state
  are derived; the revision's certificate only authenticates the exact derived facts for that
  graph.
- A stale base, failed precondition, inability to prove local eligibility, malformed cache,
  exhaustion, or cancellation never narrows validation unsafely or publishes a partial result.
- Missing cache state rebuilds or widens. Wrong-contract, foreign-revision, digest-mismatched, or
  certificate-inconsistent state never becomes reuse evidence.
- Diagnostics, accepted/rejected status, no-change result, and canonical root must agree with the
  complete path wherever local preparation applies.

## Remaining destination and reversal gate

General role-driven propagation across public signatures, types, effects, capabilities,
components, targets, tests, dependencies, and generic applications remains future work. It requires
explicit input-complete fact keys, deterministic diagnostics, sparse and dense high-fanout tests,
long valid and invalid mutation sequences, work/I/O counters that expose hidden scans, and clean
comparison with full validation and clean artifacts. Broad relation-index delta maintenance and
incremental compiler units are separate unimplemented layers.

Disable a local path and widen to the complete oracle on any unexplained mismatch. Re-enable it
only after retaining the failing mutation and passing focused differential coverage. No later
optimization may remove the complete oracle, promote disposable summaries to graph authority, or
restore name-based module invalidation.
