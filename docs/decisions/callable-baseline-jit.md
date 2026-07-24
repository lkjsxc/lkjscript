# Callable Linux x86-64 Baseline JIT Cycle

## Purpose

Fix the completion boundary and prerequisite contracts for the first native
runtime tier without describing planned work as current behavior.

## Status

**Accepted Target.** The cycle is complete only when machine code lowered from
verified typed SSA for a canonical lkjscript program is installed in W^X memory
and actually called on Linux x86-64. The current implementation boundary is
recorded in [Current State](../current-state.md).

## Platform And Tier Decision

Linux x86-64 is the only acceptance platform for this cycle. The runtime
hierarchy delivered by this cycle is:

```text
reference VM
  -> synchronous whole-function baseline JIT
```

The baseline tier is non-speculative. It does not require guards,
deoptimization, background compilation, compiler threads, loop OSR, a
persistent profile, or a persistent native-code cache. Offline PGO remains
rejected. Later loop-triggered compilation and OSR, proof-based optimization,
and only then justified guarded specialization remain separate cycles.

Generated-code execution speed is the primary performance objective after
exact semantics and safety. Compilation latency, memory, code size, and binary
size remain measured secondary costs. Emission or disassembly without a native
call is not completion.

## Explicit Executable Main

An executable root contains exactly one `main` form. Imported files contain
only `import`, function `def`, and `product` declarations and may not contain
`main`, top-level `do`, or runtime value definitions. The canonical form is:

```text
main/
sig/
->
Unit
/sig
body-expression
/main
```

The signature has no parameters and exactly one declared return type. The body
is one expression and its exact type must equal the declared return type.
Script arguments remain available through the typed `arg` operation. Functions
and products may be declared in the executable root, but all runtime effects
begin in `main`. A missing main, duplicate main, or imported main is a compile
error. Top-level `do` and arbitrary runtime global initialization are removed,
not retained as compatibility forms.

## Function-Local Mutation

The canonical mutable lexical form binds one explicitly typed local:

```text
var/
name/
state
/name
type/
Product EditorState
/type
initial-expression
body-expression
/var
```

The initializer is evaluated before the binding is in scope. The binding is in
scope only in the body. The initializer type must exactly equal the declared
type. `var` may occur in any function or main and returns the body's type.
Nested `var` forms express multiple mutable locals.

`set/ name value /set` resolves the nearest lexical binding in the same
function invocation. Its target must be a `var`; parameters and immutable
`let` bindings are rejected. Resolution never crosses a function boundary and
never targets a global. The value type must exactly equal the binding type and
`set` returns Unit. Mutable closure capture remains forbidden; this cycle does
not introduce cells or implicit references.

Top-level `def` declares only an immutable function. Program-global mutable
values, global stores, and arbitrary runtime global initializers are removed.
Function installation may remain implementation metadata, but it is not
source-observable mutable state.

lkjedit, terminal state, Brainfuck, and every other source-global workload move
state into immutable nominal products. Helpers receive state explicitly and
return a replacement product. Executable main owns the evolving value in one
or a small bounded number of local vars. A mutable object stored in a product
retains that object's existing explicit mutation semantics; it does not make
the product itself mutable.

## Inferred Effects

The compiler, not source declarations, computes deterministic function effect
summaries over resolved callees. The compact lattice contains at least:

- allocation;
- memory read;
- memory write;
- local mutation;
- host IO;
- possible trap;
- possible explicit exit or other process outcome change;
- possible divergence.

Each expression contributes its direct effects. A direct call contributes its
arguments, call operation, and resolved callee summary. Canonical generic
instantiations map to their resolved callee identity. Indirect or unresolved
call provenance remains conservatively all-effects.

Summaries are the least monotone fixed point over the finite bitset. Recursive
strongly connected components converge together and retain divergence without
inventing unrelated allocation, IO, trap, or write effects. Function and
summary order is stable by compiler identity, independent of hash iteration or
declaration order. No native movement or dead-code decision may drop an effect
absent proof.

## Validated Execution Boundary

Raw mutable bytecode construction is distinct from executable bytecode. One
validator consumes a raw chunk and returns an opaque `ValidatedChunk` or a
validation error. Ordinary VM, disassembly, SSA/native linkage, and tiering
paths cannot construct or execute a validated value without that validator.
Compiler-produced chunks pass through the same boundary as directly
constructed malformed test chunks.

Validation decodes every reachable and unreachable byte before effects occur
and checks at least:

- known, non-retired opcodes and complete operands;
- bytecode, table, metadata, and per-function size limits;
- constant, prototype/function, local, function-metadata, product, field, and
  any remaining implementation-global indexes;
- product identity, descriptors, field categories, and duplicate or
  inconsistent metadata;
- zero captures while source closures cannot capture;
- function arity, local count, main entry, and return shape;
- jump bounds and instruction-boundary targets;
- stack underflow and equal stack shape at CFG joins;
- definite local initialization on every path;
- statically checkable Option, Result, list, buffer, handle, and operation
  categories.

Validation failure is not a language trap and no bytecode executes before it is
reported. The VM and native tiers consume the same validated semantic object;
there is no backend-specific weaker validator.

## Structured Execution Outcomes

The execution core never terminates the Rust process. VM and native execution
use one structured terminal model with distinct categories equivalent to:

```text
Returned(value)
Exited(code)
Trapped(trap)
DeadlineExceeded
ResourceLimitExceeded(kind)
HostFailure(error)
```

A language `Option`, language `Result`, validation error, language trap,
explicit exit, deadline, resource limit, and host-service failure are never
conflated. Ordinary recoverable `sys-*` failures remain language Results when
the operation contract says so. Generated code and the VM never call
`std::process::exit`.

The outer execution owner stops execution, releases or transfers runtime
resources, restores terminal state exactly once, and flushes output according
to the language contract before the CLI translates a completed outcome into a
process exit status. Cleanup failure is reported without erasing the prior
outcome. A later VM instance is independent of earlier exit, trap, deadline, or
resource exhaustion.

Instruction fuel, stack/frame depth, aggregate heap/allocation, handle count,
output, bytecode, and native-code resources receive explicit bounded
configuration. A wall deadline is checked at calls, loop backedges,
allocations, host calls, polls, and tier transitions; blocking operations must
honor remaining time or report that the deadline contract is unsupported.
Forced native execution cannot claim support for a missing required limit.

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

The accepted [Linux x86-64 Native Backend](linux-x86-64-native-backend.md)
selects the owned encoder because its generated execution was materially faster
under the predeclared rule. Avoiding a dependency did not decide the result.
Cranelift is rejected for this production baseline decision but remains a
future measured replacement candidate under the record's explicit conditions.
LLVM is not the default fallback.

Selection is now followed by an isolated source-independent scalar foundation:
a closed machine-plan verifier, owned encoder, complete installable-image
metadata, and safe bounded W^X boundary with actual intermediate generated-code
calls. Temporary spike artifacts remain removed. This does not lower verified
typed SSA, transfer from the VM, provide an engine or runtime tier, or satisfy
forced source-derived native/JIT completion.

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

After acceptance, `auto` becomes the ordinary run default. Tests can select a
low deterministic threshold or disable tiering. Diagnostics and native counts
never contaminate ordinary program stdout.

## Completion And Evidence

Completion requires a meaningful multi-block or multi-function canonical
program and a long scalar/numeric workload to execute generated code. Required
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

## Rejected For This Cycle

- observation-only or inert engine seams;
- machine-code fixtures unrelated to typed SSA;
- emitted or disassembled but uncalled code;
- hard-coded benchmark expressions or output;
- silent VM fallback in forced mode;
- bytecode or untyped-syntax reinterpretation in the backend;
- RWX memory or unchecked executable bytes;
- process exit from the execution core or generated code;
- imprecise native GC roots;
- background compilation, OSR, speculative optimization, guards, and
  deoptimization;
- persistent profiles, persistent code cache, and offline PGO;
- non-Linux or non-x86-64 completion claims.
