# Architecture: Crate Graph

[Authority](../architecture.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Crate Graph

```text
jit -> executable -> native -> contracts
 |         app -> linux-host -> contracts + resource
 |          |                       |
 +-> ir + core + resource           +-> core

vm -> core + host + jit + sys
runtime -> contracts + core + host + vm
app -> contracts + core + compiler + ir + host + jit + linux-host + resource + runtime + vm
```

The exact internal product dependency edges are:

- `lkjscript-contracts`, `lkjscript-host`, and `lkjscript-sys` have no dependencies;
- `lkjscript-core -> contracts`; `lkjscript-native -> contracts`;
- `lkjscript-resource -> contracts + core`; `lkjscript-ir -> contracts + core + resource`;
- `lkjscript-compiler -> contracts + core + ir`;
- `lkjscript-executable -> native`;
- `lkjscript-linux-host -> contracts + resource`;
- `lkjscript-jit -> core + executable + ir + native + resource`;
- `lkjscript-vm -> core + host + jit + sys`;
- `lkjscript-runtime -> contracts + core + host + vm`;
- `lkjscript-database -> host`;
- `lkjscript-app -> contracts + core + compiler + ir + host + jit + linux-host + resource + runtime + vm`;
- `lkjscript-xtask -> contracts + compiler + core`.

The app test target additionally uses `lkjscript-native`. No workspace crate has
a third-party Rust dependency.
## Ownership Map

- **CLI:** `crates/lkjscript-app/src/main.rs` — `main`, `real_main`.
- **Public compiler API:** `crates/lkjscript-compiler/src/lib.rs`.
  Entries: `compile_path`, `compile_path_with_sources`, `compile_source`, `validate_source`.
  <!-- LKJ-EXACT-DATA -->
- **Validated Semantic Source:** `crates/lkjscript-compiler/src/source/`.
  Owns `ValidatedSourceTree`, contained loading, parsing, bounds, identities, formatting, and diagnostics.
  <!-- LKJ-EXACT-DATA -->
- **Resolution and typed HIR:** compiler `analyze/`, `effects/`, `hir/`, and `operation/`.
  Owns fixed-point effects, bindings, local slots, and typed operations.
  <!-- LKJ-EXACT-DATA -->
- **Ownership analysis:** `crates/lkjscript-compiler/src/ownership/`.
  Owns bounded affine place, move, same-block-loan, and join analysis.
  <!-- LKJ-EXACT-DATA -->
- **HIR-to-SSA:** `crates/lkjscript-compiler/src/ssa/`.
  Owns environment renaming, ordered CFG parameters, and typed ownership transfer.
  <!-- LKJ-EXACT-DATA -->
- **Typed SSA:** `crates/lkjscript-ir/src/`.
  Owns IR, verification, evaluation, baseline passes, and proof optimization/checking.
  <!-- LKJ-EXACT-DATA -->
- **Types:** `crates/lkjscript-compiler/src/types/` — source/HIR parsing and substitution.
- **Bytecode lowering:** `crates/lkjscript-compiler/src/codegen/` — `compile_program`.
  <!-- LKJ-EXACT-DATA -->
- **x86-64 foundation:** `crates/lkjscript-native/src/`.
  Owns typed plans, frame homes, runtime sites, encoding, and installable images.
  <!-- LKJ-EXACT-DATA -->
- **Native adapter:** `crates/lkjscript-jit/src/`.
  Owns scalar/structural/unique/region lowering, code objects, tiers, and execution.
  <!-- LKJ-EXACT-DATA -->
- **Bytecode/value ABI:** `crates/lkjscript-core/src/`.
  Owns `Chunk`, `Op`, `Value`, structural images, lists, region products, and outcomes.
- **VM loop:** `crates/lkjscript-vm/src/run/` — dispatch, calls, and deterministic storage.
- **Host resources:** `crates/lkjscript-vm/src/host*.rs` — IO, byte views, and descriptors.
- **Executable mechanism:** `crates/mechanisms/lkjscript-executable/src/`.
  Owns W^X installation, invocation, active frames, and typed runtime callbacks.
  <!-- LKJ-EXACT-DATA -->
- **Linux topology:** `crates/mechanisms/lkjscript-linux-host/src/`.
  Owns bounded observation, affinity, and worker binding.
  <!-- LKJ-EXACT-DATA -->
- **Residual host/SQLite:** `crates/lkjscript-sys/src/`.
  Owns file/path/socket/time/poll/random/terminal wrappers and SQLite FFI.
  <!-- LKJ-EXACT-DATA -->
- **Repository gates:** `crates/lkjscript-xtask/src/` — verification and structure checks.
- **Language library:** `src/std/` — imported standard definitions.
- **Validation package:** `src/lib/lkjedit/` — editor state and control.
- **Executables:** `src/examples/` — examples and benchmarks.
## Compile Flow

```text
CLI path
  -> compile_path
  -> package-root and import resolution through an explicit dependency-first stack
  -> checked source-unit/file/closure implementation maxima
  -> lex/parse each source with exact spans and trivia
  -> enforce unchanged canonical source per-file/tree limits
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
      +-> baseline-jit: VerifiedProgram typed SCC group -> canonical native baseline object
          -> structural or typed invocation-region service -> native main
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
projection from the validated tree; ordinary compilation requires the exact Current
source and semantic contract digests,
and no sibling parser or raw public AST can enter analysis. HIR owns an explicit Main
and Functions, resolved binding IDs and local slot references, immutable
declaration kinds, MutableLocal/SetLocal nodes, nominal product IDs and field
indexes, exact static type facts, source origins, canonical operation identities
and per-call signatures, compact fixed-point function summaries, and final
per-expression effects. SSA owns backend control/data flow, exact types,
effects, proof frame states, cleanup plans, and deterministic bytecode links. Direct
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
optimizing execution enter real generated code. The affine byte-vector and whole-owner slice
ownership safe island, marker traits, and closed-plan canonical native contract typed
frame homes are Current;
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
