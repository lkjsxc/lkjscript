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
| Static types | `crates/lkjscript-compiler/src/types/` | `typecheck_program`, inference and prelude |
| Bytecode lowering | `crates/lkjscript-compiler/src/codegen/` | `compile_program` |
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
  -> install and check global signatures
  -> lower to Chunk + FunctionProto bytecode
  -> run_chunk_with_args
```

Imported definitions currently share one program-global namespace. Modules,
exports, package versions, locks, and serialized bytecode are absent.

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

A normalized typed intermediate representation should eventually unify symbol
resolution, typing, and lowering. Real modules, process-safe host services,
byte strings, compiled-code objects, and adaptive
memory management build on that layer. They must be measured and delivered as
coherent vertical slices rather than dormant interfaces.
