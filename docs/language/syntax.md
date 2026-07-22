# Syntax And Semantics

## Purpose

Define expression, type, and import behavior above the physical line format.

## Status

**Current.** The exact numeric contract is
[numeric-semantics.md](../decisions/numeric-semantics.md).

## Expressions

- Atoms are numbers, `true`, `false`, `unit`, or symbols. `nil` is removed.
- `Unit` has exactly one value, `unit`, for completion without useful data.
- `empty-list/ T /empty-list` creates the empty `List T`; `empty-list?` is its
  only predicate. `null?` is removed.
- `some/ value /some` constructs `Option T`; `none/ T /none` constructs typed
  absence. `is-some` is the positive predicate and `unwrap-some` is explicit.
- `str/` blocks produce `Str` values.
- Calls use matching open and close markers around child expressions.
- Division is named `div` because slash is structural.
- Evaluation is eager except for special forms that explicitly control it.
- Program definitions and mutable global values currently share one global
  namespace across imported files.

## Current Special Forms

Implemented control and binding forms include `def`, `fn`, `if`, `while`,
`let`, `bind`, `set`, `do`, `quote`, `product-value`, `field`, and
`with-field`. `product`, `fields`, `sig`, `params`, `forall`, `type`, `name`,
and `import` are contextual declaration/loading forms rather than freely
evaluable runtime calls.

Every function definition has a mandatory signature and typed parameters.
`forall/` declares annotation-driven type variables. There is no `Any`, trait,
typeclass, Hindley-Milner inference, or implemented user-defined type alias.

`if` requires exactly three operands: a Bool condition and two branches with
exactly the same type. There is no omitted branch or nil-based type join.
Empty `do`, `while`, `set`, and side-effecting operations return Unit.

`set` currently mutates program-global state, but resolved HIR now requires an
existing mutable value target, assignable value type, and returns Unit. Global
mutation itself remains temporary and is removed by the accepted local
`var`/`set` redesign.

## Immutable Nominal Products

A top-level `product` declares from zero through 15 ordered, uniquely named,
explicitly typed fields. Product names begin with ASCII uppercase and continue
with ASCII letters, digits, or hyphens. The annotation `Product Name` identifies
exactly that declaration; same-shaped declarations are distinct types. Declarations add
metadata but no runtime global or initializer.

`product-value` supplies every field once in declaration order. `field` reads a
statically named field. `with-field` returns a newly allocated product with one
field replaced and leaves the original unchanged. Missing, extra, duplicate,
out-of-order, unknown, or wrongly typed fields are compile errors. Products do
not participate in any current equality family. The exact forms and examples
are in [Immutable Nominal Products](../decisions/immutable-nominal-products.md).

## Semantic Migration

The first AI-first semantic slices are **Current**: dedicated `Unit`/`unit`,
strict three-arm `if`, typed empty lists, Option/no-nil semantics, explicit
equality families, and immutable nominal products are enforced from source
through HIR and VM. `arg` returns
`Option Str`; negative or out-of-range indexes return none. The remaining
semantic core is an
**Accepted Target**:

- replace global mutation with immutable declarations and function-local
  `var`/`set`;
- replace top-level `do` with one explicit executable `main` and prohibit
  imported initialization effects;
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

The canonical numeric ordering names are `lt`, `lte`, `gt`, and `gte`, using
the arithmetic I64/F64 promotion rule. Equality never promotes: `equal-value`
requires identical supported types and gives exact I64 or IEEE F64 equality;
`f64-bits-equal` compares exact F64 bits. `same-object` is Buf/Handle identity,
and `list-equal` is bounded structural equality. There is no negative alias;
use `not` around a positive operation. See
[Exact I64 And F64 Semantics](../decisions/numeric-semantics.md) and
[Explicit Equality Families](../decisions/equality-families.md).

## Files And Imports

Top-level forms are `def`, `do`, `import`, and `product`, bounded by
`MAX_TOPLEVEL_FORMS`. Product declarations are effect-free metadata; arbitrary
runtime global value definitions and top-level `do` remain current only until
the accepted explicit-main cutover.

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
