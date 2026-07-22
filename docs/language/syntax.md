# Syntax And Semantics

## Purpose

Define expression, type, and import behavior above the physical line format.

## Status

**Current.** The exact numeric contract is
[numeric-semantics.md](../decisions/numeric-semantics.md).

## Expressions

- Atoms are numbers, `true`, `false`, `nil`, or symbols.
- `str/` blocks produce `Str` values.
- Calls use matching open and close markers around child expressions.
- Division is named `div` because slash is structural.
- Evaluation is eager except for special forms that explicitly control it.
- Program definitions and mutable global values currently share one global
  namespace across imported files.

## Current Special Forms

Implemented control and binding forms include `def`, `fn`, `if`, `while`,
`let`, `bind`, `set`, `do`, and `quote`. `sig`, `params`, `forall`, `type`, and
`import` are contextual declaration/loading forms rather than freely evaluable
runtime calls.

Every function definition has a mandatory signature and typed parameters.
`forall/` declares annotation-driven type variables. There is no `Any`, trait,
typeclass, Hindley-Milner inference, or implemented user-defined type alias.

`set` currently mutates program-global state, but resolved HIR now requires an
existing mutable value target, assignable value type, and a runtime Nil result.
Global mutation itself remains temporary and is removed by the accepted local
`var`/`set` redesign.

## Accepted Semantic Redesign

The AI-first semantic core is an **Accepted Target**, not current syntax:

- remove generic `nil` and separate `Unit`/`unit`, typed empty lists, and
  explicit `Option T` values `some`/typed `none`;
- require exactly three operands for every `if` and exact reachable branch
  types;
- replace global mutation with immutable declarations and function-local
  `var`/`set`;
- replace top-level `do` with one explicit executable `main` and prohibit
  imported initialization effects;
- replace universal `eq` with typed value, object identity, structural list,
  and F64 bit comparisons;
- keep Option absence, Result failure, and process-safe VM Trap outcomes
  distinct.

See [AI-First Semantic Core](../decisions/semantic-core.md). Resolved typed HIR
is now the current boundary through which these forms will migrate, so typing
and lowering cannot interpret them differently.

## Numeric Contract

The canonical numeric types are only `I64` and `F64`. Integer and decimal
literal spelling is explicit and exact; malformed or out-of-range literals are
errors. Binary arithmetic is checked I64 arithmetic unless either operand is
F64, in which case IEEE-754 F64 behavior applies. F64 identity is preserved,
I64 values cover the complete signed 64-bit range, bitwise operations cover all
64 bits, and host narrowing is checked.

The canonical comparison names are `eq`, `ne`, `lt`, `lte`, `gt`, and `gte`.
I64 equality is exact and F64 equality follows IEEE rules rather than an
epsilon. See [Exact I64 And F64 Semantics](../decisions/numeric-semantics.md)
for literal grammar, promotion, overflow, representation, and removed
vocabulary.

## Files And Imports

Top-level forms are `def`, `do`, and `import`, bounded by
`MAX_TOPLEVEL_FORMS`.

Current path resolution supports:

- `std/...`, `lib/...`, and `examples/...` from package `src` trees;
- `./...` relative to the importing file;
- installed fallback through `LKJSCRIPT_ROOT` when a local category directory
  is absent.

Parent path components and absolute imports are rejected. Canonicalized paths
must remain inside the project or installed root, so symlink escapes fail.
Cycles are rejected and repeated canonical files are deduplicated. Definitions
are not namespaced modules yet.

Every entry and import must end in `.lkjscript`. `.lkjml`, extensionless source,
and unrelated extensions are rejected before parsing.

## Strings And Bytes

Strings are UTF-8 host strings but many operations index bytes. Arbitrary file
and network bytes do not currently have a complete round-trip-safe contract.
A distinct bulk byte path is deferred; current APIs must document byte indexing
and reject invalid UTF-8 boundaries rather than imply character indexing.
