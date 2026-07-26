# Resource Profiles: Hierarchical Preallocation

[Authority](../resource-budget-profiles.md)

## Purpose

Define one request-owned checked ledger, lower-only child grants,
pre-allocation accounting, failed prefixes, and closed Profile V2 categories.

## Status

**Current core foundation and deterministic journal; whole-pipeline migration
incomplete.** Profile V2's closed categories, named ceilings, bounded authority
paths, lower-only child grants, move-only pre-allocation reservations, and fixed
nonallocating deterministic ledger journal are Current in `lkjscript-core`.
Compiler enum shape and match pattern/arm/matrix/plan/witness categories reserve
from validated source shape before HIR allocation. Immutable HIR reserves its
exact charged input shape before SSA construction, and immutable normalized SSA
reserves its exact charged input shape before bytecode construction. Public compiler and Semantic Source
`_with_ledger` entry points share one
outer-owned ledger through strict protocol decode, typed request execution,
hole exploration, legal actions, transaction staging, Edition 2 migration
staging, and exact response preflight. Local sessions invoke the typed one-shot
engine directly rather than serializing and decoding an inner request/response;
an explicitly supplied session ledger is retained across frames. Source loading
still allocates behind the bounded Foundation V1 reader before its exact loaded
shape is reserved, and complete cross-authority pre-allocation remains an
Accepted Target.

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

The closed authorities are `compile_request`, `semantic_request`, `transaction`,
`proof`, `artifact_build`, `execution`, `source_loading`, `parsing`,
`schema_validation`, `diagnostics`, `resolution`, `type_analysis`,
`trait_analysis`, `ownership_analysis`, `hir`, `effects`,
`pattern_usefulness`, `pattern_lowering`, `ssa_construction`,
`ssa_verification`, `normalization`, `proof_discovery`, `proof_checking`,
`bytecode`, `native_lowering`, `protocol_decode`, `protocol_encode`, `holes`,
`session_caching`, and `repository_queries`. Authority paths contain one through
16 entries in a fixed nonallocating representation. Missing authority and a
seventeenth entry fail closed.

Before allocation or amplification, an authority reserves the worst-case
bounded category charge. Callers consume exact completed units and explicitly
return unused units after success or known failure. Dropping an active token
conservatively commits its remainder, so forgotten return cannot recreate
budget. Reservation identity, owner, category, amount, semantic cause, parent
grant, and state are explicit; move semantics prevent double return/commit and
cross-owner use.

The Current core journal has a fixed capacity of 256 reservation records and
uses no heap allocation. Journal capacity is checked before a reservation ID is
issued or category grant is changed: the 256th reservation succeeds and the
257th rejects without mutation or allocation. IDs are monotonic and never
reused. ID exhaustion, journal exhaustion, consume overrun, invalid child grant,
and path overflow reject without mutation.

Every precheck rejection owns an immutable deterministic prefix containing the
Profile V2 identity, aggregate committed totals in closed category order, every
prior reservation in ID order, and the exact rejected event. Each reservation
record contains identity, owner path, category, semantic cause, amount,
consumed amount, explicitly returned amount, and whether Drop conservatively
committed its remainder. Prefixes taken in nested scopes include completed
ancestors and prior siblings plus current work exactly once. Missing authority
returns the same prefix shape without allocation. Rejected events contain kind,
category when applicable, ceiling and prior/increment amounts when applicable,
semantic cause, owner path, and `allocated_before_rejection=false` for all core
prechecks. The existing post-phase `ResourceDiagnostic` is explicitly legacy,
does not carry this prefix, and makes no pre-allocation claim.

A prefix cannot claim totals for work that did not occur or expose noncanonical
host paths. Publication is staged only within an already reserved bound and is
atomic after validation.

The accepted complete design requires source bytes to reserve token/line space
before lexing; that source-parser reservation is not Current. Current compiler
source bytes/tokens/nodes are measured after the fixed-limit parser and before
HIR. Pattern rows/columns reserve usefulness work, immutable HIR shape reserves
before SSA construction, and immutable normalized SSA shape reserves before
bytecode construction. Diagnostics, hole candidates, legal actions, protocol request/response bytes,
session frame bytes, enum metadata, and staged publication bytes reserve at
their documented Current boundaries. Protocol response size is first measured
with a nonallocating writer until the self-reported exact byte charge stabilizes;
that exact size is reserved before `Vec` capacity. Schema V2 cannot add a
structured budget-prefix field: the `_with_ledger` typed result retains the
exact `BudgetError`, while V2 wire errors retain its deterministic textual
rendering only. A typed prefix field requires Schema V3.

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
The appended ceilings are exact inclusive sandbox ceilings below.
`deterministic`, `default`, `build`, and `trusted-local` use respectively
4, 8, 32, and 64 times each value, preserving a positive monotonic order.

| categories | sandbox ceiling |
| --- | ---: |
| enum_declarations | 1,024 |
| enum_variants | 4,096 |
| variant_fields | 16,384 |
| enum_recursion_work | 65,536 |
| patterns | 16,384 |
| match_arms | 8,192 |
| usefulness_rows, usefulness_columns | 32,768 |
| usefulness_specialization_work | 1,000,000 |
| match_plans | 8,192 |
| exhaustiveness_witness_bytes | 1,048,576 |
| hole_count | 1,024 |
| hole_candidates | 16,384 |
| hole_search_work | 1,000,000 |
| legal_actions | 65,536 |
| semantic_session_lifetime_fuel | 1,000,000 |
| semantic_session_input_bytes, semantic_session_output_bytes | 4,194,304 |
| semantic_session_nodes | 65,536 |
| semantic_session_snapshots | 256 |
| semantic_session_retained_bytes | 4,194,304 |
| semantic_session_cache_entries | 1,024 |
| semantic_session_cached_revisions | 256 |
| transactions | 1,024 |
| transaction_operations | 8,192 |
| transaction_impact_nodes | 32,768 |
| staged_publication_bytes | 4,194,304 |
| staged_publication_nodes | 65,536 |
| logical_aggregate_constructions | 1,000,000 |

These ceilings establish the core contract; they do not claim that existing
pipelines reserve every allocation or share one ledger.

## Edition 1 Boundary

All Edition 1 per-file, form, product-field, and source-directory limits remain
Current until these replacement charges are pre-allocation Current across the
complete amplification path. Profile selection cannot weaken implementation
maxima or those limits.
