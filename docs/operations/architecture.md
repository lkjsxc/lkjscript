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
| Ownership analysis | `crates/lkjscript-compiler/src/ownership.rs` | mandatory aggregate-bounded `Owned Buf` lexical place/move/same-block-loan analysis and exact joins |
| HIR-to-SSA conversion | `crates/lkjscript-compiler/src/ssa.rs` | environment renaming, BindingId-ordered branch/loop parameters, exact operation/type/effect/ownership transfer |
| Typed SSA authority | `crates/lkjscript-ir/src/` | IR model, `verify`, `evaluate`, isolated baseline passes, bounded proof optimization/certificate verification, bytecode link metadata |
| Type representation | `crates/lkjscript-compiler/src/types/` | canonical source/HIR Type parsing and substitution |
| SSA bytecode lowering | `crates/lkjscript-compiler/src/codegen/` | `compile_program`; no sibling HIR semantic emitter |
| Owned x86-64 foundation | `crates/lkjscript-native/src/` | closed scalar/reference machine plan, verifier-owned bounded liveness certificates, exact typed maps plus private structural requirements, ABI-2 reservation/encoding, opaque installable image |
| Verified SSA/native runtime adapter | `crates/lkjscript-jit/src/` | scalar plus host-independent GC lowering, `GcHeap` runtime services, code objects, tier state, forced/auto execution |
| Shared bytecode/value ABI | `crates/lkjscript-core/src/` | `Chunk`, `Op`, `Value`, `HeapObj` |
| VM loop | `crates/lkjscript-vm/src/run.rs`, `run/` | `Vm::run`, dispatch and calls |
| Heap/GC | `crates/lkjscript-core/src/gc.rs` | pure session-owned non-reusing stable-index `GcHeap`, transactional estimated-byte-accounted mutation, transitive snapshots, bounded counters/collection policy, VM and forced-JIT use |
| Host resources | `crates/lkjscript-vm/src/host*.rs` | IO, buffers, descriptor table |
| Linux FFI and W^X | `crates/lkjscript-sys/src/` | owned file/socket/time/ioctl wrappers, safe bounded executable installation/invocation, private raw active-frame chain, copied typed-root service callback |
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
  -> enforce the aggregate ownership-expression budget
  -> run mandatory whole-place lexical ownership/same-block-loan analysis
  -> infer stable fixed-point function effects and recompute expression effects
  -> environment-rename HIR locals/mutation into typed SSA block parameters
     with explicit place init/end and current-owner transport facts
  -> verify typed SSA with bounded forward ownership CFG dataflow
  -> run each deterministic isolated baseline pass with post-pass verification
  -> lower only normalized SSA and retain deterministic bytecode link metadata
  -> install internal function closures as implementation metadata
  -> mutable Chunk builder
  -> validate_chunk -> opaque immutable ValidatedChunk
  -> ExecutableProgram { verified SSA, link metadata, ValidatedChunk }
      +-> vm: run_chunk_with_args(program.bytecode(), ExecutionConfig)
      +-> baseline-jit: VerifiedProgram scalar/reference SCC group -> ABI-2 baseline object
          -> typed frame-home HeapDispatchV1 -> session GcHeap -> native main
      +-> optimizing-jit: bounded stable-ID proof edits -> private reconstruction
          -> VerifiedOptimizedProgram -> shared lowering -> optimizing-only object/main entry
      +-> auto: VM entries -> bounded hotness -> later baseline native function calls
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
bounded code objects, callable function-entry baseline tier, host-independent
reference/allocation SCC groups in forced mode, and the forced first proof-based
optimizing pipeline are **Current**. Forced and auto baseline engines plus forced
optimizing execution enter real generated code. The initial `Owned Buf`
ownership safe island, marker traits, and closed-plan ABI-2 typed-reference
frames/maps are Current;
general ownership, Handle/host native allocation, native/VM reference transitions,
automatic optimizing promotion, broader optimization passes, loop OSR,
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
forced main or hot scalar VM function entry
  -> verified scalar or host-independent reference eligibility and reachable SCC group
  -> synchronous typed-SSA lowering at a safepoint
  -> bounded W^X callable ABI-2 baseline code object
  -> one invocation-time pthread stack-bounds query, then cached descriptor/budget/bounds frame reservation before each stack subtraction
  -> initialized registered frame and verifier-certified exact scalar or typed-reference call map
  -> unboxed direct call or canonical-fact verified-home HeapDispatchV1 safe runtime service
  -> GcHeap collection/allocation with root writeback, argument re-materialization,
     transactional mutation, and transitive owned return snapshot
  -> PollV1/CollectReferenceV1 and structured return/trap/exit/deadline/resource/host status
  -> exactly one unregister on every registered outcome
```

Forced baseline and optimizing modes enter generated main and never fall back.
The optimizing mode verifies the bounded complete proof before source effects,
lowers only opaque `VerifiedOptimizedProgram`, installs only optimizing objects,
and retains certificate/accounting metadata. Auto compiles at one
eligible scalar-adapter function entry and uses the baseline object only on later calls;
reference-signature helpers may be generated direct callees but remain
ineligible VM/native entries. Unsupported code stays VM-correct with same-epoch
retry suppression. The old observation-only hook is
removed. Closed plans retain exact Buf-reference collection. Forced SSA/source execution
also supports Str, legacy Buf, Product, List, Option, and Result allocation and
direct/mutual recursion. Auto intentionally keeps reference-typed functions in
VM because reference transitions remain absent. Loop
OSR, automatic optimizing promotion, broader proof passes, background
compilation, speculative tiers, persistent profiles, and persistent code caches
are absent.

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
The forced first proof pipeline and callable baseline tier are Current;
automatic optimizing promotion is not. Minimal file
emission remains only for diagnostics and backend tests, and offline PGO is
rejected. The exact active boundary is [Callable Linux x86-64 Baseline JIT
Cycle](../decisions/callable-baseline-jit.md); the next contracts are
[Allocation-Capable Baseline JIT](../decisions/allocation-capable-baseline-jit.md)
and [Proof-Based Optimizing JIT](../decisions/proof-based-optimizing-jit.md).
Real modules, process-safe host services, byte strings/views, and measured
memory strategies build on those layers as vertical slices.

The containing host-independent allocation slice based on `0daa7a0` passed the
focused cross-crate tests, strict affected Clippy, docs/tree/source checks,
`quiet verify` (182 unit/integration tests plus one compile-fail doctest), locked
release build, scalar/hello/Brainfuck smokes, and forced allocation-graph smoke
described in [Current State](../current-state.md). Docker and performance were
not run.
