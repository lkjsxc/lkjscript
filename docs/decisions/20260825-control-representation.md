# Compact semantic-change control representation

Date: 2026-08-25 UTC.

## Status

Partially implemented. Flat records, focused discovery, normalized plan/apply, connected creation,
flat expression edges, exact body replacement, compact output, predecessor JSON rejection, an
explicit normalized request codec, and label-independent allocation are public. Complete operation
coverage, external large-value inputs, direct common flags, and complete workflow measurements
remain required before the grammar is frozen.

## Problem

The predecessor `change` command required a recursive JSON request. The released command now uses
flat records and explicit typed hashing. The public compact subset remains narrower than the
transport-neutral authored operation enum.

The control representation must let an agent discover, author, repair, dry-run, and publish exact
semantic operations with bounded output. It is an ephemeral request representation, never accepted
program authority.

## Decision

- Use flat UTF-8 line records with no indentation semantics: one closed operation name followed by
  closed `field=value` assignments.
- Use dotted operation names from one executable registry. Reject unknown operations, fields,
  duplicate fields, implicit scalar types, and noncanonical escaping.
- Use one deterministic escaping rule shared with compact result records.
- Use `$name` for request-local semantic allocations and `@name` for notation-only type,
  expression, and member fragments. Symbol spelling is removed during normalization; typed ordinal
  references, not labels, enter the normalized request digest.
- Declare flat fragments in a mechanically resolvable order. A connected multi-owner change remains
  one request; expression and type trees do not require nested JSON braces.
- Parse independent physical UTF-8 records with input-byte, record, record-byte, field, name, value,
  fragment, operation, and diagnostic bounds. Adopt streaming input and explicit path-plus-digest
  values before workflows require payloads that should not be retained with the complete request.
- Keep typed selectors, operations, preconditions, normalized requests, plans, results, and
  diagnostics independent from tokenization, terminal rendering, serde tags, Rust enum order, and
  optional projections.
- Provide direct flags for common single operations, lowering to the same typed normalization and
  preparation path.
- Bind every plan and apply to the exact semantic base and normalized request digest. Physical
  package transport, idempotency, and operational output choices remain typed control fields but do
  not enter accepted semantic meaning.
- Retain JSON only as an explicit bounded projection for a demonstrated external integration. Do
  not retain JSON request authoring or generated schema without such a consumer.

## Corrections before public grammar freeze

- Completed: hash normalized authored intent through an explicit bounded codec rather than
  bincode, serde, or Rust layout.
- Completed: allocate by typed encounter ordinal rather than user label spelling or ordered-map
  lexical order.
- Completed: use fixed stable tags, big-endian lengths and integers, and typed fixed-width identity
  fields.
- Completed: replace `DeleteOwner { cascade: bool }` with closed reject-only leaf deletion.
  Owned-closure deletion remains absent until one exported typed impact plan binds every removed
  owner and relation.
- Remove physical semantic-root and derived-summary preconditions from caller intent.
- Completed: replace the scalar change-work maximum with independent typed budgets and
  observations.
- Author connected expression subtrees through flat fragments rather than a low-level single-node
  replacement field.

## Alternatives

- Strict JSON remains the measurement baseline, but its quoting, recursive shape, schema size, and
  repair locality make it unsuitable as the required path.
- A parenthesized typed notation remains a comparison candidate only if flat records cannot express
  nested fragments compactly.
- YAML and TOML were rejected because implicit typing, indentation/alias behavior, or mismatched
  recursive models add ambiguity without semantic benefit.
- A binary framing may serve a measured future resident transport, but agents will not be required
  to emit it directly.
- A source language, macro system, includes over ambient files, or full textual graph projection is
  rejected as a second authoring authority.

## Evidence required for acceptance

Compare strict JSON, flat records, direct flags, and the parenthesized alternative only where needed
for a connected creation, expression-heavy edit, structural refactor, stale-base repair, and
malformed-input repair. Retain request and command bytes, output bytes, commands, parse failures,
repair attempts, elapsed time, and implementation/help size. Provider tokens or monetary cost are
reported only from exact telemetry.

Public acceptance additionally requires focused registry discovery, deterministic plan/apply
responses, predecessor JSON rejection, external-file digest checks, bounded continuation, and a
copied-binary workflow that does not read repository source.

## Consequences

The private authored JSON protocol and schema were deleted. The executable registry owns public
names and forms. Parser source locations are retained only through planning and diagnostics.
Accepted graph bytes and revision identity are independent of the compact representation. Durable
allocation hashes only normalized authored intent plus repository identity. Authored record and
list order is allocation-significant, including where lowering stores keyed graph relations; this
avoids a hidden second traversal and keeps operation ordering locally repairable. The reviewed plan
also binds the exact multidimensional budget, idempotency key, and intent without making those
fields semantic meaning. Before grammar freeze, reverse this ordering choice only if complete
workflow evidence shows material identity churn from harmless model reordering; reversal requires
one explicit normalized request view and a new allocation-seed contract domain.

## Current evidence

- Compact parser unit tests cover connected typed lowering, out-of-order indexed edges, exact
  malformed-record locations, cycles, unused/shared expression rejection, and JSON rejection.
- Black-box copied-binary tests cover plan/apply allocation equality, reviewed-plan mismatch,
  connected module/record/function creation, 100-module bounded output, predecessor JSON rejection
  without HEAD movement, and complete old-body ownership retirement.
- Codec tests retain a golden authored-intent digest, prove label-renaming equality, budget
  separation, semantic-field and list-order sensitivity, undefined-symbol rejection, and depth
  exhaustion. Repository tests prove equal semantic roots and allocated identities across local
  label, admissible-budget, idempotency-key, and intent changes.
- The private JSON adapter, schema generator, repository convenience methods, and predecessor
  success tests are absent. Generated contract bytes come from the executable registry.

## Reversal condition

Delete or simplify the notation if complete measured workflows are materially worse than a smaller
typed representation. A future resident framing may be added only when repeated stateless workflow
measurements show a material end-to-end benefit; it must reuse the same normalized typed model.
