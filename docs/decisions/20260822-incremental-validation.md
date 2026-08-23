# Persistent semantic facts and full validation oracle

Date: 2026-08-22 UTC.

## Status

Accepted and partially implemented for the then-current meaning graph, semantic-summary,
semantic-fact, and validator contracts. Their current executable identities are generated in the
[contract registry](../generated/contracts.md); this decision does not duplicate them. Three
persistent fact maps, a constant-size authenticated certificate, a bounded dependency frontier,
and four local transaction classes are current. General frontier-driven publication validation is not yet
implemented; unsupported changes retain complete preparation.

## Selected model

Each canonical module produces an integrity-bound derived summary covering public signatures,
implementation and effect digests, graph-owned tests, and typed dependency facts for namespaces,
types, values, calls, capabilities, components, ports, targets, and tests. Summary identity binds
the exact module object, package, graph contract, summary contract, and validator contract.

Semantic-fact contract 3 indexes those facts in three independently rooted path-compressed Merkle
maps:

- module ID to exact summary-input and summary-content digests;
- test owner to an empty set value; and
- target, dependency kind, and dependent owner to an empty set value.

The flat reverse key makes every `(target, kind)` dependent set an ordered prefix range, so bounded
frontier traversal does not decode unrelated relations. A constant-size semantic certificate binds
the package, fact/summary/validator contracts, and the three map roots. The revision core stores
that certificate; the revision/root-bound `facts.lkix` manifest binds the same roots to one exact
repository, package, canonical root, and revision.

Content-addressed summary objects live under `indexes/semantic/summaries`; fact pages live under
`indexes/semantic/pages`. These files and the manifest are disposable acceleration. Missing,
malformed, predecessor, or digest-inconsistent cache state rebuilds from canonical modules. If a
canonical rebuild produces a certificate different from the accepted revision, the repository is
corrupt; cache bytes cannot substitute different meaning.

The predecessor single packed `ReverseDependencyIndex` was rejected because every local revision
rewrote a value proportional to all modules and edges. The selected maps path-copy changed ranges,
keep deterministic iteration, and retain a simple full construction and verification oracle.

## Current local preparation

Exactly four precondition-free transaction classes may avoid complete candidate reconstruction:

- `incremental_pure_body_slice` accepts eligible pure-function body replacements, loads the
  changed modules and recursively imported local dependencies, and records removed nested IDs as
  tombstone-map deltas.
- `incremental_independent_module_create` validates only new independent empty modules and exact
  namespace/root changes.
- `incremental_module_rename` updates the module object and ID/name map paths without loading or
  rewriting exact-ID importers or targets.
- `incremental_declaration_rename` updates owning modules and summary/name facts without loading or
  rewriting exact-ID callers.

Each local path replaces or removes only affected module-summary bindings, test owners, and reverse
edges. Batched map edits produce exactly the same roots and certificate as full fact rebuilding.
The prepared result also carries exact-owner/name index deltas; only touched content-addressed
buckets change. Focused differential tests compare logical roots, fact roots, certificates,
validation facts, and exact-index generations with full reconstruction.

Every request with preconditions and every unsupported mixed, move, signature, type, effect,
capability, target, test, or dependency change still uses `prepared_once_full_oracle`. That path
reconstructs and validates the complete candidate. Missing disposable state rebuilds or makes a
local optimization widen; it never permits narrower unchecked publication.

## Invalidation frontier

The fact engine classifies a one-module delta as unchanged, private implementation, or public
signature. Given an exact base manifest and changed before/after summaries, it traverses only typed
reverse prefixes, returns deterministically sorted modules to validate and retest, and records
edges, pages, and bytes read. A declared edge budget is mandatory; exhaustion is distinct and
publishes nothing. Stale revision bindings, foreign owners, and summary-digest mismatch reject.

Current property coverage proves private versus public propagation, test selection, relation
retargeting, test-owner replacement, stale-base rejection, budget exhaustion, 10,000-module
delta/full root equality, and bounded new pages. The frontier is not yet used as the general
transaction admission path, so no broader incremental-validation claim is made.

## Publication and oracle

Both local and complete preparation produce one exact result bound to base root, result root,
changed module objects, semantic-fact map roots, semantic certificate, semantic diff, and validation
facts. Publication rereads HEAD under the write lock, rejects a changed base, verifies the prepared
result bindings, writes new canonical bytes, and does not repeat semantic validation. Fact pages,
summary objects, and manifests are installed best-effort because their loss cannot block valid
canonical publication.

Complete canonicalization, complete semantic validation, packed reconstruction, deterministic full
root/fact rebuilding, and deep doctor remain independent correctness or recovery paths. Current
differential coverage is focused; there is no retained long randomized valid/invalid mutation
sequence and no claim that every invalidation class is implemented.

## Failure and reversal rules

- Accepted graph revisions are authority; summaries, fact pages, manifests, and prepared values
  are derived.
- Staleness, failed preconditions, malformed cache, exhaustion, cancellation, or inability to prove
  local eligibility publishes nothing or widens to the full oracle.
- Diagnostics, accepted/rejected status, no-change result, canonical root, and fact certificate must
  agree with the full path wherever local preparation applies.
- Disable a local path on any unexplained mismatch and retain the reproducer before re-enabling it.
- No optimization may delete the complete oracle, promote derived facts to meaning authority, or
  restore name-based reference invalidation.

## Remaining dependency closure

General role-driven propagation across signatures, types, effects, capabilities, components,
targets, tests, dependencies, and generic applications requires input-complete summary keys,
deterministic diagnostic equality, sparse and high-fanout mutation sequences, and explicit
work/I/O evidence. Incremental compiler units remain a separate layer. The broad query relation
index may consume the same accepted delta, but it remains a disposable query projection rather
than this validator fact owner.
