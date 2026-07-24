# Architecture

## Purpose

Give humans and coding agents a direct map from product behavior to the files
that own it.

## Status

**Current**, with accepted foundation changes called out explicitly.

## Crate Graph

```text
lkjscript-core
    ^             ^
    |             |
compiler          sys
    ^             ^
    |             |
    +--- app -----+--- vm
    |
    +--- xtask
```

The actual dependency edges are:

- `lkjscript-compiler -> lkjscript-core`
- `lkjscript-vm -> lkjscript-core + lkjscript-sys`
- `lkjscript-app -> compiler + core + vm`
- `lkjscript-xtask -> compiler + core`

`lkjscript-core` and `lkjscript-sys` have no third-party dependencies.

## Ownership Map

| Concern | Primary location | Entry symbols |
| --- | --- | --- |
| CLI | `crates/lkjscript-app/src/main.rs` | `main`, `real_main` |
| Public compiler API | `crates/lkjscript-compiler/src/lib.rs` | `compile_path`, `compile_path_with_sources`, `compile_source`, `validate_source` |
| Source loading/imports | `crates/lkjscript-compiler/src/import.rs` | `load_program`, import resolution |
| Physical syntax | `crates/lkjscript-compiler/src/lex.rs`, `parse.rs` | `lex`, `parse_tokens` |
| Resolution and typed HIR | `crates/lkjscript-compiler/src/analyze.rs`, `effects.rs`, `hir.rs`, `operation.rs` | `analyze_program`, fixed-point effect inference, explicit Main/Function, BindingId, local slots, typed operations/effects |
| Type representation | `crates/lkjscript-compiler/src/types/` | canonical Type parsing and substitution |
| HIR bytecode lowering | `crates/lkjscript-compiler/src/codegen/` | `compile_program` |
| Shared bytecode/value ABI | `crates/lkjscript-core/src/` | `Chunk`, `Op`, `Value`, `HeapObj` |
| VM loop | `crates/lkjscript-vm/src/run.rs`, `run/` | `Vm::run`, dispatch and calls |
| Heap/GC | `crates/lkjscript-vm/src/arena.rs` | `Arena` |
| Host resources | `crates/lkjscript-vm/src/host*.rs` | IO, buffers, descriptor table |
| Linux FFI | `crates/lkjscript-sys/src/` | owned file/socket/time/ioctl wrappers |
| Repository gates | `crates/lkjscript-xtask/src/` | `quiet verify`, source/tree/doc checks |
| Language library | `src/std/` | imported `std/...` definitions |
| Validation package | `src/lib/lkjedit/` | editor state and control loop |
| Executables | `src/examples/` | hello, Mandelbrot, HTTP, benchmark, editor |

## Compile Flow

```text
CLI path
  -> compile_path
  -> package-root and import resolution
  -> lex each source
  -> per-file source limits
  -> parse matched forms
  -> enforce one root main and declaration-only imports
  -> collect immutable function and product headers
  -> resolve exact types, binding IDs, and local slots into owned HIR
  -> infer stable fixed-point function effects and recompute expression effects
  -> install internal function closures, then lower explicit main and functions
  -> mutable Chunk builder
  -> validate_chunk -> opaque immutable ValidatedChunk
  -> run_chunk_with_args(ValidatedChunk, ExecutionConfig)
```

Imported immutable function and product declarations share one program
declaration namespace. Modules, exports, package versions, locks, and
serialized bytecode are absent. Source runtime globals and imported execution
are rejected.

## Compiler Pipeline Status

Parsed AST -> resolved typed HIR -> reference bytecode is **Current**. HIR owns
an explicit Main and Functions, resolved binding IDs and local slot references,
immutable declaration kinds, MutableLocal/SetLocal nodes, nominal product IDs
and field indexes, exact static type facts, source origins, canonical operation
identities and per-call signatures, compact fixed-point function summaries, and
final per-expression effects. Direct resolved function calls use canonical
callee summaries; indirect provenance remains all-effects. Codegen no longer
re-parses declarations or resolves names. Source SetGlobal and runtime value-
definition paths are absent.

Typed SSA, the selected owned Linux x86-64 native code-object backend,
function/loop-triggered runtime JIT, a minimal AOT test emitter, and direct Wasm
consuming the same semantic IR family remain **Accepted Targets**. Backend
selection is complete, but no native code is implemented. Native scalar/product
representations replace universal tagged values only after differential
SSA/backend gates. The VM remains the cold tier and oracle. See
[Typed Compiler Pipeline And Runtime JIT](../decisions/compiler-pipeline.md),
[Linux x86-64 Native Backend](../decisions/linux-x86-64-native-backend.md), and
[Runtime JIT Instead of Offline PGO](../decisions/runtime-jit-instead-of-offline-pgo.md).

## Runtime Flow

```text
ValidatedChunk main
  -> explicit ExecutionConfig budgets and monotonic deadline
  -> install internal immutable function closures
  -> execute the source main body
  -> dense opcode dispatch with fuel/stack/frame/heap/allocation metering
  -> stack frames and return-adjacent tail reuse
  -> tagged immediate values or arena objects
  -> bounded handle/output accounting
  -> host operation dispatch
  -> lkjscript-sys Linux FFI
  -> owned Returned value or structured terminal outcome
  -> drop resources, restore terminal, flush, then CLI status translation
```

The VM is synchronous and single-threaded. It never terminates the Rust process;
exit, traps, limits, deadlines, and host failures stop only the current VM.
Returned heap values own a private reachable-object snapshot, and later VM
instances have fresh globals, arenas, handles, counters, and deadlines.
Process-global stdin/stdout and the terminal guard still prevent parallel VM
supervision. Cooperative deadlines can overrun inside current filesystem and
write/send wrappers; hard-deadline mode rejects those operations before effects
rather than claiming cancellation.

The accepted later native flow is:

```text
VM function entry / loop backedge
  -> bounded process-local saturating hotness
  -> synchronous typed-SSA compilation at a safepoint
  -> bounded callable native code object
  -> VM/native or OSR transfer
  -> exact VM fallback or structured outcome
```

No part of that flow is current. The existing observation hook sees closure
calls only and cannot compile or transfer execution. The active cycle ends only
when synchronous whole-function baseline code is actually called on Linux
x86-64 in truthful forced and automatic modes. Loop OSR, background
compilation, optimizing tiers, persistent profiles, and persistent code caches
are not part of that cycle.

## Source Layout Rule

The current language rule limits each lkjscript source directory to 16
immediate entries, counting files and subdirectories together. Rust crates,
documentation, metadata, `.git`, and build output are not language source
and are outside this rule.

The repository gate checks the complete in-tree language corpus. The compiler
also rejects an entry or imported source directory that violates the rule, so
an external project receives the same contract.

## Change Guide

- Change syntax: lexer/parser docs, compiler lexer/parser, corpus, and negative fixtures.
- Change types: language docs, type prelude/inference, lowering, VM behavior, and conformance tests.
- Add an opcode: core ABI, code generation, dispatch, disassembly, and malformed-bytecode validation.
- Add host capability: accepted decision, sys safety wrapper, VM resource boundary, typed prelude, script policy wrapper, and failure tests.
- Change limits: language decision, shared core constant, compiler enforcement, repository gate, and boundary tests.
- Change packaging: imports decision, resolver, installed layout, Docker/native bundle, and external-project smoke.

## Accepted Redesign Direction

Explicit main, effect-free imported libraries, local-only mutation,
product-threaded editor, terminal, and Brainfuck state, whole-chunk validation,
structured process-safe outcomes, bounded VM execution, and deterministic
fixed-point function effects are now Current. Typed SSA, its verifier and
differential oracle, and the selected owned Linux x86-64 native code-object
backend follow. The first adaptive execution target remains synchronous
callable baseline JIT; loop OSR
and proof-based optimizing JIT are later. Minimal file emission remains only
for backend tests, and offline PGO is rejected. The exact active boundary is
[Callable Linux x86-64 Baseline JIT Cycle](../decisions/callable-baseline-jit.md).
Real modules, process-safe host services, byte strings/views, and measured
memory strategies build on those layers as vertical slices.
