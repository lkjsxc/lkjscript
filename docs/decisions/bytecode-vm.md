# Bytecode VM

## Purpose

Record the current execution architecture and the status of its native-code
seam under the runtime-JIT-first plan.

## Status

Dense Rust bytecode lowered from verified normalized typed SSA, tagged values,
precise mark-sweep, whole-chunk validation, structured process-safe outcomes,
configured VM resource limits, bounded function-entry hotness, and callable
allocation-free scalar Linux x86-64 baseline code objects are **Current**. The
old observation hook is removed. Loop hotness/OSR, native references and
allocation, and optimizing tiers remain **Accepted Targets** and are not
implemented.

## Decision

Use a compact Rust bytecode VM with contiguous stacks and an owned heap rather
than a tree-walking-only interpreter or host-language GC. Keep it as the cold
execution tier, correctness oracle, deterministic/debugging path, unsupported-
platform path, and fallback for unsupported or resource-limited JIT code.

Bytecode lowering consumes only verified normalized `lkjscript-ir` SSA shared
with the later runtime JIT, minimal AOT test emitter, and Wasm. The obsolete
sibling HIR semantic emitter is deleted. A native backend must not reinterpret
bytecode semantics independently of typed SSA.

The former `JitHook::observe_call` placeholder is removed. VM call dispatch now
uses a real `RuntimeTier`: auto entry counts can synchronously compile a verified
scalar function group, and later calls can transfer exact scalar values to an
installed entry. VM-only dispatch remains a no-tier implementation.

## Current Validated Execution Boundary

Raw mutable `Chunk` values are builder/compiler/malformed-test inputs, not
executable programs. The one `validate_chunk` function consumes a raw chunk and
produces an opaque immutable `ValidatedChunk`. Public VM entry points,
disassembly, and the JIT observation interface accept only that type. Compiler
`ExecutableProgram` exposes validated bytecode explicitly and also retains the
verified SSA and deterministic link metadata. Compiler-produced chunks cross
the same validator as direct malformed test chunks.

Validation decodes reachable and unreachable bytes and checks opcode retirement,
operand completeness, configured encoded/table/code/metadata limits, constants,
prototype and local/global indexes, main/arity/local shape, zero captures,
product IDs/descriptors/fields and duplicate metadata, instruction-boundary
jumps, CFG stack depth/category joins, definite local initialization, reachable
fallthrough, return shape, explicit trap diagnostics, and statically known
scalar, Option, Result, list, buffer, handle, and product operation categories.
Validation errors remain outside execution outcomes and occur before VM effects.

VM execution returns `ExecutionOutcome::{Returned, Exited, Trapped,
DeadlineExceeded, ResourceLimitExceeded, HostFailure}`. `Returned` contains an
owned heap snapshot rather than a live arena index. The execution core does not
call `std::process::exit`; after VM stop it drops handles, restores terminal
state, and flushes stdout before the CLI translates the outcome. Cleanup errors
are `HostFailure` and retain a summary of the prior outcome. Ordinary `sys-*`
failures remain language `ResultErr` values.

`ExecutionConfig` bounds fuel, operand/local stack values, frames, estimated
live heap bytes, aggregate allocations, monotonically allocated handle slots,
and stdout bytes. Its cooperative monotonic deadline is checked throughout VM
execution; stdin, handle reads, accept/receive, poll, and wait use the remaining
time through current sys polling/sleep APIs. With `require_hard_deadline`, an
operation whose current wrapper cannot be cancelled, including console writes,
filesystem open/query, and send/write, returns truthful `HostFailure` before
that effect. Cooperative mode can overrun while inside such a host call, and
cleanup flush/restoration is not cancellable.

The exact active contract is
[Callable Linux x86-64 Baseline JIT Cycle](callable-baseline-jit.md).

## Current Product Bytecode

Chunks carry immutable product names/field names separately from mutable runtime
globals. `Trap` names an exact validated Str diagnostic and terminates only the
current VM execution. `MakeProduct` names a product metadata index;
`LoadProductField` and `WithProductField` name resolved product/field
descriptors. The VM validates metadata, descriptor, field, category, and nominal
identity boundaries before access. Construction and immutable replacement
allocate traced product objects; access returns an existing field. Product
metadata never installs a global value or executes initialization code.

## Tier 0 Current Boundary

Normal auto execution begins in the VM so short commands can finish without JIT
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
- Current scalar native frames have exact empty reference stack maps; nonempty
  maps remain required before allocation-capable code executes.
- VM/native transitions preserve traps, roots, handles, metering, deadlines,
  output, and arguments.
- Runtime JIT measurements include trigger, compilation, warmup, fallback, and
  whole-program cost rather than only steady state.
- The observation hook and its null public type are removed; explicit runtime
  tier state owns compilation and transfer.

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
