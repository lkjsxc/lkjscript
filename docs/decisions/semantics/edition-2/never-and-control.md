# Edition 2: Never And Control

[Authority](../edition-2.md)

## Purpose

Define the uninhabited `Never` type, structured divergence, early return, and
typed loop control.

## Status

**Accepted Target, not Current.** Existing Edition 1 control behavior remains
Current.

## Never

`Never` is uninhabited and has no runtime materialization, default, slot, field,
argument, return payload, or ABI value. It joins only an explicitly divergent
edge with a surviving expression type. It is not universal assignability,
subtyping, coercion, or an arbitrary missing value.

These exact control expressions have type `Never`:

- `exit I64`;
- `trap TrapValue`;
- `return T`, where `T` exactly matches the function return type;
- `break T`, where `T` exactly matches the targeted typed loop result; and
- `continue`, targeting the nearest loop.

Edition 2 adds early return and typed loop/break/continue. `while` remains Unit
and its `break`, if accepted in a while body, must carry Unit. Labels and
nonlocal control are outside this slice.

## Exact HIR Terminators

HIR uses `Return { value }`, `Break { loop_id, value }`,
`Continue { loop_id }`, `Exit { code }`, and `Trap { value }`. Each ends its
control path; no synthetic Never value is produced.

## Exact SSA Terminators

SSA uses `Return(value)`, `Jump(loop_break, [value])`,
`Jump(loop_header, loop_args)`, `Exit(code)`, and `Trap(value)`. Typed loop
headers and exits use verified block parameters. No instruction defines a
Never-typed SSA value, and malformed fallthrough after a terminator is rejected.
CFG verification proves target identity, argument types, dominance, and exact
reachable joins before any evaluator or backend consumes the program.
