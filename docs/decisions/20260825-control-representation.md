# Compact semantic-change control representation

Date: 2026-08-25 UTC.

## Status

Proposed. The current typed operation inventory and JSON baseline are audited. The streaming parser,
transport-neutral normalization, complete workflow comparison, and direct public cutover remain to
be implemented and measured.

## Problem

The released `change` command currently requires a recursive JSON request. The normalized authored
operation enum is broader and typed, but its private protocol still derives JSON shape and schema
from Rust/serde representation. Expression construction is recursively nested, request hashing is
performed before semantic normalization, local-symbol spelling and lexical map order influence
allocation, and one malformed nested value has poor shell and repair locality.

The control representation must let an agent discover, author, repair, dry-run, and publish exact
semantic operations with bounded output. It is an ephemeral request representation, never accepted
program authority.

## Proposed decision

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
- Parse incrementally from `BufRead` with independent input-byte, record, field, list, string,
  fragment, operation, and diagnostic bounds. Large text and bytes use explicit file path plus
  digest.
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

## Corrections required before public grammar freeze

- Hash the semantic normalized request, not its bincode/serde representation.
- Make allocation traversal independent from user label spelling and ordered-map lexical order.
- Replace implicit Rust enum-layout tags with explicit stable operation and form tags.
- Replace `DeleteOwner { cascade: bool }` with a closed delete policy, initially `reject` and later
  an exact reviewed repair or closure plan.
- Remove physical semantic-root and derived-summary preconditions from caller intent.
- Replace the scalar change-work maximum with independent typed budgets and observations.
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

The private authored JSON protocol and schema are transitional and will be deleted. The typed
operation registry becomes the sole owner of public names and forms. Parser source locations are
retained only through planning and diagnostics. Accepted graph bytes and revision identity remain
independent of line spelling, whitespace, comments, output format, and future transports.

## Reversal condition

Delete or simplify the notation if complete measured workflows are materially worse than a smaller
typed representation. A future resident framing may be added only when repeated stateless workflow
measurements show a material end-to-end benefit; it must reuse the same normalized typed model.
