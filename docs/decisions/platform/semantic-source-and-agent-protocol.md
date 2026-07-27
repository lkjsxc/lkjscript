# Semantic Source And Agent Protocol

## Purpose

Define the primary manipulation boundary for complete and incomplete programs,
the deterministic the removed legacy source contract projection adapter, and the smallest complete
agent-facing transaction/query slice.
## Status


**Current** for closed `lkjscript.semantic-source` version 2 over the removed legacy source contract and
the implemented the canonical source contract identity, generic-enum, construction, closed-pattern,
source-match, and Never/structured-control slices. Source-unit records expose
exact edition and edition-framed identity; snapshots expose tree edition and
identity; marker, enum, match, pattern, Never, loop, return, break, continue,
trap, exit, and edition-number nodes roundtrip strictly.
Schema identity remains version 2 rather than introducing an edition-specific
schema version.
Schema preserves the exact legacy contract node, value, declaration, type, built-in,
trivia, transaction-expression, diagnostic, and correlation representation as
its base and adds typed-hole source identity, bounded legal actions, exact enum
and match transaction expressions, and match-arm expected/scope facts. Version
1 input is historical and rejected; there is no alias, shim, or fallback.
Unknown kinds, fields, operations, versions, duplicates, and trailing input
fail. Inferred facts remain derived authority.

The bounded one-shot `snapshot`, `read-entity`, `query-node`, `diagnostics`,
`hole-context`, `legal-actions`, atomic `rename`, `replace-expression`,
`insert-hole`, `fill-hole`, `refine-hole`, and structurally legal `delete-hole`
operations emit the canonical contract. Hole context comes from the pinned parsed tree and a bounded
checker-valid completion; unavailable expected, capability, ownership-correlation,
qualification, and the removed legacy source contract control facts are explicit rather than invented.
One outer-owned resource profile ledger now spans strict request decode, exact loaded
shape/query work reservation, snapshot/query/hole/action construction,
transaction staging, and exact response encoding. Hole, legal-action,
transaction, impact, and staged-publication categories reserve before candidate
or transaction staging allocation. The public `_with_ledger` result preserves
a typed `BudgetError` and deterministic prefix. Schema's closed error record
cannot carry that new typed prefix without changing its exact schema contract, so wire failures retain
the deterministic budget rendering in `message`; this wire gap is explicit.

The Current local `semantic serve --stdio` session uses the same typed canonical
engine with no serialized inner one-shot request/response round trip, exact
framing, revision pinning, external-change rejection, refresh, shutdown, and
resource profile lifetime/input/output/node/snapshot/retention ceilings. An explicit
`serve_with_ledger` caller owns the ledger across frames; the closed command
selects it from the first bounded request and then retains it. Responses are
exactly preflighted before allocation and publication; journaled publication
retains exclusion, rollback, descriptor anchoring, and conflict preservation. The former
Removed agent-foundation and Semantic Source identities are historical and
rejected. No generation-numbered agent schema is emitted.

Broader canonical source authoring beyond the implemented enum/match/control slices,
wider agent operations, nonzero incremental query caching, full-language
inhabitation or token masking, parser-preallocation before bounded source reads,
and a ledger shared onward into proof/artifact/runtime authorities remain
**Accepted Targets**. Unavailable HIR/SSA/layout/proof/native
correlations are explicit and revisioned rather than guessed. Unsupported
operations do not exist as inert endpoints.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each
capsule preserves one cohesive part of the accepted record. **Current** means
implemented and evidenced. **Accepted Target**, **Accepted Implementation
Contract**, and **Accepted Implementation Selection** are future contracts.
**Deferred** and **Rejected** remain non-current. A capsule cannot promote a
capability beyond the explicit status in its text.

## Strict Capsule Manifest

- [Identity, authority, and atomic edit model](semantic-source-and-agent-protocol/problem.md)
- [Canonical protocol](semantic-source-and-agent-protocol/protocol.md)
- [Canonical schema](semantic-source-and-agent-protocol/semantic-schema.md)
- [Typed holes and legal actions](semantic-source-and-agent-protocol/typed-holes-and-legal-actions.md)
- [Local session](semantic-source-and-agent-protocol/local-session.md)
- [Historical first candidate](../../history/platform/semantic-source/first-candidate.md)
- [Acceptance gates and deferred scope](semantic-source-and-agent-protocol/acceptance-gates.md)
