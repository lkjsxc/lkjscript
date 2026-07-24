# Typed Compiler Pipeline And Runtime JIT

## Purpose

Define one semantic pipeline shared by the reference VM, runtime JIT, minimal
native file-emission tests, and future Wasm so performance backends cannot
reinterpret the language.

## Status

**Current** for parsed AST -> resolved typed HIR -> reference bytecode. Typed
SSA, native code objects, baseline JIT, proof-based optimizing JIT, minimal AOT
test emission, and direct Wasm are **Accepted Targets**. Native compilation is
not implemented. The future Linux x86-64 backend is the owned encoder selected
by [Linux x86-64 Native Backend](linux-x86-64-native-backend.md). Offline PGO is
**Rejected by Product Decision** in
[Runtime JIT Instead of Offline PGO](runtime-jit-instead-of-offline-pgo.md).

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

Typed HIR currently owns resolved binding IDs, declaration kinds, nominal
product IDs and field indexes, exact static type facts, source origins,
canonical operation identities and per-call signatures, and conservative
effects. Code generation consumes HIR rather than
re-parsing definitions, parameters, operators, and names independently.

`Unit`, typed empty lists, Option, immutable nominal products, and the explicit
value/object/list/F64-bit equality families have distinct exact HIR/runtime
semantics, and every `if` has
exactly three operands with matching branch types. Nil, universal equality, and
the legacy runtime-union escape hatch are absent. User calls conservatively
carry every effect until fixed-point function summaries replace the safe
over-approximation.

Typed SSA uses explicit basic-block parameters, exact scalar/product types,
trap edges, calls, and effects. It is the sole optimization authority for
reference lowering, JIT, the AOT test surface, and direct Wasm. The active cycle
may retain HIR-to-bytecode only while differential tests establish SSA; it then
lowers reference bytecode from verified SSA and deletes the sibling semantic
lowering before native lowering is authoritative. Runtime JIT and file emission
differ in code placement, relocation, and linking, not semantics or
optimization ownership.

## HIR Resolution Invariants

- Every name resolves once to a stable binding ID.
- Duplicate declarations are errors.
- Unknown symbols are errors and never become implicit globals.
- Locals use lexical shadowing consistently in analysis and lowering.
- Binding kind distinguishes immutable local, local var, parameter, function,
  const, static, capability, and future explicit reference cells.
- Calls resolve to canonical operations or declarations before codegen.
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

The required called-code experiment is complete. The accepted
[Linux x86-64 Native Backend](linux-x86-64-native-backend.md) selects a small
owned byte encoder over Cranelift 0.134.2 from generated-code speed plus visible
compilation, binary, RSS, dependency, safety, stack-map, W^X, and maintenance
costs. The decision adds no product dependency and implements no backend.

After typed SSA and differential correctness gates, implement only that selected
backend behind the shared native code-image/code-object boundary. Callable
baseline JIT remains the primary adaptive performance path. File emission
exists only to inspect code, test relocations and ABI behavior, use external
debuggers, and compare backend output with the VM.

Native target modes are explicit and every result records its target. Linux
x86-64 is the only acceptance platform for the active callable-baseline cycle;
Linux AArch64 begins only afterward. Backend-independent typed SSA and native
ABI boundaries must avoid accidental x86-specific semantic assumptions.

## Runtime Tiering

All normal execution starts in the VM. Bounded saturating function-entry and
loop-backedge counters eventually identify hot code without per-instruction
profiling. The accepted native sequence is:

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
