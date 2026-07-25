# Architecture

## Purpose

Give humans and coding agents a direct map from product behavior to the files
that own it.

## Status

**Current** for the implementation map. Semantic Source Foundation V1 is
Current; complete Schema/Agent Protocol V1, transactions, typed holes, and
semantic derived-fact queries remain **Accepted Targets**. Synchronous automatic
proof promotion remains an **Accepted Implementation Selection** but is no
longer the immediate priority.

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
| Validated Semantic Source foundation | `crates/lkjscript-compiler/src/source/` | opaque `ValidatedSourceTree`, iterative contained loading, parser/limits, spans/origins, revision/keys/nodes, structural formatter, source diagnostics |
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
  -> package-root and import resolution through an explicit dependency-first stack
  -> checked source-unit/file/closure implementation maxima
  -> lex/parse each source with exact spans and trivia
  -> enforce unchanged Edition 1 per-file/tree limits
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

Opaque validated Semantic Source Foundation tree -> resolved typed HIR ->
verified typed SSA -> verified baseline normalization -> reference bytecode is
**Current**. HIR currently consumes a private mechanically checked Edition 1
form projection from the validated tree; no sibling parser or raw public AST can
enter analysis. HIR owns an explicit Main
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
general ownership, Handle/host native allocation, native/VM reference
transitions, broader optimization passes, loop OSR, minimal AOT file emission,
and direct Wasm remain **Accepted Targets** or later work. Automatic optimizing
promotion has an exact **Accepted Implementation Selection** but is not Current. The VM remains the cold tier and runtime oracle. See [Ownership And
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
VM because reference transitions remain absent. Loop OSR, automatic optimizing promotion, broader proof passes, background
compilation, speculative tiers, persistent profiles, and persistent code caches
are absent. The selected but unimplemented automatic flow is:

```text
VM root entries --64--> synchronous baseline install; triggering call stays VM
later scalar root entry -> exact Baseline(function, object, tier) token
  -> count exact baseline entries of that root
  --N--> capture current baseline token/object
          -> synchronous bounded proof/check/lower/W^X install
          -> OptimizingPending; invoke captured baseline object
  -> later root entry validates/publishes pending token -> OptimizingNative
```

N is CLI-opt-in and candidate-controlled at 64/256/1,024/4,096; optimizing is
disabled by default until retained adoption. The process-local session owns
coexisting baseline/optimizing objects, one current and optional pending
selection, and bounded stale mappings until drop. Epoch changes invalidate
optimized selection back to baseline; stale tokens cannot be selected. One
attempt per epoch, a bounded total, same-epoch suppression, and structured tier
failure are architectural boundaries, not optimizer hints. Source main and all
reference VM/native entries remain VM-only, while generated reference helpers
may call and allocate internally. There is no OSR, background compile,
deoptimization, guard, or speculation.

## Source Layout Rule

The current Edition 1 language rule limits each lkjscript source directory to
16 immediate entries, counting files and subdirectories together. Rust crates,
documentation, metadata, `.git`, and build output are not language source and
are outside this rule.

The repository gate checks the complete in-tree language corpus. The compiler
also rejects an entry or imported source directory that violates the rule, so
an external project receives the same contract. The accepted destination is an
AI-maintainability lint, but this check is not weakened until aggregate source
closure/import/byte/node safety bounds are Current. See [Resource Budget
Profiles](../decisions/resource-budget-profiles.md).

## Change Guide

- Change source semantics/projection: Semantic Source schema/validator, edition adapter/formatter, language docs, complete mechanical corpus migration, semantic transaction tests, and negative fixtures; backends never interpret spelling.
- Change types: language docs, type prelude/inference, lowering, VM behavior, and conformance tests.
- Add an opcode: core ABI, code generation, dispatch, disassembly, and malformed-bytecode validation.
- Add host capability: accepted decision, sys safety wrapper, VM resource boundary, typed prelude, script policy wrapper, and failure tests.
- Change limits: language decision, shared core constant, compiler enforcement, repository gate, and boundary tests.
- Change packaging: imports decision, resolver, installed layout, Docker/native bundle, and external-project smoke.

## Accepted Redesign Direction

Explicit main, effect-free imported libraries, local-only mutation,
product-threaded editor/terminal/Brainfuck state, whole-chunk validation,
structured process-safe outcomes, bounded VM execution, deterministic
fixed-point effects, resolved typed HIR, verified typed SSA, independent
evaluation, baseline normalization, reference bytecode, exact roots, owned
x86-64/W^X code, callable baseline execution, and forced proof-checked
optimizing execution are Current.

The accepted Target architecture is:

```text
goal/specification
  -> versioned Semantic Source with typed holes
  -> opaque validated source graph and deterministic Edition adapter
  -> resolved typed Core HIR
  -> ownership/effect/capture/capability analysis
  -> verified semantic SSA
  -> verified memory/region/drop lowering
  -> deterministic baseline normalization
  -> optional proof-checked optimization
  -> target-neutral verified machine plan
      +-> deterministic evaluators
      +-> validated portable VM artifact
      +-> baseline native compiler
      +-> optimizing JIT
      +-> AOT/cache
      +-> Wasm/components
```

[Semantic Source And Agent
Protocol](../decisions/semantic-source-and-agent-protocol.md) now has a Current
Foundation V1: one validated source-tree authority, exact 113-file tracked
source roundtrip, exact revision identity, stable declaration keys, dense nodes,
and structural source diagnostics. Existing HIR/SSA/VM/JIT behavior remains
unchanged through that cutover, and no sibling parser/tree path independently
feeds a backend. Atomic semantic edits, resolved-reference facts, the remaining
structured compiler diagnostics, typed holes, and bounded protocol transport
are the next implementation boundary and are not Current.

[AI-Native Language And Platform](../decisions/ai-native-platform.md) owns the
long-term dependency order. [Resource Budget
Profiles](../decisions/resource-budget-profiles.md) prevents weakening tiny
Current limits before aggregate replacements exist. [Measured Execution
Portfolio](../decisions/execution-portfolio.md) accepts later AOT, cache,
optional local PGO, and Wasm measurement without making them Current. The
process-local automatic proof-promotion contract remains selected and disabled
by default, but no longer pre-empts the Semantic Source foundation.

The containing host-independent allocation slice based on `0daa7a0` passed the
focused cross-crate tests, strict affected Clippy, docs/tree/source checks,
`quiet verify` (182 unit/integration tests plus one compile-fail doctest), locked
release build, scalar/hello/Brainfuck smokes, and forced allocation-graph smoke
described in [Current State](../current-state.md). Docker and performance were
not run.
