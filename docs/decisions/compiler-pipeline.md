# Typed Compiler Pipeline And Early AOT

## Purpose

Define one semantic pipeline shared by the reference VM, native AOT, future
Wasm, and future JIT so performance backends cannot reinterpret the language.

## Status

**Current** for parsed AST -> resolved typed HIR -> reference bytecode.
Typed SSA, native AOT, direct Wasm, and JIT remain **Accepted Targets**; native
compilation is not implemented.

## Pipeline

The accepted pipeline is:

```text
canonical source
  -> parsed AST
  -> resolved typed HIR
  -> typed SSA IR
  -> verified optimization pipeline
  +-> reference bytecode
  +-> native AOT object/executable
  +-> direct Wasm
  +-> in-memory native code when JIT is justified
```

Typed HIR now owns resolved binding IDs, declaration kinds, exact static type
facts, source origins, canonical operation identities and per-call signatures,
and conservative effects. Code generation consumes HIR rather than re-parsing
definitions, parameters, operators, and names independently.

`Unit`, typed empty lists, and Option have exact distinct HIR/runtime semantics,
and every `if` has exactly three operands with matching branch types. Nil and
the legacy runtime-union escape hatch are absent. User calls conservatively carry every effect until fixed-point
function summaries replace the safe over-approximation.

Typed SSA uses explicit basic-block parameters, exact scalar/product types,
trap edges, and effects. It is the optimization authority for both AOT and JIT.
AOT and JIT differ in code placement and linking, not language semantics or
optimizer ownership.

## HIR Resolution Invariants

- Every name resolves once to a stable binding ID.
- Duplicate declarations are errors.
- Unknown symbols are errors and never become implicit globals.
- Locals use lexical shadowing consistently in analysis and lowering.
- Binding kind distinguishes immutable local, local var, parameter, function,
  const, static, capability, and future explicit reference cells.
- Calls resolve to canonical operations or declarations before codegen.
- Types and arity are attached to HIR nodes and are not recomputed by backends.
- Imported source origin remains available for machine-readable diagnostics.

The first HIR slice preserves accepted workload behavior while removing the
duplicate typechecker/code-generator interpretation. It also rejects unknown
or duplicate bindings, operation-name collisions, unresolved generic variables,
and invalid global assignments. `set` now yields Unit, and top-level
control-flow fragments use correct absolute jump targets.
Semantic-core changes now land as complete HIR-to-VM vertical slices.

## Runtime Representations

The reference VM may retain compact tagged values for cold execution and
conformance. Typed HIR/SSA/native hot paths do not carry a universal boxed
Value when a static representation exists:

- I64 and F64 use native scalar registers;
- Bool and Unit use canonical scalar/zero-sized representations;
- Str, Bytes, Slice, and views use typed pointer/length layouts;
- products are flattened or passed according to the target ABI;
- Option uses a proven type-specific niche or an explicit tag;
- generic code is monomorphized where bounded code growth justifies it.

Dynamic dispatch and runtime type tests remain only when explicitly required by
a dynamic interface contract.

## Native Priority

After typed SSA and differential correctness gates, implement a minimal Linux
x86-64 native AOT backend early enough to measure the language's performance
ceiling. The reference VM remains the correctness oracle and cold path.

The project prefers owned implementation and no permanent opaque dependency in
runtime hot paths. A small owned baseline assembler/backend is therefore the
first candidate. Mature build-time backends may later be evaluated as reference
or optimization candidates under separate dependency, correctness, compile-time,
binary-size, and runtime measurements. Cryptography is never reimplemented as
a compiler-dependency experiment.

Native target modes are explicit: portable, x86-64-v2, x86-64-v3, and native.
Benchmark results always record the selected CPU target. Linux AArch64 ABI
validation begins before x86-specific assumptions become structural.

## Wasm And JIT

The VM compiled to Wasm is a conformance/reference path. Direct typed-SSA-to-Wasm
is the browser performance path and follows native AOT, not an indefinitely
deferred replacement.

Baseline JIT follows process-safe outcomes, executable code objects, typed SSA,
and AOT differential tests. Optimizing JIT and deoptimization follow only when
warmup and runtime specialization beat PGO AOT on declared workloads. Inline
caches are not a default architecture for statically resolved calls.

## Optimization Trust

Every optimization preserves typed-IR semantics and is tested against the
reference evaluator/VM. Assumptions require proof or explicit checks. Undefined
behavior is not used to make invalid programs appear fast. Differential,
property, boundary, and corpus tests precede benchmark adoption.

## Adoption Gates

A backend or optimization candidate records:

1. semantic and ABI version;
2. differential correctness against reference execution;
3. malformed-input and trap behavior;
4. compile time, startup, runtime, code size, RSS, and relevant latency;
5. target CPU and enabled assumptions;
6. isolated and combined variants;
7. adoption/rejection thresholds and retained artifacts.

## Rejected

- Lowering untyped AST independently in each backend.
- Carrying tagged universal Value through typed native hot paths.
- Treating the observation-only JIT hook as a native-code boundary.
- Delaying all native execution until after product frameworks.
- Making AOT and JIT use different semantic IRs.
- Trusting AI-authored optimization hints.
