# Architecture

## Purpose

Give humans and coding agents a direct map from product behavior to the files
that own it.

## Status

**Current**, with accepted foundation changes called out explicitly.

## Crate Graph

```text
lkjscript-ir      lkjscript-core      lkjscript-native
      ^                 ^                    ^
      |                 |                    |
      +--- compiler ----+--- lkjscript-jit --+--- lkjscript-sys
               ^                   ^                    ^
               |                   |                    |
               +------ app --------+------ vm ----------+
               |
               +------ xtask
```

The actual product dependency edges are:

- `lkjscript-ir` and `lkjscript-native` have no dependencies
- `lkjscript-compiler -> lkjscript-ir + lkjscript-core`
- `lkjscript-sys -> lkjscript-native`
- `lkjscript-jit -> ir + core + native + sys`
- `lkjscript-vm -> core + jit + sys`
- `lkjscript-app -> compiler + ir + core + jit + vm`
- `lkjscript-xtask -> compiler + core`

The app test target also uses `lkjscript-ir` for evaluator/VM differential
checks. No workspace crate has a third-party Rust dependency.

## Ownership Map

| Concern | Primary location | Entry symbols |
| --- | --- | --- |
| CLI | `crates/lkjscript-app/src/main.rs` | `main`, `real_main` |
| Public compiler API | `crates/lkjscript-compiler/src/lib.rs` | `compile_path`, `compile_path_with_sources`, `compile_source`, `validate_source` |
| Source loading/imports | `crates/lkjscript-compiler/src/import.rs` | `load_program`, import resolution |
| Physical syntax | `crates/lkjscript-compiler/src/lex.rs`, `parse.rs` | `lex`, `parse_tokens` |
| Resolution and typed HIR | `crates/lkjscript-compiler/src/analyze.rs`, `effects.rs`, `hir.rs`, `operation.rs` | `analyze_program`, fixed-point effect inference, explicit Main/Function, BindingId, local slots, typed operations/effects |
| HIR-to-SSA conversion | `crates/lkjscript-compiler/src/ssa.rs` | environment renaming, BindingId-ordered branch/loop parameters, exact operation/type/effect transfer |
| Typed SSA authority | `crates/lkjscript-ir/src/` | IR model, `verify`, `evaluate`, isolated baseline passes, bytecode link metadata |
| Type representation | `crates/lkjscript-compiler/src/types/` | canonical source/HIR Type parsing and substitution |
| SSA bytecode lowering | `crates/lkjscript-compiler/src/codegen/` | `compile_program`; no sibling HIR semantic emitter |
| Owned x86-64 foundation | `crates/lkjscript-native/src/` | closed scalar machine plan, verification, encoding, opaque installable image |
| Verified SSA/native runtime adapter | `crates/lkjscript-jit/src/` | scalar eligibility, deterministic lowering, code objects, tier state, forced/auto execution |
| Shared bytecode/value ABI | `crates/lkjscript-core/src/` | `Chunk`, `Op`, `Value`, `HeapObj` |
| VM loop | `crates/lkjscript-vm/src/run.rs`, `run/` | `Vm::run`, dispatch and calls |
| Heap/GC | `crates/lkjscript-vm/src/arena.rs` | `Arena` |
| Host resources | `crates/lkjscript-vm/src/host*.rs` | IO, buffers, descriptor table |
| Linux FFI and W^X | `crates/lkjscript-sys/src/` | owned file/socket/time/ioctl wrappers and safe bounded executable installation/invocation |
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
  -> environment-rename HIR locals/mutation into typed SSA block parameters
  -> verify typed SSA
  -> run each deterministic isolated baseline pass with post-pass verification
  -> lower only normalized SSA and retain deterministic bytecode link metadata
  -> install internal function closures as implementation metadata
  -> mutable Chunk builder
  -> validate_chunk -> opaque immutable ValidatedChunk
  -> ExecutableProgram { verified SSA, link metadata, ValidatedChunk }
      +-> vm: run_chunk_with_args(program.bytecode(), ExecutionConfig)
      +-> baseline-jit: verified scalar group -> code object -> native main
      +-> auto: VM entries -> bounded hotness -> later native function calls
```

Imported immutable function and product declarations share one program
declaration namespace. Modules, exports, package versions, locks, and
serialized bytecode are absent. Source runtime globals and imported execution
are rejected.

## Compiler Pipeline Status

Parsed AST -> resolved typed HIR -> verified typed SSA -> verified baseline
normalization -> reference bytecode is **Current**. HIR owns an explicit Main
and Functions, resolved binding IDs and local slot references, immutable
declaration kinds, MutableLocal/SetLocal nodes, nominal product IDs and field
indexes, exact static type facts, source origins, canonical operation identities
and per-call signatures, compact fixed-point function summaries, and final
per-expression effects. SSA owns backend control/data flow, exact types,
effects, safepoints, frame states, and deterministic bytecode links. Direct
resolved function calls use canonical callee summaries; indirect provenance
remains all-effects. Codegen consumes only verified normalized SSA. The former
HIR semantic bytecode emitter, source SetGlobal, and runtime value-definition
paths are absent.

The dependency-free SSA evaluator is the differential oracle for host-
independent semantics and does not call bytecode, VM, native, or host helpers.
Console, filesystem, sockets, terminal, time, and handle operations are
explicitly unsupported in it. The selected owned Linux x86-64 closed scalar
machine plan, encoder, metadata, safe W^X boundary, verified SSA adapter,
bounded code objects, and callable function-entry baseline tier are **Current**
for allocation-free Unit/Bool/I64/F64 acyclic direct-call groups. Forced and
auto engines execute real entries. Ownership/traits, native references/allocation, loop OSR, an optimizing tier,
minimal AOT file emission, and direct Wasm remain **Accepted Targets** or later
work. The VM remains the cold tier and runtime oracle. See [Ownership And
Borrowing](../decisions/ownership-and-borrowing.md), [Coherent Traits And Static
Dispatch](../decisions/traits-and-static-dispatch.md), [Native References,
Frames, And Exact GC Stack Maps](../decisions/native-references-and-gc-stack-maps.md),
[Allocation-Capable Baseline JIT](../decisions/allocation-capable-baseline-jit.md),
[Proof-Based Optimizing JIT](../decisions/proof-based-optimizing-jit.md), [Typed
Compiler Pipeline And Runtime JIT](../decisions/compiler-pipeline.md), [Linux
x86-64 Native Backend](../decisions/linux-x86-64-native-backend.md), and [Runtime
JIT Instead of Offline PGO](../decisions/runtime-jit-instead-of-offline-pgo.md).

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

The current native flow is:

```text
forced main or hot VM function entry
  -> verified scalar eligibility and acyclic reachable group
  -> synchronous typed-SSA lowering at a safepoint
  -> bounded W^X callable baseline code object
  -> exact VM/native adapter or unboxed direct native call
  -> PollV1 and structured return/trap/exit/deadline/resource/host status
```

Forced mode enters generated main and never falls back. Auto compiles at one
function entry and uses the object only on later calls; unsupported code stays
VM-correct with same-epoch retry suppression. The old observation-only hook is
removed. Loop OSR, background compilation, optimizing tiers, native references
or allocation, persistent profiles, and persistent code caches are absent.

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
structured process-safe outcomes, bounded VM execution, deterministic
fixed-point function effects, typed SSA, verification, independent evaluation,
baseline normalization, reference-bytecode cutover, and the owned low-level
x86-64/W^X foundation are now Current. SSA-to-native lowering and exact VM/code-
object tier ownership follow. The first adaptive execution target remains
synchronous callable baseline JIT. Exact native roots/allocation and a
proof-based optimizing tier now precede later loop OSR in the accepted sequence.
The allocation-free scalar callable baseline tier is Current; minimal file
emission remains only for diagnostics and backend tests, and offline PGO is
rejected. The exact active boundary is [Callable Linux x86-64 Baseline JIT
Cycle](../decisions/callable-baseline-jit.md); the next contracts are
[Allocation-Capable Baseline JIT](../decisions/allocation-capable-baseline-jit.md)
and [Proof-Based Optimizing JIT](../decisions/proof-based-optimizing-jit.md).
Real modules, process-safe host services, byte strings/views, and measured
memory strategies build on those layers as vertical slices.
