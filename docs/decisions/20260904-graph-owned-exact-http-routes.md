# Graph-owned exact HTTP routes

- Status: accepted
- Date: 2026-09-04
- Decision owner: exact inbound HTTP route topology

## Decision

An inbound `http` target owns a finite nonempty set of stable route owners. Each route stores one
exact method/path key and one function-backed port belonging to the target component. HTTP targets
have no universal port; non-HTTP targets retain one exact port. Compilation derives a canonical
table, and the runtime performs one bounded exact lookup before admitting the selected handler.

## Rationale

A universal target port makes application functions rebuild topology as ordinary branching. That
hides route identity from authoring, review, impact, inspection, validation, and compilation; it
also makes duplicate keys and no-effect unknown-route behavior impossible to enforce mechanically.
Graph-owned route records contract this authority while keeping transport validation and lookup as
generic Rust mechanisms and keeping authorization and domain behavior in graph meaning.

## Consequences

Routes can be added, changed, inspected, and deleted through the ordinary exact-base workflow.
Duplicate or malformed keys reject before publication, artifacts bind deterministic route tables,
and an unmatched valid request has a fixed empty 404 path that cannot invoke effects or create a
resident task. Deployment cannot override routing. Existing predecessor target and artifact forms
are rejected rather than migrated or hidden behind a fallback.

## Reversal condition

Reconsider parameterized paths, precedence, or middleware only when a maintained workload cannot
be represented as a bounded finite exact route set and a proposal supplies deterministic graph
semantics, explicit effect ordering, bounded public authoring, dependency-closed migration and
deletion, and independent dispatch proof. A universal fallback port is not a reversal path.
