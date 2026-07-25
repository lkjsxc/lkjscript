# Typed Compiler Pipeline And Runtime JIT

## Purpose

Define one semantic pipeline shared by the reference VM, runtime JIT, minimal
native file-emission tests, and future Wasm so performance backends cannot
reinterpret the language.

## Status

**Current** for parsed AST -> resolved typed HIR -> mandatory initial `Owned
Buf` ownership analysis with fixed-point function effects -> verified typed SSA -> verified baseline normalization -> reference
bytecode. The independent bounded SSA evaluator and bytecode link metadata are
also Current. The selected owned Linux x86-64 scalar machine-plan encoder, safe W^X boundary,
narrow verified-SSA adapter, bounded code objects, forced/automatic callable
baseline tier, host-independent native references/allocation, and the forced
first proof-based optimizing slice are **Current**. Synchronous automatic
proof promotion is an **Accepted Implementation Selection**, not Current.
Handle/host native calls, native/VM reference transitions, loop OSR, minimal AOT
test emission, production AOT, content-addressed cache, and direct Wasm are
**Accepted Targets**. The backend is specified by [Linux x86-64 Native
Backend](linux-x86-64-native-backend.md). Optional explicit local PGO is a
**Deferred Optional Target**, not Current, under [Measured Execution
Portfolio](execution-portfolio.md).

## Pipeline

The accepted pipeline is:

```text
canonical source
  -> parsed AST
  -> resolved typed HIR
  -> typed SSA
  -> verified optimization pipeline
      +-> reference bytecode
      +-> native code object backend
              +-> test/AOT emission harness
              +-> baseline JIT code objects
              +-> optimizing JIT code objects
      +-> direct Wasm later
```

Typed HIR owns resolved binding IDs, declaration kinds, nominal product IDs and
field indexes, exact static type facts, source origins, canonical operation
identities and per-call signatures, per-expression effects, and compact per-
function effect summaries. One compiler lowering environment-renames HIR
locals and mutation into `lkjscript-ir`; code generation consumes only verified
normalized SSA rather than HIR, definitions, parameters, operators, or names.

The initial `Owned Buf` safe island retains exact lexical place initialization/
end, current-owner transport, move, borrow, loan, kind, and alias identities in
HIR/SSA. The public verifier applies bounded forward CFG ownership dataflow and
exact joins after every pass. Generic ownership/reference substitution,
cross-block Borrow results, and aggregate storage are rejected in this slice;
verified reference-bytecode lowering may erase wrappers to the existing arena
handle and the scalar baseline rejects the new types/operations explicitly. `Unit`, typed empty lists, Option, immutable nominal products, and the explicit
value/object/list/F64-bit equality families have distinct exact HIR/runtime
semantics, and every `if` has
exactly three operands with matching branch types. Nil, universal equality, and
the legacy runtime-union escape hatch are absent. Direct calls through resolved
function storage use the canonical callee's inferred summary; calls through
parameters or locals, and any missing provenance, conservatively carry every
effect. Call arguments always retain their independently inferred effects.

Typed SSA uses dense deterministic function/block/value identities, explicit
ordered basic-block parameters, exact scalar/product/collection/function types,
structured terminators, canonical operation identities, trap/outcome facts,
calls, effects, safepoints, frame states, and source origins. It is the sole
optimization authority for reference lowering, JIT, the AOT test surface, and
direct Wasm. The active cycle may retain HIR-to-bytecode only while differential
tests establish SSA; it then lowers reference bytecode from verified normalized
SSA and deletes the sibling semantic lowering before native lowering is
authoritative. Runtime JIT and file emission differ in code placement,
relocation, and linking, not semantics or optimization ownership.

The completed cutover changes the compiler result from a bare bytecode value to
an `ExecutableProgram` that owns both the opaque validated reference bytecode
and verified normalized SSA, plus deterministic function-to-prototype/main and
SSA-to-bytecode link metadata. Bytecode remains available through an explicit
accessor. Mutable raw chunks remain builder inputs only for malformed-bytecode
tests and always cross `validate_chunk` before execution.

The Current differential evaluator is independent of bytecode, the VM, native
helpers, and host effects. It covers deterministic scalar and control
semantics, calls and recursion, SSA-converted local mutation, products,
Option/Result, lists, strings, deterministic arguments, and host-independent
buffers under explicit fuel, frame, allocation, buffer, and list-comparison
bounds. Console, filesystem, sockets, terminal, time, process-global handles,
and other host operations are explicit unsupported-evaluator results rather
than inert behavior.

Current baseline normalization runs only independently testable deterministic
passes, verifying after each: constant folding/propagation with no F64
arithmetic fold, copy propagation, branch simplification, unreachable-block
elimination, empty-block forwarding, effect-aware dead-instruction elimination,
direct-call resolution, and canonical block/fallthrough order. There is no
speculation, native lowering, engine selection, OSR, guard, or deoptimization
in this cutover.

The **Current forced first** optimizing boundary starts from this verified
baseline form and returns an opaque separately verified optimized program. Its
certificate vocabulary is exact scalar algebraic replacement and local/
dominator-ordered value numbering, followed by the existing cleanup passes.
The separate bounded certificate verifier reconstructs the candidate from
checked stable-ID edits on a private clone, requires exact candidate equality,
and runs the ordinary SSA verifier again. Baseline normalization is not relabeled
as optimization, and unchecked optimizer output cannot reach optimizing native
lowering. `auto` remains baseline-only; automatic promotion and broader passes
remain Accepted Targets.

## HIR Resolution Invariants

- Every name resolves once to a stable binding ID.
- Duplicate declarations are errors.
- Unknown symbols are errors and never become implicit globals.
- Locals use lexical shadowing consistently in analysis and lowering.
- Binding kind distinguishes immutable local, local var, parameter, function,
  const, static, capability, and future explicit reference cells.
- Calls resolve to canonical operations or declarations before codegen.
- Function summaries are the least monotone finite-bitset fixed point in stable
  BindingId order; call cycles add divergence without unrelated effects.
- Every expression effect is recomputed from final summaries, including main.
- Types and arity attach to HIR nodes and are not recomputed by backends.
- Imported source origin remains available for machine-readable diagnostics.

The first HIR slice preserved accepted workload behavior while deleting
duplicate resolution/lowering. It rejects unknown or duplicate bindings,
operation-name collisions, unresolved generic variables, and invalid global
assignments. `set` yields Unit, and top-level control-flow fragments use correct
absolute jump targets. Semantic-core changes continue as complete HIR-to-VM
vertical slices.

## Runtime Representations

The reference VM retains compact tagged values for cold execution and
conformance. Typed SSA/native hot paths do not carry universal `Value` when a
static representation exists:

- I64 and F64 use native scalar registers;
- Bool and Unit use canonical scalar or zero-sized representations;
- Str, Bytes, Slice, and views use typed pointer/length layouts;
- products flatten or follow an explicit target ABI;
- Option uses a proven type-specific niche or explicit tag;
- generic code is monomorphized where bounded code growth justifies it;
- heap references remain typed and visible in precise stack maps.

Dynamic dispatch and runtime type tests remain only when an explicit dynamic
interface requires them. VM/native adapters own boxing and unboxing; compatible
native callers call compiled callees directly.

## Native Code Object Priority

The required called-code experiment selected the
[Linux x86-64 Native Backend](linux-x86-64-native-backend.md): a small owned
encoder over Cranelift 0.134.2 from generated-code speed plus visible
compilation, binary, RSS, dependency, safety, stack-map, W^X, and maintenance
costs.

The Current low-level foundation implements its closed typed scalar machine
plan, verifier, deterministic stack-slot x86-64 encoder, relocations, ABI/frame
and scalar-stack-map metadata, opaque installable image, and bounded safe
`lkjscript-sys` RW-to-RX installation/invocation. Boundary tests call generated
multi-block, loop, direct-call, allowlisted-runtime-call, exact I64/F64,
trap/exit code. Safe APIs expose no arbitrary bytes, addresses, mappings, or
function pointers; unsafe memory and invocation remain in `lkjscript-sys`.

The foundation itself remains source/SSA-independent. The separate
`lkjscript-jit` adapter now consumes only `VerifiedProgram` for baseline or the
opaque `VerifiedOptimizedProgram` for optimizing code, and lowers eligible
scalar groups into the closed plan, attaches versioned PollV1/entry calls and
bounded code-object/VM tier ownership, and executes generated entries. Callable
baseline JIT is the current first adaptive performance path for this exact
subset. File emission exists only to inspect code,
test relocations and ABI behavior, use external debuggers, and compare backend
output with the VM.

Native target modes are explicit and every result records its target. Linux
x86-64 is the only acceptance platform for the active callable-baseline cycle;
Linux AArch64 begins only afterward. Backend-independent typed SSA and native
ABI boundaries must avoid accidental x86-specific semantic assumptions.

## Runtime Tiering

Normal `auto` execution starts in the VM. Bounded saturating function-entry
counters currently identify hot scalar functions without per-instruction
profiling. Loop-backedge tier counters are not implemented. The accepted native
sequence is:

1. synchronous whole-function baseline JIT with low-cost non-speculative passes;
2. loop-triggered compilation and verified loop-header OSR;
3. proof-based optimizing JIT from types, SSA, effects, and ownership;
4. guarded runtime specialization and deoptimization only when later evidence
   justifies them.

The baseline tier must not need deoptimization. Current-process observations
are local, bounded, discarded at process exit, and never telemetry. Persistent
profiles and cross-run native caches are outside this plan.

The complete state machine, OSR mapping, executable-code object, W^X, code-cache,
GC stack-map, failure, engine-selection, benchmark, and adoption contracts are
in [Runtime JIT Instead of Offline PGO](runtime-jit-instead-of-offline-pgo.md).
The mandatory Linux x86-64 completion slice is
[Callable Linux x86-64 Baseline JIT Cycle](callable-baseline-jit.md).

## Wasm

The VM compiled to Wasm remains a conformance/reference path. Direct
SSA-to-Wasm is the later browser performance path. It follows the same typed
semantics and verified optimization pipeline and does not wait on an offline
profile pipeline.

## Optimization Trust

Every optimization preserves typed-IR semantics and is tested against the
reference evaluator/VM. Assumptions require proof or explicit guards and exact
side exits. Undefined behavior is not used to make invalid programs appear
fast. Differential, property, boundary, and corpus tests precede performance
adoption.

## Adoption Gates

A backend or optimization candidate records:

1. semantic and native ABI version;
2. differential values, output, traps, and outcomes against the VM;
3. malformed-input and resource-limit behavior;
4. compile time, trigger time, first native execution, startup, total and
   steady-state runtime, code/metadata size, code-cache peak, and RSS;
5. target CPU and enabled proven assumptions;
6. isolated and combined variants;
7. compilation, fallback, OSR, guard-failure, and deoptimization counts where
   applicable;
8. adoption/rejection thresholds and artifact cleanup.

Forced JIT tests fail when required code cannot compile or execute natively;
they cannot silently pass through the VM. A machine-code emission test is not
proof that native code executed.

## Rejected

- Lowering untyped AST independently in each backend.
- Carrying tagged universal `Value` through typed native hot paths.
- Treating the observation-only JIT hook as a native-code boundary.
- Treating the minimal AOT emitter as the primary optimization strategy.
- Offline PGO, training builds, profile merging, and profile-use rebuilds.
- Persistent cross-run JIT profiles or native-code caches without a new decision.
- Making file emission and JIT use different semantic IRs.
- Trusting AI-authored optimization hints.
