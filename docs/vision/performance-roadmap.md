# Performance roadmap

## Purpose

Organize the path toward a low-level, highly abstract, eventually very fast
`lkjscript2026` runtime. This is aspiration and sequencing — not a release claim.

## Layers

```mermaid
flowchart TB
  now[Now_correctness_and_scratch_host]
  thin[Thin_sys_primitives_plus_script_libs]
  types[Static_types_and_modules]
  gc[Precise_then_generational_GC]
  jit[Baseline_JIT]
  pgo[Profile_guided_adaptive_opts]
  now --> thin --> types --> gc --> jit --> pgo
```

### Now

Correct editor/runtime, package-root imports, hardcoded limits, thin TCP/HTTP
demo, honest numeric benchmarks versus C, and a scratch `lkjscript2026-sys`
layer (no crates.io OS helpers).

### Thin primitives + `.lkjscript` libraries

Shrink fat host opcodes over time. Keep hot ops obvious so a future JIT can
specialize them; put policy (termios, sockets, buffering) in `.lkjscript`.
See [decisions/scratch-host.md](../decisions/scratch-host.md).

### Static types and modules

Gradual annotations and checked defs; sealed modules later (consult before
breaking shared editor globals).

### GC

Replace bump-only allocation with precise mark-sweep, then generational
collection and write barriers in the host.

### Baseline JIT

Grow `JitHook` from a stub into a baseline native compiler for hot calls.

### Adaptive / profile-guided speed

Hot-path counters and specialization so long-running processes get faster
over time (PGO-style), without promising “world’s fastest” in marketing copy.

## Deferred

- Browser IDE (explicitly deferred)
- Full Rust-parity type system in one jump
- Claiming victory over C before measurements say so
