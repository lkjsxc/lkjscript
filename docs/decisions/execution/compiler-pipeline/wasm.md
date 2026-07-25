# Typed Compiler Pipeline And Runtime JIT: Wasm

[Authority](../compiler-pipeline.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

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
