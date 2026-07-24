# Native References, Frames, And Exact GC Stack Maps

## Purpose

Define the safety boundary that must be Current before generated code may
allocate or carry heap references across a collecting safepoint.

## Status

Precise non-moving mark-sweep in the VM and allocation-free native frames with
exact empty reference maps are **Current**. Native heap references, native
allocation, active-frame root enumeration, barriers, and collection during a
generated call are an **Accepted Target**. Emitted non-empty metadata without a
collection test is experimental evidence, not completion.

## Selected Implementation Contract

**Accepted Next Slice; not Current until its retained tests pass.** Semantic ABI
version 1 remains stable. The first reference-capable code objects use native
ABI version 2. Runtime ABI version 1 is extended only with enum-identified V1
frame, collection-dispatch, and barrier calls; scalar ABI-1 objects are rejected
rather than reinterpreted as ABI 2.

The VM arena moves into the pure shared runtime layer so VM and generated code
use one session-owned stable-handle heap. `lkjscript-jit` supplies safe runtime
services; `lkjscript-sys` alone may hold raw generated-frame addresses while an
invocation is active. Code installation remains owned RW-then-RX and no heap
word is an exposed native pointer.

Native plans identify each GC reference by an exact reference/layout identity.
Frame descriptors enumerate every reference-capable value/local home. A bounded
backward CFG analysis derives the sorted typed live-root subset at each
collecting call, and independent machine-plan/image validation rejects omitted,
out-of-frame, mistyped, duplicate, unsorted, or stale roots. Registers are not
roots in the first slice: references are homed before collection and reloaded
afterward.

Generated prologues reserve bounded frame depth/bytes, initialize storage, and
register one frame. A collecting call first publishes its dense safepoint
identity. The sys trampoline validates the active chain and stack map,
materializes only exact live roots for the safe runtime collector, and writes
back updated handles before generated execution resumes. Every registered
return, trap, exit, deadline, resource, host-failure, and propagated-callee edge
unregisters once; reservation failure uses an unregistered epilogue.

## Reference Representation

A worker-local GC reference is a typed stable runtime handle, not a raw object
address and not a lexical borrow. Generated code may hold its machine word only
where SSA and code-object metadata identify it as a GC reference. The handle
indexes runtime-owned storage and therefore remains valid if a future moving
collector relocates the object behind that storage.

GC references may be copied within one worker according to their type contract,
but are not automatically `Send`, cannot be converted to an integer or native
pointer in safe source, and cannot outlive their owning runtime session. Lexical
shared/exclusive borrows, uniquely owned values, immutable cross-worker bytes,
and explicitly pinned values are distinct categories.

## Native Frame Chain

Every active generated function has an explicit runtime-visible frame record.
The record identifies:

- code object and function descriptor;
- frame base and bounded frame size;
- current safepoint/call-site identity;
- spill, argument, result, and callee-saved locations;
- caller record;
- source/outcome/frame-state metadata.

Generated prologues register the frame only after its descriptor and storage are
valid. Every normal, trap, exit, deadline, resource, and host-failure edge
unregisters exactly once. Registration is non-reentrant and bounded. Platform
unwinding heuristics and conservative stack walking are not semantic root
mechanisms.

Any raw stack address needed to implement registration and root access remains
inside `lkjscript-sys`. Safe callers receive only owned descriptors, bounded
invocation objects, and structured outcomes.

## Stack Maps

Each collecting safepoint records an exact sorted set of typed root locations.
A location is one of:

```text
stack slot at a checked frame-base displacement
native argument home
returned-value home
runtime-context root slot
```

Registers containing references are either described explicitly or spilled to
an exact home before a collecting call. A root is included only while live, but
must not be omitted because the current collector is non-moving. Duplicates are
permitted only if verifier-normalized as aliases to the same live handle.
Metadata validation checks frame bounds, alignment, location kind, liveness,
reference type, safepoint ownership, and code offset.

A future moving collector may update every listed root. Generated code reloads
live references after a collecting call rather than relying on stale registers.

## Safepoints

Safepoints are required at allocation slow paths, allocation-capable runtime
calls, direct calls whose summaries may allocate, recursive calls, loop/deadline
polls, explicit GC polls, and VM/native transitions. A collecting safepoint has
an exact frame state and stack map before native installation.

## Allocation ABI

Runtime ABI version 1 is extended only through new enum-identified versioned
calls. The accepted shape is:

```text
worker-local allocation context
  -> optional checked bump-pointer fast path
  -> bounded slow allocation call
  -> collection request
```

The first correct implementation may use the slow call for every allocation.
It receives a verified layout identity and fully initialized reference inputs;
it returns a typed GC handle or structured resource/host failure. Partially
initialized objects are runtime-private and never visible to source or the
collector as complete objects.

## Write Barrier ABI

Every heap store is classified in SSA as initialization, scalar store,
reference store, or reference-clearing store. A versioned write-barrier call may
be a verified no-op for the current non-generational collector, but code and
metadata retain owner/value/type facts. This permits a later nursery/old
collector without reinterpreting source operations.

## Pinning

Pinning is explicit, session-owned, bounded, and unavailable to ordinary safe
source until a separate native-capability contract requires it. Objects are not
pinned merely to simplify generated code.

## Collection Acceptance

Native allocation becomes Current only when tests force collection with roots
in arguments and spill slots, across native/native and native/runtime calls,
through recursion, immediately before return, and around a native/VM
transition. Tests include live/dead product, Option, Result, string, buffer, and
list graphs, tiny heap limits, barriers, traps, exits, deadlines, and repeated
install/invoke/drop. Missing or stale roots must fail validation or tests rather
than become undefined behavior.

## Long-Term GC Sequence

```text
exact native roots
  -> allocation and barrier ABI
  -> escape and stack allocation
  -> worker-local copying nursery
  -> promoted old generation
  -> incremental/concurrent marking
  -> measured compaction
  -> immutable shared large objects
```

Only the first two steps are in scope for the current implementation cycle.
Conservative scanning, default pinning, and a shared mutable process heap are
**Rejected**.
