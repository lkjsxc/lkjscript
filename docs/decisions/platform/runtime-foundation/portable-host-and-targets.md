# Portable Host and Targets

## Status

**Accepted Contract with Experimental Implementation.** Linux x86-64 remains
the only Current acceptance host. Safe-Rust target, clock, logging,
cancellation, and durable-storage contracts are implemented; VM host cutover,
optimized providers, and other native targets remain absent.

## Current Inherited State

Current source resolves once through typed HIR and verified SSA before VM or
native execution. Current Linux host effects remain explicit capabilities.
Portability is a design constraint, not a support claim. There is no Current
Wasm, WASI, Component Model, or Cranelift execution path.

## Accepted Boundary

A target describes code and ABI constraints. A host supplies measured execution
policy, capabilities, clocks, interruption, and resource ceilings. Neither may
reinterpret source syntax or bypass verified SSA. Target identity, runtime image
identity, capability grants, and resource profile are content-addressed inputs.

The [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
is adopted **only as an external component ABI reference**. It is rejected as
lkjscript's internal type system, object model, package identity, or semantic
IR. Core language values and effects remain authoritative.

## Current Unsafe Boundary Slice

`LKJ-UNSAFE-BOUNDARY` now enforces the accepted bounded machine-readable
registry. Every authored Rust file containing a lexical `unsafe` code token
appears exactly once under one stable boundary identity; every registered file
exists and contains such a token. The scanner ignores comments and
string/character literals. Registry locations extend beyond `lkjscript-sys` only after architecture and
caller-contract review. The Current host entry reads Linux Unix-socket peer
credentials behind a safe typed principal API; inherited sys mechanisms remain
registered separately.

## Implemented Portable Slice

`lkjscript-host` composes implemented stdio, clock, logging, cancellation,
directory, database-interface, and durable-storage families. Validated VM direct
stdio and clock operations consume app-private provider references; file,
terminal, network, SQLite, and stream-resource cutover remains incomplete. The
standard durable provider preserves native paths in the
control boundary, uses checked relative object names, syncs data, and syncs a
parent directory after atomic replacement where the host supports that call.
Portable application paths are normalized relative segments resolved only by a
directory provider. Buffered stdio and fake durable storage provide deterministic
observations and injected short-write, disk-full, sync, corruption, and crash
behavior. Explicit target
facts do not alter language semantics. Host/database build and a transactional
fake-storage probe execute for `wasm32-wasip1`; VM/runtime-system do not build
there.

## Deferred Wasmtime Cell Reference

Wasmtime remains a strategic but Deferred reference for a later optional
isolated cell. This cycle does not add or expand Wasm execution. Its
official material documents:

- [fast instance reuse](https://docs.wasmtime.dev/examples-fast-instantiation.html);
- [store resource limiting](https://docs.wasmtime.dev/api/wasmtime/struct.Store.html);
  and
- [epoch interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html).

Adoption means measuring equivalent bounded construction, memory/table limits,
and deterministic cancellation. It does not adopt Wasmtime as a universal
runtime, permit ambient WASI, or make a cell the internal ABI. The probe must
pin dependency and interface identities, grant capabilities explicitly, and
fail closed when limits or interruption are unavailable.

## Deferred Backend Candidate

[Cranelift](https://github.com/bytecodealliance/wasmtime/tree/main/cranelift) is
a backend candidate, **Deferred** pending accepted dependency record,
license/advisory review, build/package cost, measured compile/runtime behavior,
and proof that it consumes the shared verified SSA contract without semantic
divergence. No workspace dependency is authorized by this decision.

## Rejected This Cycle

- claiming portability acceptance from emission, disassembly, or scaffolding;
- exposing WASI as ambient host authority;
- making the Component Model an internal runtime representation;
- replacing callable native-tier evidence with Wasm observation; and
- treating unverified WASI 0.3 release metadata as Current evidence.
