# VM

## Purpose

Explain the execution engine implemented in this checkout.

## Shape

- Tagged `u64` values with integers, booleans, nil, heap references, and opaque
  handles.
- Dense bytecode with locals stored in contiguous stack-frame slots.
- Precise mark-sweep heap rooted from globals and the VM stack; allocation
  pressure triggers collection after 1,024 allocations.
- Return-adjacent calls reuse the current frame for tail recursion.
- Closures reference function prototypes in the chunk.
- Host operations cover console IO, buffers, opaque file/socket handles, time,
  polling, and thin sys primitives; policy remains in `.lkjml` libraries.
- A VM is synchronous and single-threaded. The CLI currently runs one VM per OS
  process; scheduling multiple logical processes is future work.
