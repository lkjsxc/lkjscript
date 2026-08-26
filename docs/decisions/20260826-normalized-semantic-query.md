# Normalized semantic query authority and continuations

Date: 2026-08-26 UTC.

## Status

Accepted and implemented by normalized query contract 3 and CLI contract 7.

## Decision

Released `query` reads only the current accepted normalized `GraphRepository` through one
revision-pinned immutable view. The exhaustive public actions are bounded live-owner enumeration,
exact canonical namespace lookup, and one incoming or outgoing relation prefix for a live local
owner or the current package. Canonical owners remain program authority. Committed namespace and
relation witnesses are authenticated bounded locators and evidence, not a second editable graph.

Paged operations use a bounded stateless continuation containing an exclusive canonical logical
resume key. Its canonical payload binds query/continuation versions, repository, package, exact
revision, operation, normalized selector and ordering digest, plus a domain-separated integrity
digest. Item and output-byte limits are deliberately excluded so valid limits may change on a
later page. Continuations never contain page coordinates, pack offsets, process handles, or
repository-side state.

Public correctness has no query-index dependency. Ordinary query descends persistent maps from the
logical lower bound, reads canonical objects and committed witnesses, reports separate map, store,
decode, and output dimensions, and neither invokes complete reconstruction nor writes or repairs
an index. Full reconstruction and canonical relation extraction remain independent test oracles.

Context traversal and generic impact are excluded. Impact depends on an exact candidate change and
its semantic dimensions, so its next authority is reviewed change-plan evidence. Context requires
a separately justified traversal and continuation design.

## Rejected alternatives

- Marker-selected dual dispatch and normalized-then-predecessor fallback were rejected because they
  preserve two public authorities and make equal argv depend on ambient project edition.
- Adapting the predecessor `SemanticWorkspace`/`SemanticQueryIndex` path was rejected because
  its JSON/request grammar, scalar work budget, index cursors, and broad in-memory behavior do not
  satisfy normalized authority or logical pagination.
- Retaining callers/callees/types/capabilities aliases, JSON query files, or predecessor
  continuations was rejected because one behavior has one current public name and contract.
- Adding a persistent query index, cache, session, or daemon was rejected because committed maps
  already support bounded correct reads and no measured public need justifies mutable query state.

## Consequences

A changed HEAD makes a continuation stale even when its key still exists. Missing or inconsistent
committed witness data is corruption rather than an index miss; no-match is successful only when
the namespace key is absent. Foreign relation endpoints may be reported but never cause ambient
foreign repository access. Private predecessor query indexes remain only for named out-of-scope
workspace, diff, legacy inspect, change, transaction, and repository consumers and do not own
public query behavior.

## Reversal conditions

Change the logical continuation representation only when a new public query needs traversal state
that cannot be canonically replayed within measured admission bounds; prefer an explicit bounded
output file before mutable server state. Add an acceleration index only when equal end-to-end
workflows demonstrate a material need, canonical authority remains sufficient to rebuild and
verify it, and deletion/reversal criteria are recorded. Expand public query only with a named
consumer, independent oracle, bounded deterministic result contract, and dependency-closed
predecessor deletion.
