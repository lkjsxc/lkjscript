# Semantic Source And Agent Protocol

## Purpose

Define the primary manipulation boundary for complete and incomplete programs,
the deterministic Edition 1 projection adapter, and the smallest complete
agent-facing transaction/query slice.
## Status

<!-- LKJ-STATUS id=edition-2-identity-migration/1 status=current -->

**Current** for closed `lkjscript.semantic-source` version 2 over Edition 1 and
the Edition 2 identity/marker slice over existing declarations. Source-unit
records expose exact edition and edition-framed identity; snapshots expose tree
edition and identity; marker and edition-number nodes roundtrip strictly.
Schema identity remains version 2 rather than introducing an edition-specific
schema version.
Schema V2 preserves the exact V1 node, value, declaration, type, built-in,
trivia, transaction-expression, diagnostic, and correlation representation as
its base and adds typed-hole source identity and bounded legal actions. Version
1 input is historical and rejected; there is no alias, shim, or fallback.
Unknown kinds, fields, operations, versions, duplicates, and trailing input
fail. Inferred facts remain derived authority.

The bounded one-shot `snapshot`, `read_entity`, `query_node`, `diagnostics`,
`hole_context`, `legal_actions`, atomic `rename`, `replace_expression`,
`insert_hole`, `fill_hole`, `refine_hole`, and structurally legal `delete_hole`
operations emit V2. Hole context comes from the pinned parsed tree and a bounded
checker-valid completion; unavailable expected, capability, ownership-correlation,
qualification, and Edition 1 control facts are explicit rather than invented.
Profile V2 hole, legal-action, transaction, impact, and staged-publication
categories reserve before candidate or transaction staging allocation.

The Current local `semantic serve --stdio` session uses the same V2 engine with
exact framing, revision pinning, external-change rejection, refresh, shutdown,
and Profile V2 lifetime/input/output/node/snapshot/retention ceilings. Responses
are bounded before publication; journaled publication retains exclusion,
rollback, descriptor anchoring, and conflict preservation. The former
`lkjscript.agent-foundation/1` and `lkjscript.semantic-source/1` identities are
historical and rejected. `lkjscript.agent/1` is not emitted.

Edition 2 authoring, wider agent operations, nonzero incremental query caching,
full-language inhabitation or token masking, and a cross-authority shared ledger
remain **Accepted Targets**. Unavailable HIR/SSA/layout/proof/native
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
- [Complete Protocol V1 target](semantic-source-and-agent-protocol/protocol-v1.md)
- [Complete Schema V1](semantic-source-and-agent-protocol/complete-schema-v1.md)
- [Typed holes and legal actions](semantic-source-and-agent-protocol/typed-holes-and-legal-actions.md)
- [Local session](semantic-source-and-agent-protocol/local-session.md)
- [Historical Agent Foundation operations](semantic-source-and-agent-protocol/first-current-candidate.md)
- [Acceptance gates and deferred scope](semantic-source-and-agent-protocol/acceptance-gates.md)
