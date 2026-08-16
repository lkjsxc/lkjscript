# Bootstrap language specification

## Types and values

The closed semantic types are `unit`, `bool`, signed two's-complement `i64`, and
`nominal(declaration Node ID)`. A nominal target is exactly one product or closed-sum declaration in
the same workspace. Primitive equality is by primitive kind; nominal equality is only by persistent
declaration identity. Names and equal shape do not affect type equality. There are no implicit
conversions, null or dynamic values, casts, exceptions, or generics. Functions own ordered
parameters and one result type. Values are function parameters, structured block arguments, or the
single result of an operation.

A product declaration owns an immutable ordered field sequence; fields have persistent identity,
dense ordinals, names, and exact value types. A sum declaration owns a nonempty immutable ordered
variant sequence; variants have persistent identity, dense ordinals, names, and zero or one payload
type. Duplicate member names reject. Rename is presentation-only. Shape changes require a new
declaration identity.

Product fields and variant payloads are by-value dependencies. Direct, indirect, and mixed
product/sum cycles reject through deterministic iterative validation. The cycle diagnostic selects
the strongly connected cycle component whose lowest declaration ID is smallest, targets that
participant, and reports only that component's sorted participants under the generic diagnostic
bound; acyclic declarations that depend on the cycle are not participants. Derived layout is not
semantic state: `unit` is size 0/alignment 1/zero cells, `bool` is 1/1/one cell, and `i64` is
8/8/one cell.
Products lay out fields in declaration order with checked alignment; sums use declaration ordinal as
discriminant, the smallest 1/2/4/8-byte tag width, and maximum payload size/alignment/cells. Layout
overflow is an explicit derived unrepresentable fact and does not invalidate the graph.

Current primitive and named immutable values have copy semantics. Operation contracts record `copy`
operand use. This is the accepted contract for the implemented pure value classes, not a decision
that future resource-owning or move-only values must copy. No resource ownership or borrow rules are
currently accepted.

## Memory-safety surface

A valid program in the current language cannot express raw pointers, arbitrary addresses, unchecked
loads or stores, pointer arithmetic, unchecked casts, arbitrary byte reinterpretation, direct
foreign-memory access, explicit deallocation, or shared mutable memory. It therefore cannot express
use-after-free, double free, invalid pointer dereference, out-of-bounds pointer access, type confusion,
or a data race through an accepted operation. Primitive values and acyclic named immutable values
are lowered through independently verified layouts; the current interpreter stores them in bounded
flat cells.

This language-level exclusion is one layer of the memory-safety contract, not a formal proof about
the implementation or its trusted computing base. Resource exhaustion is distinct: fuel, frames,
runtime-value depth/items/bytes, and live cells may reject under documented operational policy.
User-scalable calls, control, aggregate traversal, validation, and decoding must use explicit frames
or work collections rather than consuming unbounded native stack.

No universal future lifetime-management strategy is selected here. A future heap, shared value,
cycle, mutable object, foreign value, or external resource must add implemented and verified aliasing,
lifetime, cleanup, concurrency, and permission semantics appropriate to that data class. Tracing
collection, reference counting, affine ownership, regions, stable handles, borrowing, copy-on-write,
and hybrids remain evidence-gated implementation and language-design options.

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
- `construct_product(product, fields)` names one product declaration and one identity-keyed value
  for every exact field; canonical storage follows declaration order and every value has the field's
  exact type;
- `project_field(value, field)` names an exact product field and requires a value of that field's
  owning product type;
- `construct_variant(variant, payload)` names an exact sum variant and carries no payload for a
  nullary variant or exactly one value of the declared payload type;
- `match_sum(scrutinee, result, arms)` requires a closed-sum value and owns exactly one arm for every
  variant in declaration order. A payload arm has one exact payload block argument, a nullary arm has
  none, every arm yields `result`, and only the selected arm is semantically evaluated;
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
operation, or an already-complete target rejects. A nominally typed hole may refine only to a valid
regionless product construction, variant construction, or field projection with the same result
type; `match_sum` is never refinement-eligible.

## Compilation and execution

A selected entry is eligible for lowering only when its dependency closure is complete. Unused
incomplete definitions do not block that entry. The single executable route is:

```text
immutable SPG snapshot -> completeness/type validation -> Core IR -> verifier -> interpreter
```

The compiler iteratively discovers the exact direct-call closure and transitive nominal-type closure.
Dense private function IDs follow persistent function Node-ID order. The private type table fixes
`unit`, `bool`, and `i64` first, then reachable nominal declarations in persistent Node-ID order;
unreachable declarations are omitted. Every nominal entry retains its semantic declaration/member
origins and a fully recomputed deterministic layout. Core value types use only private type IDs.

Function-scoped blocks use typed parameters and explicit branch arguments; values never flow
implicitly between blocks. Lowering threads the complete visible semantic environment through
generated block parameters. `if` lowers to lazy arm blocks and a join. `for_i64` lowers through a
header, body, and exit. Product construction, projection, and variant construction lower to exact
aggregate instructions. `match_sum` lowers to one exhaustive variant switch, one payload marker only
for payload variants, lazy arm blocks, deterministic captures, and one typed join. The independent
verifier rederives type layouts and frame footprints and rejects malformed dependencies, aggregate
instructions, switch tables, payload edges, or indexes.

Public invocation values are exact `unit`, `bool`, `i64`, product, or sum projections. Products name
the declaration and every field Node ID; input field order may vary but is normalized to declaration
order. Sums name the declaration and exact variant Node ID and carry a payload exactly when declared.
Nested values are checked against the selected immutable revision and bounded to depth 24. The
4,096-item and 64 KiB encoded-value policies aggregate across all Run arguments; componentwise
mandatory-result maxima are preflighted before compilation or execution.

Execution uses one deterministic loop over explicit frames. Each frame owns one flat cell arena plus
separate per-value initialized facts. Unit uses zero cells, bool and i64 use one, products concatenate
field ranges, and sums use one discriminant cell plus the maximum payload range; inactive payload
cells are zero. The 65,536 live-cell policy applies to the peak of all live frame arenas plus exact
argument, edge, return, and public-flatten scratch, plus a new callee arena when applicable. The peak
is checked before allocation or copy; scratch ends at its transfer boundary and returned frame arenas
are released immediately. Aggregate construction, projection, and discriminant reads operate directly
on exact arena ranges, and block entry invalidates value facts without clearing the arena.

Fuel is charged before work. Every executed instruction or transfer costs one base unit. Each
logically copied value additionally costs `max(1, materialized_cells)`, so unit values, unit fields,
and unit arguments remain metered. Product construction charges its field copies, projection charges
the full projected result, selected match edges charge only their selected captures/payload, and call,
branch, and return charge every transferred value. Variant construction charges the full sum cell range for canonicalization plus the active payload's logical copy, including a logical copy for zero-cell payloads. Unselected match arms consume
neither execution work nor copy fuel. Frame/live-cell exhaustion, fuel exhaustion, and arithmetic
overflow are distinct structured failures and do not mutate daemon state.

There are currently no general patterns, generics, effects, permission values, host operations,
resource-owning or move-only values, native execution, or source syntax.
