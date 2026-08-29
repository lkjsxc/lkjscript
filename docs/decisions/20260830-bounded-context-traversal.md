# Bounded revision-pinned semantic context traversal

Date: 2026-08-30 UTC.

## Status

Accepted and implemented by normalized query contract 4 and CLI contract 13. This decision extends
the normalized-query authority decision of 2026-08-26 and supersedes only its deliberate exclusion
of context traversal.

## Decision

One public `query context` operation reconstructs a complete admitted local semantic neighborhood
from an exact immutable `RepositoryView`, canonical owner records, and committed forward/reverse
relation witnesses. It accepts one live local root, mandatory incoming/outgoing/both direction, and
depth 1 through 8. Breadth-first expansion assigns minimum local-owner distance. Package and foreign
owner endpoints remain selected relation boundaries and are never expanded.

The complete logical result is materialized before success under fixed maxima of 4,096 local
owners, 16,384 unique relations, and 32,768 witness visits plus separately derived map, store, and
decode admissions. Exhaustion or corruption returns no partial neighborhood. Owners order by depth
and canonical owner key; existing relation records follow in canonical forward-edge order.

Pagination recomputes that bounded result from the pinned view. Query-4 continuations bind
repository, package, revision, operation, root, direction, depth, ordering, output section, and
exclusive canonical key while excluding page item and byte limits. No frontier, graph result,
cursor handle, or accumulated state is encoded or stored.

Full snapshot reconstruction plus the existing canonical relation extractor is the independent
semantic oracle. Production traversal does not call that oracle and does not create or repair a
query index, cache, file, or session.

## Rejected alternatives

- The removed `SemanticQueryIndex` context implementation, repeated seeds, scalar work/fanout
  budgets, mutable continuation cursor, JSON requests, and `--continue` spelling remain rejected.
- Multiple roots, package-root context, relation-kind filtering, full owner bodies, generic impact,
  and historical traversal are separate contracts without a maintained need in this cutover.
- Returning a partial neighborhood on traversal exhaustion was rejected because pagination would
  then conceal semantic incompleteness.
- Persisting a frontier or carrying it in a token was rejected because the admitted result is
  replayable from immutable authority and the token must remain bounded and stateless.

## Consequences

Query-3 continuation bytes reject directly after cutover. Page limits may change during traversal,
but any change to authority, selector, operation, or ordering rejects. A context query can report
foreign endpoints without accessing foreign authority. Success, stale/malformed continuation,
cancellation, corruption, and exhaustion leave repository inventory unchanged.

The 0.1.11 source snapshot is unreleased; immutable v0.1.10 remains the supported public
distribution and does not gain this operation retroactively.

## Reversal conditions

Change this design only when a maintained workflow exceeds the fixed logical admissions or proves
that deterministic replay is materially unacceptable. Any replacement must retain one semantic
authority, exact revision binding, independently reconstructed equality, explicit resource
dimensions, atomic failure, and a dependency-closed removal of query-4 inputs and tokens.
