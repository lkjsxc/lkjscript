# Callable Linux x86-64 Baseline JIT Cycle: Typed SSA Authority

[Authority](../callable-baseline-jit.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Typed SSA Authority

Resolved typed HIR lowers to typed SSA before reference bytecode or native
code. The authoritative transition for this cycle is:

```text
resolved typed HIR
  -> verified typed SSA
  -> verified non-speculative normalization
      +-> reference bytecode
      +-> Linux x86-64 native code objects
```

A temporary sibling HIR-to-bytecode lowering may exist only while differential
tests establish the SSA evaluator and bytecode cutover. It is removed before
the native backend is called authoritative.

SSA represents stable function, block, and value identities; exact types;
block parameters; direct and versioned runtime calls; explicit branch,
conditional branch, return, trap, and structured-outcome terminators; product,
Option, Result, scalar, buffer, and allocation operations; source origins;
effect facts; safepoint requirements; and bytecode-position/local/operand-stack
mappings retained for later OSR.

The verifier runs before and after every transformation and rejects duplicate or
missing IDs, use before definition, invalid dominance, block argument count or
type mismatch, malformed loops or terminators, instructions after terminators,
return/call type or arity mismatch, invalid product access, impossible runtime
signatures, invalid effects, and malformed trap/outcome edges.

An exact SSA evaluator is independent of native lowering and is compared with
the reference VM for scalar, product, Option, Result, control flow, calls,
recursion, local mutation conversion, and host-independent buffer behavior.
Baseline normalization consists only of separately tested deterministic passes:
constant folding/propagation, copy propagation, unreachable-block elimination,
branch simplification, empty-block forwarding, effect-aware dead instruction
elimination, direct-call resolution, canonical block order, and simple
fall-through layout.
## Backend Selection Gate

The gate is complete for future implementation. A real called-code spike
compared an owned x86-64 byte encoder with Cranelift 0.134.2 over checked integer
and F64 arithmetic, comparisons, branches, a loop, a direct generated call, and
a runtime-style call. It retained generated-code execution, compilation,
binary/build/RSS, dependency/unsafe/license/security, code/metadata, stack-map,
W^X, platform, maintenance, and replacement evidence.

The accepted [Linux x86-64 Native Backend](../../execution/linux-x86-64-native-backend.md)
selects the owned encoder because its generated execution was materially faster
under the predeclared rule. Avoiding a dependency did not decide the result.
Cranelift is rejected for this production baseline decision but remains a
future measured replacement candidate under the record's explicit conditions.
LLVM is not the default fallback.

Selection was followed by the source-independent scalar foundation and is now
integrated through a narrow verified-SSA adapter, owned bounded code objects,
safe W^X installation, forced main calls, and automatic later-call VM/native
transfer. Temporary spike artifacts remain removed. The integration retains the
closed plan boundary and explicitly rejects every reference/allocation/host
path rather than broadening the foundation into another semantic IR.
## Linux x86-64 ABI And Runtime Calls

The Rust/runtime boundary follows System V AMD64. The ABI record fixes integer
and floating argument/return registers, 16-byte call-site stack alignment,
caller/callee-saved registers, spill and frame layout, and a stable execution-
context pointer. Generated code does not rely on the red zone. It never embeds
private Rust layouts or unversioned object addresses.

Native values use typed I64, F64, Bool, Unit, Option, product, string/buffer,
and handle representations. The universal tagged VM `Value` is restricted to
VM/native adapters. Compatible compiled callees call each other directly.
Every code object records semantic and native ABI versions.

Complex, allocating, or host-sensitive work initially uses canonical versioned
runtime-call identities with declared argument/result types, effects,
trap/outcome behavior, safepoint status, allocation, and blocking behavior.
Pure scalar operations remain in generated code. At least one versioned runtime
call is exercised by the completion workload.
## Executable Memory, Code Objects, And GC

Executable memory is owned by safe `lkjscript-sys` wrappers whose safe input
cannot express arbitrary unsafe machine code. Installation is strictly:

```text
allocate RW -> emit and relocate -> validate -> transition to RX -> execute
```

No page is RWX, relocation never occurs after RX sealing, and mapping lifetime
strictly contains every native invocation and direct reference.

A code object retains entry and accounted sizes, function identity, tier,
semantic/native ABI versions, relocations, runtime-call references, safepoints,
exact stack maps, source/trap/outcome mappings, compilation statistics,
invalidation state, and native execution count. Process-local retention is
bounded by configurable executable bytes, object count, metadata bytes,
per-object size, compilation work, and compilation wall time. This cycle has no
persistence, serialization, background mutation, or eviction.

Allocation-free scalar functions may land first. Allocation-capable code is
accepted only after every allocation, collecting runtime call, GC poll, tier
transition, deadline poll, and cancellation point has an exact live-reference
stack map. Arbitrary machine words are never conservatively scanned.
## Engine And Tier State

Engine flags are added only after a focused test calls generated native code.
The final CLI modes are:

```text
lkjscript run --engine vm <file.lkjscript> [--] [args...]
lkjscript run --engine auto <file.lkjscript> [--] [args...]
lkjscript run --engine baseline-jit <file.lkjscript> [--] [args...]
```

`vm` never compiles or calls native code. `baseline-jit` compiles main before
entry and each reached supported user function before it is required; any
unsupported function, failed compilation, invalid code object, or exhausted
native resource is a visible structured error and never silent VM fallback.
`auto` begins in the VM, uses bounded saturating function-entry counters, and
compiles a hot supported function synchronously at a safepoint for later calls.
Unsupported or resource-limited auto compilation remains correct in the VM.
One long invocation is not accelerated without OSR and is reported as such.

Per-function state is `VmOnly`, `Observed`, `BaselineCompiling`,
`BaselineNative`, or `Disabled`, with bounded counts for calls, compile attempts,
native entries, last structured failure, object identity, and configuration or
resource epoch. Compilation is non-reentrant. A known failure is not retried in
the same epoch; a changed epoch permits only a bounded retry.

Ordinary `run` defaults to `auto` with the conservative measured threshold of
64 function entries. Explicit `--engine vm` remains deterministic and never
compiles native code; all three modes remain explicit. Tests can select another
deterministic auto threshold or disable auto tiering without disabling forced
mode. Diagnostics, retained metrics, and native counts never contaminate
ordinary program stdout.
## Completion And Evidence

The current focused source-native gate uses meaningful multi-block,
multi-function, loop-heavy scalar programs that execute generated code. Current
coverage includes checked I64 arithmetic/division, Bool/comparisons, branches,
loops, locals, direct calls, Unit, F64 arithmetic/comparisons, structured
return/trap/exit, VM/native transitions, native/native calls for compatible
functions, and a versioned runtime call. Unsupported allocation or language
semantics fail explicitly in forced mode and are named as baseline JIT MVP
coverage gaps.

Forced tests prove a code object was installed, its generated entry was called,
its native count is nonzero, no required user code fell back, and value, output,
and structured outcome match `vm`.

Retained timing separates source/HIR/SSA construction, verification,
normalization, native compilation, relocation/RX installation, first call,
native execution, total process time, steady repetitions, code/metadata bytes,
cache peak, and RSS. Short workloads use at least four warmups and 31 randomized
samples per variant and retain every sample, median, MAD, p95, minimum, and
maximum. The aspirational native-execution target is 5x over the same-commit VM
on a supported loop-heavy workload, excluding compilation; end-to-end
break-even is reported separately and an unmet target remains negative
evidence.

Brainfuck Mandelbrot is primarily a later loop-OSR workload because one long
VM invocation may never reach a later compiled entry. No current work is called
OSR until an already-running VM frame transfers through a verified loop-header
mapping.
