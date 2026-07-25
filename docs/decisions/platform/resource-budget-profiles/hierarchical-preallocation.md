# Resource Profiles: Hierarchical Preallocation

[Authority](../resource-budget-profiles.md)

## Purpose

Define one request-owned checked ledger, lower-only child grants,
pre-allocation accounting, failed prefixes, and closed Profile V2 categories.

## Status

**Accepted Target, not Current.** Current Profile V1 post-phase compiler and
one-shot protocol ledgers remain separate and unchanged.

## One Ledger

Each compile, semantic request, transaction, proof, artifact build, or execution
begins with one request-owned host ledger. Typed child scopes cover source
loading, parsing, schema validation, diagnostics, resolution, type/trait and
ownership analysis, HIR/effects, pattern usefulness/lowering, SSA construction
and verification, normalization, proof discovery/checking, bytecode, native
lowering, protocol decode/encode, holes, session caching, and repository
queries invoked by that request.

A child receives category-specific grant no larger than its parent's remaining
grant. It may lower but never raise or relabel a ceiling, create an unmetered
sibling, or return more grant than it received. Checked addition and conversion
precede reservation and allocation.

Before allocation or amplification, an authority reserves the worst-case
bounded category charge. Successful construction commits exact consumed units
and releases the unused reservation. Failure releases unconsumed reservation
but retains completed semantic/work charges. Reservation identity, owner,
category, amount, semantic cause, parent grant, and state are explicit; double
commit/release and cross-owner use fail closed.

A failed response includes the deterministic ledger prefix: profile identity,
all committed charges and reservation transitions, and the rejected event's
category, unit, ceiling, prior charge, attempted increment, semantic node,
phase, allocation-before-rejection flag, and child-scope path. It cannot claim
totals for work that did not occur or expose noncanonical host paths.
Publication is staged only within an already reserved bound and is atomic after
validation.

Source bytes reserve token/line space before lexing; decoded lengths reserve
collections before growth; pattern rows/columns reserve specialization;
diagnostics, hole candidates, SSA IDs, response bytes, enum metadata, and
staged publication bytes reserve before accumulation or copying.

## Closed Profile V2 Categories

Profile V2 has identity `lkjscript.resource-profile/2`. It retains all 25 V1
categories and appends these distinct closed categories:

```text
enum_declarations enum_variants variant_fields enum_recursion_work
patterns match_arms usefulness_rows usefulness_columns
usefulness_specialization_work match_plans exhaustiveness_witness_bytes
hole_count hole_candidates hole_search_work legal_actions
semantic_session_lifetime_fuel semantic_session_input_bytes
semantic_session_output_bytes semantic_session_nodes
semantic_session_snapshots semantic_session_retained_bytes
semantic_session_cache_entries semantic_session_cached_revisions
transactions transaction_operations transaction_impact_nodes
staged_publication_bytes staged_publication_nodes
logical_aggregate_constructions
```

Unknown categories require Profile V3. Names cannot alias or overload V1.
`*_bytes` use exact bytes; `*_work` and lifetime fuel use deterministic work
units; logical constructions use semantic events; all others use records.
Concrete ceilings require retained corpus/adversarial measurements and exact
boundary tests before V2 can be Current.

## Edition 1 Boundary

All Edition 1 per-file, form, product-field, and source-directory limits remain
Current until these replacement charges are pre-allocation Current across the
complete amplification path. Profile selection cannot weaken implementation
maxima or those limits.
