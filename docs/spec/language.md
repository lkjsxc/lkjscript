# Bootstrap language specification

## Types and values

The closed semantic types are `unit`, `bool`, and signed two's-complement `i64`. There are no
implicit conversions, null or dynamic values, casts, exceptions, generics, or nominal types.
Functions own ordered parameters and one result type. Values are function parameters, structured
block arguments, or the single result of an operation.

Scalar values have copy semantics. Operation contracts record `copy` operand use;
ownership-bearing values and borrow rules do not exist yet.

## Pure operation contracts

`src/schema.rs` owns the exhaustive operation contracts consumed by graph validation, queries,
codecs, history checks, and runtime schema description.

- `const_unit` has no operands and produces `unit`;
- `const_bool(value)` has no operands and produces `bool`;
- `const_i64(value)` has no operands and produces `i64`;
- `add_i64(lhs, rhs)` copies two `i64` values and produces `i64`;
- `lt_i64(lhs, rhs)` copies two `i64` values and produces `bool`;
- `call(function, arguments)` names a function by Node ID, requires one argument per ordered
  parameter with exact types, and produces the target function's result type;
- `hole(expected)` produces one value of its exact expected type but remains incomplete;
- `if(condition, result, then_body, else_body)` requires a `bool` condition, owns ordered then/else
  regions, and produces `result`; each region has one block with no block arguments and ends in
  `yield` of exactly `result`;
- `for_i64(start, end_exclusive, step, initial, carried, body)` requires `i64` bounds, a positive
  literal step, and an initial value of `carried`; its one body block owns ordered `loop_index: i64`
  and `loop_carried: carried` arguments, ends in `yield(carried)`, and the operation produces
  `carried`;
- `yield(value)` terminates an operation-owned structured region and must match that region's
  derived yield contract;
- `return(value)` terminates a function body and must match the function result type.

`yield` is not a function terminator, and `return` is not a structured-region terminator. Structured
regions have exactly one block in the current schema. Operations are pure. Checked `i64` addition
overflow is a structured runtime trap once the operation enters executable lowering.

## Visibility, calls, and recursion

Value visibility is lexical and structural. A use may reference function parameters, prior regular
operations in its current block, and values visible before each enclosing structured operation.
Values produced later in the same block, in a sibling arm, or inside a completed nested region are
not visible. A nested body may capture visible ancestor values. Loop body arguments are visible in
the loop body and all nested regions within it.

Calls use function identity rather than names. Forward and mutually recursive calls are valid and
executable when signatures and all value contracts are exact. Calls and recursion execute with an
explicit interpreter frame vector rather than user-depth Rust recursion.

`RefineHole` is a semantic graph edit, not an executable operation. It is the sole one-way
identity-preserving constructor transition: a typed hole may become a complete, regionless,
non-terminator operation with the same one-result contract while retaining Node ID, owner, body
position, and uses. Another hole, a terminator, a different result contract, a region-owning
operation, or an already-complete target rejects.

## Compilation and execution

A selected entry is eligible for lowering only when its dependency closure is complete. Unused
incomplete definitions do not block that entry. The single executable route is:

```text
immutable SPG snapshot -> completeness/type validation -> Core IR -> verifier -> interpreter
```

The compiler iteratively discovers the exact direct-call closure, assigns dense private function IDs
in ascending persistent Node-ID order, and lowers every reachable function into one verified
multi-function Core program. Function-scoped blocks use typed parameters and explicit branch
arguments; values never flow implicitly between blocks. Lowering threads the complete visible
semantic environment through generated block parameters. `if` lowers to lazy arm blocks and a join.
`for_i64` evaluates its three value operands once, tests the index in a header, binds index and
carried body arguments, performs a checked step addition at the loop origin, and returns the final
carried value through an exit block.

Invocation arguments are ordered `unit`/`bool`/`i64` values checked exactly against the selected
entry signature. Execution uses one deterministic loop over an explicit frame vector. Runtime fuel
decrements once for each executed Core instruction and once for each terminator transfer. Positive,
bounded fuel and frame policies are required. Aggregate live frame value storage is additionally
bounded to 65,536 typed slots; entry and call frames reject before allocation when that policy would
be exceeded, and returned frames release their slots. Frame-count and live-slot exhaustion share the
structured execution-frame-exhausted result at the exact entry or call origin. Exhaustion is distinct
from an arithmetic runtime trap.

There are no aggregates, sums, patterns, generics, effects, capabilities, host operations,
ownership-bearing values, native execution, or source syntax.
