# Syntax And Semantics: Expressions

[Authority](../syntax.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

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
and `with-field`. the canonical source contract additionally implements typed `loop`, early
`return`, nearest-loop `break`/`continue`, value-bearing `trap`, and `exit`.
`product`, `trait`, `impl`, `fields`, `sig`, `params`,
`forall`, `bounds`, `bound`, `type`, `name`, and `import` are contextual
declaration/loading forms rather than freely evaluable runtime calls.

Every function definition has a mandatory signature and typed parameters.
`forall/` declares annotation-driven type variables. The Current trait slice is
bounded declaration-only markers described below. There is no `Any`,
Hindley-Milner inference, or implemented user-defined type alias.

`if` requires exactly three operands: a Bool condition and two reachable
branches with exactly the same type. In the canonical source contract, a divergent `Never` arm
joins only with the surviving arm type and creates no value. There is no
omitted branch or nil-based type join. Empty `do`, `while`, `set`, and
side-effecting operations return Unit.

The canonical typed loop is `loop/ type/ T /type body... /loop`.
`break/ value /break` must exactly match `T`; `continue/ /continue` targets the
nearest loop. `while` remains Unit and accepts only `break/ unit /break`.
`return/ value /return` exactly matches the containing return type,
`trap/ Str /trap` preserves that value as the trap message, and
`exit/ I64 /exit` produces a structured exit outcome. Every transfer has type
`Never`; unreachable sequence elements are rejected.

`var/ name/ x /name type/ T /type initial body /var` introduces one typed
mutable local. The initializer is resolved and evaluated before the binding
enters scope. In the body, `set/ x value /set` targets only the nearest `var`
in the same main or function, requires exact type equality, and returns Unit.
Parameters, `let` bindings, functions, globals, and bindings in another
function cannot be assigned. No source global or compatibility form remains.
## Initial Owned Buffer Safe Island

The only ownership types currently accepted are exact `Owned Buf`, `Ref Buf`,
and `RefMut Buf`. `Owned Buf` is permitted for locals, parameters, and returns.
References are permitted only as function parameters and inferred local borrow
bindings; they cannot be returned by source functions or user calls or placed
in products, List, Option, or Result. Those aggregate storage positions reject
both direct and nested ownership/reference types. Parameter annotations use the
marked form `Owned/ Buf /Owned` (and the corresponding `Ref/` or `RefMut/`
form); signatures and `type/` forms use the atoms `Owned Buf`, `Ref Buf`, or
`RefMut Buf`.

`owned-buf-new` creates the only source of `Owned Buf`. Reads use
`owned-buf-len Ref Buf` and `owned-buf-ref Ref Buf I64`; writes use
`owned-buf-set RefMut Buf I64 I64`. `move/ local-name /move` consumes one whole
owned local or parameter. `borrow/ local-name /borrow` and
`borrow-mut/ local-name /borrow-mut` borrow one whole `Owned Buf` place. A
Borrow expression is legal only as an exact direct reference argument or as a
direct `let` initializer. A temporary direct-argument loan ends after the
complete call/runtime-operation expression, not after evaluation of that one
argument. Same-basic-block last-use analysis ends local loans non-lexically;
Borrow results do not cross SSA blocks. Legacy `Buf` operations and semantics
are unchanged.

Only Owned function parameters are initialized at entry. A local place becomes
initialized after its initializer succeeds and leaves ownership/loan state at
lexical scope exit, so branch-local places do not enter branch joins. `var`
reinitialization is accepted only after a move, from a fresh or explicitly moved
`Owned Buf`.

Every generic instantiation involving a direct or nested `Owned`, `Ref`, or
`RefMut` signature/substitution is unavailable. Arbitrary or nested borrow
expressions, legacy-Buf conversion, fields/indexes, reborrowing, partial moves,
reference return/storage, closures/capture, cross-block Borrow results,
loop-carried ownership state or loans, and `RefMut` user-call forwarding are
rejected. Runtime/frame cleanup is not user-visible deterministic `Drop`.
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
are in [Immutable Nominal Products](../../decisions/semantics/immutable-nominal-products.md).
## Marker Traits And Bounds

A marker declaration contains exactly one name:

```text
trait/
name/
Serializable
/name
/trait
```

An implementation contains exactly one resolved marker trait and one exact
nominal product target:

```text
impl/
trait/
Serializable
/trait
for/
Product
Record
/for
/impl
```

A generic function may place one `bounds/` child after `forall/` and before
`sig/`; every child is exactly `bound/ T Serializable /bound`. The complete
loaded source closure is the temporary coherence domain and permits at most one
implementation for a trait/product pair. Bounds are solved at each concrete
direct call and retained as verified HIR/SSA witness identities.

`Copy`, `Clone`, `Drop`, `Send`, and `Sync` are compiler-owned names. Source
cannot implement them in this marker slice. `Copy`, `Send`, and `Sync` have the
exact compiler-derived facts documented in [Coherent Traits And Static
Dispatch](../../decisions/semantics/traits-and-static-dispatch.md); `Clone` and `Drop` bounds
are unavailable until their executable method/drop contracts exist. Trait
methods, associated items, generic or blanket impls, dynamic dispatch,
specialization, generic-context bound forwarding, and first-class bounded
generic function values are rejected.
## Semantic Migration

The implemented AI-first semantic slices include dedicated `Unit`/`unit`,
strict three-arm `if`, typed empty lists, Option/no-nil semantics, explicit
equality families, immutable nominal products, explicit main, effect-free
imports, and local-only mutation. `arg` returns `Option Str`; negative or
out-of-range indexes return none. Product-valued editor, terminal, and
Brainfuck state is passed through helpers and evolved only by executable or
function-local vars. Process-safe structured VM outcomes and declaration-only
marker traits with bounded generic witnesses are Current.

See [AI-First Semantic Core](../../decisions/semantics/semantic-core.md). Resolved typed HIR
is now the current boundary through which these forms will migrate, so typing
and lowering cannot interpret them differently.
## the canonical source contract Current Slice And Accepted Target

the canonical source contract generic enum declarations, exact `variant-value` construction,
closed named-field patterns, `match`, `Never`, early return, typed loop control,
and value-bearing trap/exit are Current on the SSA evaluator, reference VM, and
forced Linux x86-64 baseline/proof JIT. Automatic/host-native enum transitions
and four explicit I64/F64 conversions remain
[Accepted Targets](../../decisions/semantics/semantic-core.md). Product
remains its separate nominal product concept. Option and Result migrate to
compiler-recognized prelude enum identities only after generic ADT differential
coverage; names never confer that identity. Backends consume verified HIR/SSA
primitives and do not interpret these source forms.

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
[Exact I64 And F64 Semantics](../../decisions/semantics/numeric-semantics.md) and
[Explicit Equality Families](../../decisions/semantics/equality-families.md).
