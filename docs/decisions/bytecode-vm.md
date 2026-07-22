# Bytecode VM

## Purpose

Record the current execution architecture and the status of its native-code
seam.

## Status

Dense Rust bytecode, tagged values, and precise mark-sweep are **Current**.
The call-observation JIT hook is explicitly **Placeholder**.

## Decision

Use a compact Rust bytecode VM with contiguous stacks and an owned heap rather
than a tree-walking-only interpreter or host-language GC. Keep the compiler/VM
ABI small enough to validate and eventually lower from a typed IR.

The placeholder JIT interface is not an execution boundary and must not return
or imply compilation success until callable compiled-code objects exist.

## Consequences

- Interpreter dispatch and value representation remain measurable hot paths.
- Runtime semantics must be normalized before native optimization.
- Public chunks need validation before arbitrary construction becomes supported.
- A future JIT can be adopted only through correctness and warmup/steady-state
  evidence.

## Rejected

- Tree-walking as the only runtime.
- Calling an observation callback a working JIT.
- Shipping native code before semantic conformance and measurement foundations.
