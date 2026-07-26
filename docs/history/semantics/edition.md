# Edition 2 Semantic Core

## Purpose

Define the accepted Edition 2 source, type, control, value, layout, error,
compiler, execution, migration, and semantic-authoring contracts before any of
them become implementation claims.

## Status


**Accepted Target overall.** Explicit identity, compiler-owned atomic
migration, generic enum declaration/resolved-type-fact, construction,
exhaustive match, Never/structured-control, explicit numeric conversion, the
125-file corpus, and ordinary-compilation cutover are Current through the
evaluator, reference VM, and forced Linux x86-64 baseline/proof JIT.
Automatic/host-native enum transitions, changed semantics beyond these slices,
and protocol-level semantic migration remain non-Current. Edition 1 is accepted
only by explicit source-validation/migration APIs and immutable fixture data.

## Authority

This page is the authority for Edition 2 and its strict capsule manifest.
Capsules may refine this contract but cannot promote it. Edition 2 is explicit
per source unit: the first semantic form is exactly `edition/`, atom `2`, and
`/edition`. Every unit in a loaded closure must agree. Explicit validation treats a file
without that form as Edition 1 migration input; ordinary compilation rejects it. The Semantic Source identity is
`lkjscript.semantic-source/2`; the public ABI changes only where public Edition
2 semantics require it.

Semantic Source remains primary. Edition 1 and Edition 2 share one source
parser and validated tree plus deterministic edition projections; a second
parser/tree is forbidden. The current line projection remains canonical during
this target. Edition 2 now accepts the complete generic `enum` declaration, resolved type
facts, and exact `variant-value` construction under exactly those names, with
no aliases. Construction executes only through verified SSA consumers: its evaluator,
validated reference bytecode/VM, and forced Linux x86-64 baseline/proof JIT.
Polymorphic entries, host operations, and automatic reference transfer reject.

## Strict Capsule Manifest

1. [Research inputs](edition/research-inputs.md)
2. [Identity and migration](edition/identity-and-migration.md)
3. [Algebraic data types](../../decisions/semantics/algebraic-data-types.md)
4. [Patterns and match](../../decisions/semantics/patterns-and-match.md)
5. [Never and control](../../decisions/semantics/never-and-control.md)
6. [Value, effects, and metering](edition/value-effects-and-metering.md)
7. [Layouts](edition/layouts.md)
8. [Numeric conversions](../../decisions/semantics/numeric-conversions.md)
9. [Typed errors](edition/typed-errors.md)
10. [HIR and SSA](edition/hir-and-ssa.md)
11. [Execution and acceptance](edition/execution-and-acceptance.md)
12. [Semantic authoring](edition/semantic-authoring.md)

The manifest is closed and ordered. Every listed capsule is required, and no
unlisted file is part of this authority.

## Cross-Authority Dependencies

Edition 2 depends on [Semantic Source and Agent
Protocol](../../decisions/platform/semantic-source-and-agent-protocol.md) Schema V1 closure
and the [Resource Budget Profiles](../../decisions/platform/resource-budget-profiles.md)
Profile V2 pre-allocation contract. Current source limits and execution
behavior remain unchanged until the acceptance capsule's gates all pass.

## Rejected

- implicit edition selection or mixed-edition closures;
- `.lkjml`, compatibility aliases, sibling source parsers, or backend syntax;
- status language that calls an Edition 2 contract Current before evidence; and
- an ABI version change made only because the source edition number changed.
