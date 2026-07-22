# Bytecode VM

## Context

Need a real runtime with a path to JIT, without host GC fighting value layout.

## Decision

Rust dense bytecode VM, tagged values, and a `JitHook` stub on calls. The
initial bump arena has since been replaced by precise mark-sweep GC; the dense
bytecode and JIT boundary remain active.

## Consequences

Cache-friendly execution; JIT can plug in later; Go/Zig hosts deferred.

## Rejected

Tree-walking only interpreter; shipping JIT in sprint 0.
