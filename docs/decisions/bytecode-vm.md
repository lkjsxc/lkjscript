# Bytecode VM

## Context

Need a real runtime with a path to JIT, without host GC fighting value layout.

## Decision

Rust dense bytecode VM, tagged values, bump arena, `JitHook` stub on calls.

## Consequences

Cache-friendly execution; JIT can plug in later; Go/Zig hosts deferred.

## Rejected

Tree-walking only interpreter; shipping JIT in sprint 0.
