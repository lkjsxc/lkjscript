# Edition 2 Semantic Core

## Purpose

Define the accepted Edition 2 source, type, control, value, layout, error,
compiler, execution, migration, and semantic-authoring contracts before any of
them become implementation claims.

## Status

<!-- LKJ-STATUS id=edition-2-identity-migration/1 status=current -->
<!-- LKJ-STATUS id=edition-2-enum-declarations/1 status=current -->

**Accepted Target overall.** Explicit identity/non-publishing migration and the
generic enum declaration/resolved-type-fact slice are Current. Enum value
construction, patterns, changed semantics, backend execution, semantic
publication, corpus migration, and cutover are not Current. Edition 1 remains
accepted for ordinary compilation and as migration input.

## Authority

This page is the authority for Edition 2 and its strict capsule manifest.
Capsules may refine this contract but cannot promote it. Edition 2 is explicit
per source unit: the first semantic form is exactly `edition/`, atom `2`, and
`/edition`. Every unit in a loaded closure must agree. A file without that form
is Edition 1 until cutover. The Semantic Source identity is
`lkjscript.semantic-source/2`; the public ABI changes only where public Edition
2 semantics require it.

Semantic Source remains primary. Edition 1 and Edition 2 share one source
parser and validated tree plus deterministic edition projections; a second
parser/tree is forbidden. The current line projection remains canonical during
this target. Edition 2 now accepts the complete generic `enum` declaration and
resolved type-fact contract under exactly that name, with no alias. It does not
expose enum values or backend operations.

## Strict Capsule Manifest

1. [Research inputs](edition-2/research-inputs.md)
2. [Identity and migration](edition-2/identity-and-migration.md)
3. [Algebraic data types](edition-2/algebraic-data-types.md)
4. [Patterns and match](edition-2/patterns-and-match.md)
5. [Never and control](edition-2/never-and-control.md)
6. [Value, effects, and metering](edition-2/value-effects-and-metering.md)
7. [Layouts](edition-2/layouts.md)
8. [Numeric conversions](edition-2/numeric-conversions.md)
9. [Typed errors](edition-2/typed-errors.md)
10. [HIR and SSA](edition-2/hir-and-ssa.md)
11. [Execution and acceptance](edition-2/execution-and-acceptance.md)
12. [Semantic authoring](edition-2/semantic-authoring.md)

The manifest is closed and ordered. Every listed capsule is required, and no
unlisted file is part of this authority.

## Cross-Authority Dependencies

Edition 2 depends on [Semantic Source and Agent
Protocol](../platform/semantic-source-and-agent-protocol.md) Schema V1 closure
and the [Resource Budget Profiles](../platform/resource-budget-profiles.md)
Profile V2 pre-allocation contract. Current source limits and execution
behavior remain unchanged until the acceptance capsule's gates all pass.

## Rejected

- implicit edition selection or mixed-edition closures;
- `.lkjml`, compatibility aliases, sibling source parsers, or backend syntax;
- status language that calls an Edition 2 contract Current before evidence; and
- an ABI version change made only because the source edition number changed.
