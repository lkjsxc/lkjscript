# Public task/capability authoring and replaceable storage seam

Status: accepted for compact change contract 5 and CLI contract 12.

## Context

The Graph 5 engine and Artifact 10 runtime could already execute request-dependent HTTP tasks,
strict JSON, lexical database transactions, and PostgreSQL-backed applications. The compact public
writer exposed only a smaller pure-expression slice, while built-in and deployment inspection did
not provide the exact references and fields needed to compose that behavior outside the checkout.
A real BBS consumer therefore required a second HTTP/storage helper despite the existing runtime.

## Decision

Expose the smallest dependency-closed existing authored slice needed by that consumer:

- exact task effects, existing-component requirements, and function-contract updates;
- structural records, lexical bindings, records/fields/lists, variants/matches, exact built-in
  calls, requirement-scoped capability calls, and lexical transactions; and
- bounded executable-owned built-in owner discovery and deployment schema discovery.

The maintained BBS starts from the closed `http` recipe and creates all additional application
meaning through reviewed compact plan/apply. Its HTTP and domain functions depend on narrow
application-owned persistence functions. The graph owns schema, parameterized statements,
migration identity/checksum, row conversion, and transaction policy; deployment owns adapter
selection, connection secret, pool/timeouts, and listener/runtime limits.

SQL and PostgreSQL are current mechanisms, not language semantics. No BBS template, frozen artifact,
ambient IO, private graph builder, arbitrary topology creation, or second application process is a
supported path.

## Consequences and reversal

Exact task/capability closure is now part of the public authored contract and must remain
discoverable and review-bound. Adding other expression or topology forms requires another
maintained consumer and dependency-closed cutover. The BBS and `lkjournal` become measured workloads
for the next storage decision.

A future first-party semantic store may replace PostgreSQL only after defining provider-independent
logical operations and durability/recovery behavior, comparing against an independent PostgreSQL
oracle, migrating every maintained consumer, rejecting predecessor configuration, and deleting
SQL-specific application meaning and permanent dual production paths.
