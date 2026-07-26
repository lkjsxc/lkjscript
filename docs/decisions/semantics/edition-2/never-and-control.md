# Edition 2: Never And Control

[Authority](../edition-2.md)

## Purpose

Define the uninhabited `Never` type, structured divergence, early return, and
typed loop control.

## Status

<!-- LKJ-STATUS id=edition-2-never-control/1 status=current -->

**Current for Edition 2.** Edition 1 retains its existing control behavior and
its existing `exit` operation; the new forms require the exact Edition 2 marker.

## Never

`Never` is uninhabited and has no runtime materialization, default, slot, field,
argument, return payload, or ABI value. It joins only an explicitly divergent
edge with a surviving expression type. It is not universal assignability,
subtyping, coercion, or an arbitrary missing value.

These exact control expressions have type `Never`:

- `exit I64`;
- `trap TrapValue`, where the Current `TrapValue` is exact `Str`;
- `return T`, where `T` exactly matches the function return type;
- `break T`, where `T` exactly matches the targeted typed loop result; and
- `continue`, targeting the nearest loop.

Edition 2 adds early return and typed loop/break/continue. The canonical typed
loop is `loop/ type/ T /type body... /loop`; natural body fallthrough and
`continue` jump to its header, while `break/ value /break` jumps to its typed
exit. `while` remains Unit and its `break` must carry exact Unit. `return`,
`break`, `trap`, and `exit` each have exactly one child; `continue` has none.
Labels and nonlocal control are outside this slice.

`Never` may be written as a type node so Semantic Source can describe it, but
source analysis rejects it from signatures, returns, parameters, locals,
fields, enum substitutions, collections, constants, and every other storage or
ABI position. It is admitted only as the derived type of a terminating control
expression and at reachable expression joins.

## Exact HIR Terminators

HIR uses `Return { value }`, `Break { loop_id, value }`,
`Continue { loop_id }`, `Exit { code }`, and `Trap { value }`, plus typed
`Loop { loop_id, result_type, body }` and Unit `While { loop_id, ... }`. Each
transfer ends its control path; no synthetic Never value is produced.

## Exact SSA Terminators

SSA uses `Return(value)`, `Jump(loop_break, [value])`,
`Jump(loop_header, loop_args)`, `Exit(code)`, and `Trap(value)`. Typed loop
headers and exits use verified block parameters. No instruction defines a
Never-typed SSA value, and malformed fallthrough after a terminator is rejected.
CFG verification proves target identity, argument types, dominance, and exact
reachable joins before any evaluator or backend consumes the program.
