# AI-First Semantic Core

## Purpose

Define the accepted semantic destination for a language written primarily by
AI systems and optimized through typed compilation rather than source-level
ambiguity or unchecked hints.

## Status

**Accepted Target** overall. Dedicated Unit and exact three-arm `if` are
**Current**. Option, typed empty lists, local-only mutation, explicit main, and
the equality split remain pending; [current-state.md](../current-state.md)
records the exact boundary.

## Canonicality

One concept has one source form. The language has no implicit conversion,
implicit import, implicit missing branch, implicit absence, implicit mutable
capture, or context-dependent equality. Source is formatted into one canonical
form, and diagnostics must be available as stable machine-readable records.
Optimization annotations are never trusted: the compiler proves them, lowers
them to checked assertions, or rejects them.

## Unit, Absence, And Empty Collections

Generic `nil` is removed. Three meanings are distinct:

```text
Unit                 unit
Option T absence     none/ T /none
empty List T         empty-list/ T /empty-list
```

`Unit` has one value, `unit`, and means successful completion without useful
data. `Option T` uses `some/ value /some` or the explicitly typed `none` form.
An empty list is a typed collection value, not Unit or Option absence.

Reference-VM representations may use dedicated singleton tags. Typed native and
Wasm backends specialize Option representation per type, including proven
niches, without changing source semantics. The all-zero internal value pattern
is invalid rather than a semantic default.

The Unit slice is implemented: `Unit`/`unit` has a dedicated VM tag, empty `do`,
`while`, `set`, and successful side-effecting operations return Unit, and
completion is no longer represented as nil. Legacy `nil` remains temporarily
only for list termination, absence, and internal default state until the
Option/typed-empty-list slice removes it.

## Control Flow

`if` is always an expression with exactly three operands: condition, then, and
else. The condition is Bool and both reachable arms have exactly the same type.
There is no implicit Unit or absence arm. `while` returns Unit. A future `Never`
type represents trap, return, and other unreachable control edges and may join
with the surviving branch type.

This control-flow contract is implemented through parser, resolved typed HIR,
bytecode, VM, tests, and the complete source corpus. Conditions are Bool rather
than general truthy values. Missing stack values,
locals, globals, or operands are validation/runtime errors, never Unit, none,
or an empty list.

## Option, Result, And Trap

The three failure paths are separate:

- `Option T`: normal presence or absence;
- `Result T E`: recoverable success or failure information;
- Trap: execution cannot continue in the current VM process.

Type errors are compile errors. Invalid bytecode is rejected before execution.
Capability absence is a link error or an operation Result. A trap returns a
structured VM outcome to the host and does not terminate the Rust process.
General exceptions and stack unwinding are not part of the accepted core.

`arg I64 -> Option Str`; an out-of-range index returns none.

## Bindings And Mutation

Top-level `def` declares an immutable function. Immutable local bindings remain
lexical. `var` introduces an explicitly typed function-local mutable binding;
`set` resolves only to an enclosing local `var`, requires exact type equality,
and returns Unit.

Global mutable values are forbidden. Immutable values may be captured by a
closure. Mutable capture is forbidden; a future explicit `Cell T` or another
reference type is required when observable shared mutation is intentional.

The current singleton lkjedit globals must migrate to an immutable product
value passed through helpers and held by one local `var` in executable main.
A record/product contract must land before that migration; positional or hidden
mutable-cell substitutes are rejected.

## Declarations And Initialization

Imported libraries contain declarations only and execute no top-level code.
Executable roots contain exactly one explicit `main`; top-level `do` is removed.
Runtime effects begin from main and receive capabilities explicitly.

Global data is eventually limited to:

- `const`: pure, transitively compile-time evaluated values;
- `static`: immutable data representable directly in the built artifact.

Arbitrary value definitions and runtime global initializers are removed.
Circular constant dependencies are compile errors. A future `once` mechanism,
if justified, requires explicit failure, retry, synchronization, and cycle
semantics rather than hidden initialization.

## Equality

There is no universal `eq` or `ne`.

- `equal-value`: statically selected value equality for Unit, Bool, I64, IEEE
  F64, Str, Bytes, Symbol, enums, and approved immutable product values;
- `same-object`: identity comparison for explicit identity-bearing reference
  types such as Handle, Buf, Cell, and Ref;
- `list-equal`: explicit bounded structural list comparison;
- `f64-bits-equal`: exact `to_bits` equality, distinct from IEEE equality.

Use `not` around a positive comparison instead of negative aliases. Closure
comparison is forbidden so closure allocation, duplication, merging,
inlining, and elimination remain unobservable.

## Integer And Floating Modes

The current checked `+`, `-`, `*`, and `div` remain the safe defaults. Explicit
wrapping and saturating integer operations may be added as separate names.
Future floating optimization assumptions are granular and independently
verified; there is no single implicit fast-math mode. AI-authored `assume` is
never forwarded to an optimizer without proof or a retained runtime check.

## Collections And Effects

Linked List/Pair is not the default performance collection. The accepted core
prioritizes Vec, Slice, Bytes, Str, fixed products, and views; List remains an
explicit persistent structure.

Typed IR records relevant effects, including allocation, memory reads/writes,
host IO, and possible traps. Effect declarations are inferred and checked,
not trusted. This enables compile-time evaluation, code motion, ownership
analysis, and safe host boundaries.

## Rejected

- Reusing Nil for Unit, absence, or empty collections.
- Optional `if` branches or Nil-based branch joining.
- Program-global mutable slots.
- Library initialization side effects.
- Universal dynamic equality.
- Observable closure identity.
- Unchecked optimizer assumptions.
- Source brevity that introduces implicit runtime behavior.
