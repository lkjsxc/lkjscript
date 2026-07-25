# Semantic Source And Agent Protocol

## Purpose

Define the primary manipulation boundary for complete and incomplete programs,
the deterministic Edition 1 projection adapter, and the smallest complete
agent-facing transaction/query slice.
## Status

**Current** for complete `lkjscript.semantic-source` version 1 over Edition 1:
one opaque validated source authority; closed typed node, value, declaration,
type, built-in, trivia, transaction-expression, diagnostic, and correlation
schemas; exact source origins/spans/revisions; stable declaration keys; dense
revision-scoped nodes; deterministic schema/source roundtrip; and all 125
tracked sources. Unknown kinds, fields, operations, versions, duplicates, and
trailing input fail. Inferred facts remain derived authority.

The bounded one-shot `snapshot`, `read_entity`, `query_node`, `diagnostics`,
atomic `rename`, and atomic `replace_expression` operations now emit that
identity. Responses are bounded before publication; journaled local publication
retains exclusion, rollback, descriptor anchoring, and conflict preservation.
The former `lkjscript.agent-foundation/1` identity is historical and is rejected
rather than retained as an alias. `lkjscript.agent/1` is not emitted.

`lkjscript.semantic-source/2`, Edition 2 authoring, typed holes/legal actions,
the local stdio session, wider agent operations, and pre-allocation metering
remain **Accepted Targets**. Unavailable HIR/SSA/layout/proof/native correlations
are explicit and revisioned rather than guessed. Unsupported operations do not
exist as inert endpoints.

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
