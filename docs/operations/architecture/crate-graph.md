# Architecture: Crate Graph

[Authority](../architecture.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Crate Graph

```text
                         lkjscript-contracts
                       /        |          \
           lkjscript-ir   lkjscript-core   lkjscript-native
                  \          /       \          /       \
                   compiler           native-cache       sys
                         \             /  \              /
                          app ---- lkjscript-jit ---- vm
                           \
                            xtask
```

The actual product dependency edges are:

- `lkjscript-contracts` has no internal dependency
- `lkjscript-ir -> contracts`; `lkjscript-core -> contracts`
- `lkjscript-native -> contracts`
- `lkjscript-compiler -> contracts + lkjscript-ir + lkjscript-core`
- `lkjscript-native-cache -> contracts + core + native`
- `lkjscript-sys -> native`
- `lkjscript-jit -> contracts + ir + core + native + native-cache + sys`
- `lkjscript-vm -> core + jit + sys`
- `lkjscript-app -> compiler + ir + core + jit + vm`
- `lkjscript-xtask -> compiler + core`

The app test target also uses `lkjscript-ir` for evaluator/VM differential
checks. No workspace crate has a third-party Rust dependency.
## Ownership Map

| Concern | Primary location | Entry symbols |
| --- | --- | --- |
| CLI | `crates/lkjscript-app/src/main.rs` | `main`, `real_main` |
| Public compiler API | `crates/lkjscript-compiler/src/lib.rs` | `compile_path`, `compile_path_with_sources`, `compile_source`, `validate_source` | <!-- LKJ-EXACT-DATA -->
| Validated Semantic Source foundation | `crates/lkjscript-compiler/src/source/` | opaque `ValidatedSourceTree`, iterative contained loading, parser/limits, spans/origins, revision/keys/nodes, structural formatter, source diagnostics | <!-- LKJ-EXACT-DATA -->
| Resolution and typed HIR | `crates/lkjscript-compiler/src/analyze/`, `effects/`, `hir/`, `operation/` | `analyze_program`, fixed-point effect inference, explicit Main/Function, BindingId, local slots, typed operations/effects | <!-- LKJ-EXACT-DATA -->
| Ownership analysis | `crates/lkjscript-compiler/src/ownership/` | mandatory aggregate-bounded `Owned Buf` lexical place/move/same-block-loan analysis and exact joins | <!-- LKJ-EXACT-DATA -->
| HIR-to-SSA conversion | `crates/lkjscript-compiler/src/ssa/` | environment renaming, BindingId-ordered branch/loop parameters, exact operation/type/effect/ownership transfer | <!-- LKJ-EXACT-DATA -->
| Typed SSA authority | `crates/lkjscript-ir/src/` | IR model, `verify`, `evaluate`, isolated baseline passes, bounded proof optimization/certificate verification, bytecode link metadata | <!-- LKJ-EXACT-DATA -->
| Type representation | `crates/lkjscript-compiler/src/types/` | canonical source/HIR Type parsing and substitution |
| SSA bytecode lowering | `crates/lkjscript-compiler/src/codegen/` | `compile_program`; no sibling HIR semantic emitter | <!-- LKJ-EXACT-DATA -->
| Owned x86-64 foundation | `crates/lkjscript-native/src/` | closed scalar/reference machine plan, verifier-owned bounded liveness certificates, exact typed maps plus private structural requirements, canonical native contract reservation/encoding, opaque installable image | <!-- LKJ-EXACT-DATA -->
| Persistent native image cache | `crates/lkjscript-native-cache/src/` | exact keys, bounded lookup, canonical image decode, atomic local publication | <!-- LKJ-EXACT-DATA -->
| Verified SSA/native runtime adapter | `crates/lkjscript-jit/src/` | scalar plus host-independent GC lowering, verified image cache integration, `GcHeap` runtime services, code objects, tier state, forced/auto execution | <!-- LKJ-EXACT-DATA -->
| Shared bytecode/value ABI | `crates/lkjscript-core/src/` | `Chunk`, `Op`, `Value`, `HeapObj` |
| VM loop | `crates/lkjscript-vm/src/run/` | `Vm::run`, dispatch and calls |
| Heap/GC | `crates/lkjscript-core/src/gc/` | pure session-owned non-reusing stable-index `GcHeap`, transactional estimated-byte-accounted mutation, transitive snapshots, bounded counters/collection policy, VM and forced-JIT use | <!-- LKJ-EXACT-DATA -->
| Host resources | `crates/lkjscript-vm/src/host*.rs` | IO, buffers, descriptor table |
| Linux FFI and W^X | `crates/lkjscript-sys/src/` | owned file/socket/time/ioctl wrappers, safe bounded executable installation/invocation, private raw active-frame chain, copied typed-root service callback | <!-- LKJ-EXACT-DATA -->
| Repository gates | `crates/lkjscript-xtask/src/` | `quiet verify`, source/tree/doc checks |
| Language library | `src/std/` | imported `std/...` definitions |
| Validation package | `src/lib/lkjedit/` | editor state and control loop |
| Executables | `src/examples/` | hello, Mandelbrot, HTTP, benchmark, editor |
## Compile Flow

```text
CLI path
  -> compile_path
  -> package-root and import resolution through an explicit dependency-first stack
  -> checked source-unit/file/closure implementation maxima
  -> lex/parse each source with exact spans and trivia
  -> enforce unchanged the removed legacy source contract per-file/tree limits
  -> build opaque ValidatedSourceTree with exact revision, stable keys, and nodes
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
      +-> baseline-jit: VerifiedProgram scalar/reference SCC group -> canonical native contract baseline object
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

Opaque validated Semantic Source Foundation tree -> resolved typed HIR ->
verified typed SSA -> verified baseline normalization -> reference bytecode is
**Current**. HIR consumes the private mechanically checked exact-edition form
projection from the validated tree; ordinary compilation requires the canonical source contract,
and no sibling parser or raw public AST can enter analysis. HIR owns an explicit Main
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
ownership safe island, marker traits, and closed-plan canonical native contract typed-reference
frames/maps are Current;
general ownership, Handle/host native allocation, native/VM reference
transitions, broader optimization passes, loop OSR, minimal AOT file emission,
and direct Wasm remain **Accepted Targets** or later work. Automatic optimizing
promotion has an exact **Accepted Implementation Selection** but is not Current. The VM remains the
cold tier and runtime oracle. See [Ownership And
Borrowing](../../decisions/semantics/ownership-and-borrowing.md), [Coherent Traits And Static
Dispatch](../../decisions/semantics/traits-and-static-dispatch.md), [Native References,
Frames, And Exact GC Stack Maps](../../decisions/jit/native-references-and-gc-stack-maps.md),
[Allocation-Capable Baseline JIT](../../decisions/jit/allocation-capable-baseline-jit.md),
[Proof-Based Optimizing JIT](../../decisions/jit/proof-based-optimizing-jit.md), [Typed
Compiler Pipeline And Runtime JIT](../../decisions/execution/compiler-pipeline.md), [Linux
x86-64 Native Backend](../../decisions/execution/linux-x86-64-native-backend.md), and [Runtime
JIT Instead of Offline PGO](../../decisions/jit/runtime-jit-instead-of-offline-pgo.md).
