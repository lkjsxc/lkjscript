# Review-bound logical change plan

Date: 2026-08-26 UTC.

## Status

Implemented for normalized public `change plan` and `change apply`.

## Problem

The predecessor `plan_` value committed only to normalized authored request control. Reviewers
could see predicted aggregate counts, but apply did not prove that the prepared semantic effects,
exact relation delta, validation frontier, selected tests, or reasons still matched what was
reviewed. Complete detail can exceed the finite compact stdout envelope.

## Decision

- Use one canonical token with two 32-byte components: a request commitment checked before project
  discovery and a prepared-plan commitment checked after re-preparation and before publication.
- Construct one typed logical plan from the exact authored preparation analysis. It contains
  normalized control, exact semantic effects, and exported validation/test scope; it is not a
  second diff, relation, ownership, or impact engine.
- Hash and optionally write the same canonical compact-record traversal. The final non-hashed
  trailer carries both commitments and the complete token.
- Keep the optional plan file outside the project as bounded, atomic, non-authoritative review
  evidence. Apply never accepts it as input and always recomputes from accepted authority.
- Exclude witness edit encoding, summary refresh, compiler-unit selection, cache/storage layout,
  staged objects, receipt work, timing, paths, and request-local labels. Those facts can change
  without changing the reviewed logical result.
- Preserve generic publication's independence from authored control. Authored preparation returns
  logical evidence beside the prepared publication, and `GraphRepository::publish` remains the
  sole normal HEAD visibility boundary.

## Alternatives rejected

A request-only token leaves the correctness gap. Hashing generic prepared-publication bytes would
bind replaceable physical and scheduling choices. Complete inline stdout would either exceed its
4 MiB/10,000-record boundary or require truncation. Importing a reviewed file during apply would
create another writer/authority path. A persisted plan cache, session, daemon, JSON projection, or
compatibility parser has no current consumer and would add state or dual contracts.

## Consequences

Changing any exported logical record changes the prepared component. Harmless local-symbol
renaming, output location, or excluded operational work does not. Apply can reject a request
mismatch without project I/O and distinguishes it from a prepared-plan mismatch. Plan output can
remain after interruption without implying accepted publication. Public owned-closure deletion now
consumes this boundary: its exact union of removed owners, retirements, surviving-parent edits,
relations, validation owners, tests, and reasons is represented by logical-plan contract 1 and
recomputed before apply.

## Reversal conditions

Replace the compact file framing only if a maintained review consumer demonstrates that it cannot
meet bounded streaming or strict typed decoding requirements. A replacement must retain the same
logical/operational boundary, direct/record equality, pre-publication recomputation, complete
before/after oracles, and one current contract. Adding signatures, provenance, or remote approval
requires a separately selected trust boundary and does not turn a plan file into program authority.
