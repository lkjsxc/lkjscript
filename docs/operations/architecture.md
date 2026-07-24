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
| Resolution and typed HIR | `crates/lkjscript-compiler/src/analyze.rs`, `hir.rs`, `operation.rs` | `analyze_program`, BindingId, typed operations/effects |
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
  -> validate top-level forms
  -> collect declaration headers
  -> resolve names and types into owned HIR
  -> lower resolved HIR to Chunk + FunctionProto bytecode
  -> run_chunk_with_args
```

Imported definitions currently share one program-global namespace. Modules,
exports, package versions, locks, and serialized bytecode are absent.

## Compiler Pipeline Status

Parsed AST -> resolved typed HIR -> reference bytecode is **Current**. HIR owns
binding identity, nominal product/field identity, exact static type facts,
declaration kind, canonical operation and resolved signature, source origin,
and conservative effects;
codegen no longer re-parses declarations or resolves names.

Typed SSA, a shared native code-object backend, function/loop-triggered runtime
JIT, a minimal AOT test emitter, and direct Wasm consuming the same semantic IR
family remain **Accepted Targets**. Native scalar/product representations
replace universal tagged values only after differential SSA/backend gates. The
VM remains the cold tier and oracle. See
[Typed Compiler Pipeline And Runtime JIT](../decisions/compiler-pipeline.md) and
[Runtime JIT Instead of Offline PGO](../decisions/runtime-jit-instead-of-offline-pgo.md).

## Runtime Flow

```text
Chunk main
  -> install globals and closures
  -> dense opcode dispatch
  -> stack frames and return-adjacent tail reuse
  -> tagged immediate values or arena objects
  -> host operation dispatch
  -> VM resource table
  -> lkjscript-sys Linux FFI
```

The VM is synchronous and single-threaded. Process exit, blocking host effects,
and process-global IO prevent safe multi-VM supervision today.

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

With resolved typed HIR and separate Unit/Option/empty-list/equality semantics
plus immutable nominal products in place, establish explicit main and
effect-free libraries, migrate global editor state into a product plus one
local var, remove
mutable globals, validate chunks, and make VM outcomes process-safe. Typed SSA,
its verifier/differential oracle, and a shared Linux x86-64 native code-object
backend follow. The first adaptive execution target is synchronous baseline
JIT, followed by loop OSR and proof-based optimizing JIT. Minimal file emission
remains only for backend tests; offline PGO is rejected. The exact active
boundary is [Callable Linux x86-64 Baseline JIT Cycle](../decisions/callable-baseline-jit.md).
Real modules, process-safe host services, byte strings/views, and measured memory strategies
build on those layers as vertical slices.
