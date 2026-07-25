# Native References, Frames, And Exact GC Stack Maps

## Purpose

Define the safety boundary that must be Current before generated code may
allocate or carry heap references across a collecting safepoint.

## Status

The closed machine-plan boundary and host-independent source allocation slice
are **Current** on Linux x86-64. Native ABI 2 carries typed stable reference
words, derives exact typed maps, registers generated frames, and reaches
`GcHeap` through safe copied-root/typed-operation callbacks. Reference-capable
SSA/JIT lowering, initialization/scalar-store classification, and direct/mutual
recursive source groups are Current for Str, legacy Buf, Product, List, Option,
and Result. Handle/host-capability allocation, lexical ownership references,
and native/VM reference transitions remain **Accepted Targets**.

## Selected Implementation Contract

**Current for closed machine plans and actual sys invocation.** Semantic ABI
version 1 remains stable. Reference-capable and scalar code objects both use
native ABI version 2. Runtime ABI version 1 has enum-identified V1 frame and
collection-dispatch calls; ABI-1 objects are rejected rather than reinterpreted
as ABI 2.

`lkjscript-native` owns pure typed handle, frame-home, liveness, exact-map, and
bounded generic heap-site metadata. `lkjscript-sys` alone retains installed
metadata beside the active invocation, holds raw generated-frame addresses,
validates every active map and heap argument/result home, and copies typed
values/handles into the safe `NativeRuntimeServices` callback.
Code installation remains owned RW-then-RX and no heap word is exposed as a
native object pointer. The former VM arena now lives in `lkjscript-core` as session-owned `GcHeap` and
is used by both execution implementations. Automatic reference transitions are
still absent, so auto conservatively retains reference-typed functions in VM.

Native plans identify each GC reference by an exact reference/layout identity.
Nested List/Option/Result identities are dense deterministic interned identities
of the complete structural `SsaType`, not truncated structural hashes; the
identity and its exact component identities remain in runtime-site metadata and
heap tags. The machine-plan verifier rejects one retained identity describing
two layouts. Frame descriptors enumerate every reference-capable value/local
home. The machine-plan verifier runs bounded backward CFG reference liveness and retains a
per-call sorted typed root-requirement certificate. It charges analysis work
and every retained root record before allocation against the backend work and
metadata-derived root budgets. The encoder must consume that certificate, and
the private image requirement is retained independently of the public stack
map so structural integrity validation rejects omitted or stale roots as well
as out-of-frame, mistyped, duplicate, or unsorted roots. Registers are not roots
in the first slice: references are homed before collection and reloaded
afterward.

Generated prologues establish only the minimal ABI frame, save incoming machine
arguments in invocation-owned scratch, and call the encoder-owned
`ReserveFrameV1` before subtracting or initializing generated-frame storage.
The sys reservation validates the exact function descriptor and requested frame
bytes, configured aggregate and per-frame native-stack budgets (4 MiB and 1 MiB
by default), active-frame capacity, and the current Linux pthread stack bounds
with a 16 KiB guard margin. Those immutable bounds are queried once when the
non-migrating invocation state is created and reused for each nested frame
check. Reservation failure reports exact `ActiveFrames` or
`NativeStackBytes` resource outcomes through an unregistered epilogue. Successful reservations are tracked
and released byte-for-byte by matching registration/unregistration. A
collecting call first publishes its dense safepoint identity. The sys trampoline
validates the active chain and stack map, materializes only exact live roots for
the safe runtime service, and writes back updated handles before generated
execution resumes. Root vectors grow only under their aggregate cap; invocation
setup does not multiply one shallow map by the maximum possible frame depth.
Runtime-service resource rejection is reported as `RuntimeService`, distinct
from `MaterializedRoots`. `RuntimeCallSlot::plan_signature` exposes only
plan-callable typed signatures; encoder-owned Reserve/Register/Publish/
Unregister/HeapDispatch slots instead expose their exact
context/ordinal/byte/pointer/safepoint/heap-site machine arguments through
`internal_abi_signature`. In particular, the second `HeapDispatchV1` argument is
a heap-site ID, not a safepoint ID. ABI-2 frame registration records the exact
source-function entry and consumes the mandatory entry poll before generated
body effects, so source lowering does not emit duplicate `EnterFunctionV1` or
entry `PollV1` transitions. Backedge and explicit polls remain ordinary runtime
calls. A verified transitive may-collect summary publishes
a caller safepoint only for a direct callee whose closure can collect; scalar
non-collecting calls retain exact empty maps without paying a publication call.
Every registered return, trap, exit, deadline, resource, host-failure, and propagated-callee edge
unregisters once. Reports
retain peak depth and native-stack bytes, collection calls, maximum roots, and
the exact root count for every collection; completed outcomes report zero
active depth and zero reserved native-stack bytes.

## Reference Representation

A worker-local GC reference is a typed stable runtime handle, not a raw object
address and not a lexical borrow. Generated code may hold its machine word only
where SSA and code-object metadata identify it as a GC reference. The handle
indexes runtime-owned storage and therefore remains valid if a future moving
collector relocates the object behind that storage.

GC references may be copied within one worker according to their type contract,
but the current `NativeReference` adapter token is explicitly non-`Send` and
non-`Sync` even though it remains `Copy`. It is a runtime-adapter token, not a
source-language reference or an independently owned heap value; it cannot be
converted to a native pointer in safe source or outlive its owning runtime
session. Lexical shared/exclusive borrows, uniquely owned values, immutable
cross-worker bytes, and explicitly pinned values are distinct categories.

## Native Frame Chain

Every active generated function has an explicit runtime-visible frame record.
The record identifies:

- code object and function descriptor;
- frame base and bounded frame size;
- current safepoint/call-site identity;
- spill, argument, result, and callee-saved locations;
- caller record;
- source/outcome/frame-state metadata.

Generated prologues reserve against descriptor, configured byte limits, active
frame count, and the invocation-cached actual guarded pthread stack bounds before they subtract or
initialize the requested storage. They register the frame only after its
descriptor and storage are valid. Reservation and registration are matched, and
every normal, trap, exit, deadline, resource, and host-failure edge unregisters
and releases exactly once. Registration is non-reentrant and bounded. Platform
unwinding heuristics and conservative stack walking are not semantic root
mechanisms.

Any raw stack address needed to implement registration and root access remains
inside `lkjscript-sys`. Safe callers receive only owned descriptors, bounded
invocation objects, and structured outcomes.

## Stack Maps

Each direct or native-runtime call records an exact sorted, deduplicated set of
typed root locations. Calls that may collect publish the map's dense identity
before transfer; PollV1 and EnterFunctionV1 remain non-collecting.
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
Metadata validation checks frame bounds, alignment, location kind, verifier-
certificate equality, reference type, safepoint ownership, and code offset.

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
it returns a typed GC handle or structured resource/host failure. Every
publication path, including the VM compatibility path, checks allocation,
heap-byte, and stable `u32` handle-index exhaustion before publication; index
exhaustion is an allocation resource outcome and cannot create a duplicate
handle. Partially initialized objects are runtime-private and never visible to
source or the collector as complete objects.

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

The Current closed-plan acceptance forces `CollectReferenceV1` with an exact
Buf-reference argument/local map, proves a dead Buf home is absent, observes a
caller/callee active chain, writes back copied handles, covers callback failure
and every structured epilogue, enforces frame bounds, and repeats W^X
install/invoke/drop. It does not allocate a language object.

Source-level host-independent native allocation is Current after tests force collection with roots
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
