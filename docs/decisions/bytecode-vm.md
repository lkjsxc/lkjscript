# Bytecode VM

## Purpose

Record the current execution architecture and the status of its native-code
seam under the runtime-JIT-first plan.

## Status

Dense Rust bytecode, tagged values, and precise mark-sweep are **Current**. The
call-observation JIT hook is explicitly **Placeholder**. Bounded hotness,
callable code objects, tiering, and OSR are **Accepted Targets** and are not
implemented.

## Decision

Use a compact Rust bytecode VM with contiguous stacks and an owned heap rather
than a tree-walking-only interpreter or host-language GC. Keep it as the cold
execution tier, correctness oracle, deterministic/debugging path, unsupported-
platform path, and fallback for unsupported or resource-limited JIT code.

Bytecode lowering remains behind resolved typed HIR and later typed SSA shared
with runtime JIT, the minimal AOT test emitter, and Wasm. A native backend must
not reinterpret bytecode semantics independently of typed SSA.

The placeholder `JitHook::observe_call` is not an execution boundary. It returns
no status, sees no loop backedge or VM state, and cannot install or call code.
It must not return or imply compilation success until verified callable code
objects and execution transfer replace it.

## Accepted Validated Execution Boundary

Raw mutable chunks are builder/test inputs, not executable programs. One
validator must consume a raw chunk and produce an opaque immutable
`ValidatedChunk` before VM, disassembly, or JIT use. It decodes all bytes and
checks indexes, metadata identities/categories, arity/locals, zero unsupported
captures, instruction-boundary jumps, CFG stack compatibility, definite local
initialization, return shape, and configured size limits. Validation failure is
not a language trap and no effects occur first. Direct malformed-chunk tests use
the same validator as compiler output.

VM and native execution return one structured model distinguishing returned
values, explicit exit, language traps, deadline, resource limits, and host
failure. The execution core and generated code never call
`std::process::exit`; the CLI maps a completed outcome only after resource
cleanup, terminal restoration, and required flushing.

The exact active contract is
[Callable Linux x86-64 Baseline JIT Cycle](callable-baseline-jit.md).

## Current Product Bytecode

Chunks carry immutable product names/field names separately from mutable runtime
globals. `MakeProduct` names a product metadata index;
`LoadProductField` and `WithProductField` name resolved product/field
descriptors. The VM validates metadata, descriptor, field, category, and nominal
identity boundaries before access. Construction and immutable replacement
allocate traced product objects; access returns an existing field. Product
metadata never installs a global value or executes initialization code.

## Tier 0 Target

Normal execution begins in the VM so short commands can finish without JIT
cost. Tier 0 eventually records only bounded saturating function-entry and
loop-backedge events, with optional selected block counts after baseline JIT
works. It does not profile every ordinary instruction. Counters are process-
local, consumed by the same process, discarded at exit, and never telemetry.

Initial compilation remains synchronous at a safepoint. Unsupported functions,
resource exhaustion, or ordinary `auto` backend failure stay correct in the VM
and record structured fallback. Forced JIT tests fail visibly instead of
silently using the VM.

Long-running single invocations require loop-triggered compilation and verified
loop-header OSR. Compiling only for a later call is not OSR.

## Consequences

- Interpreter dispatch and tagged representation remain measurable VM hot paths.
- Runtime semantics and process outcomes must be normalized before native calls.
- Public chunks need validation before arbitrary construction is supported.
- Tagged `Value` remains a reference-VM representation, not the typed native ABI.
- Native frames require exact stack maps before allocation-capable code executes.
- VM/native transitions preserve traps, roots, handles, metering, deadlines,
  output, and arguments.
- Runtime JIT measurements include trigger, compilation, warmup, fallback, and
  whole-program cost rather than only steady state.
- The observation hook remains in code only as explicitly labeled incomplete
  behavior until a complete replacement lands.

The accepted tiering, code-object, W^X, cache, OSR, and adoption contract is
[Runtime JIT Instead of Offline PGO](runtime-jit-instead-of-offline-pgo.md).

## Rejected

- Tree-walking as the only runtime.
- Calling an observation callback a working JIT.
- Calling emitted but unexecuted machine code a JIT.
- Per-instruction profiling as the initial hotness policy.
- A separate bytecode semantic compiler for native execution.
- Shipping native execution before semantic, outcome, GC, and resource gates.
- Offline PGO or persistent cross-run profiles as part of VM tiering.
