# Syntax And Semantics

## Purpose

Define expression, type, and import behavior above the physical line format.

## Status

**Current.** Executable roots use exactly one explicit `main`, imported files
are declaration-only, and mutation is limited to typed function-local
`var`/`set`. The exact numeric contract is
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
- Function and product declarations share one immutable program declaration
  namespace across imported files; source runtime globals do not exist.

## Current Special Forms

Implemented control and binding forms include `main`, `def`, `fn`, `if`,
`while`, `let`, `bind`, `var`, `set`, `do`, `quote`, `product-value`, `field`,
and `with-field`. `product`, `fields`, `sig`, `params`, `forall`, `type`,
`name`, and `import` are contextual declaration/loading forms rather than
freely evaluable runtime calls.

Every function definition has a mandatory signature and typed parameters.
`forall/` declares annotation-driven type variables. There is no `Any`, trait,
typeclass, Hindley-Milner inference, or implemented user-defined type alias.

`if` requires exactly three operands: a Bool condition and two branches with
exactly the same type. There is no omitted branch or nil-based type join.
Empty `do`, `while`, `set`, and side-effecting operations return Unit.

`var/ name/ x /name type/ T /type initial body /var` introduces one typed
mutable local. The initializer is resolved and evaluated before the binding
enters scope. In the body, `set/ x value /set` targets only the nearest `var`
in the same main or function, requires exact type equality, and returns Unit.
Parameters, `let` bindings, functions, globals, and bindings in another
function cannot be assigned. No source global or compatibility form remains.

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

The implemented AI-first semantic slices include dedicated `Unit`/`unit`,
strict three-arm `if`, typed empty lists, Option/no-nil semantics, explicit
equality families, immutable nominal products, explicit main, effect-free
imports, and local-only mutation. `arg` returns `Option Str`; negative or
out-of-range indexes return none. Product-valued editor, terminal, and
Brainfuck state is passed through helpers and evolved only by executable or
function-local vars. Process-safe structured VM outcomes remain an
**Accepted Target**.

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

Imported files contain only `import`, immutable function `def`, and `product`
declarations. An executable root contains those declarations plus exactly one
`main/ sig/ -> T /sig body-expression /main`. Main has no parameters, its body
has exactly `T`, and `arg` remains the script-argument operation. A main in an
import, no root main, a duplicate root main, top-level `do`, and non-function
`def` are compile errors. All source files remain bounded by
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
