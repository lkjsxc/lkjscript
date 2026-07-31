# Allocation-Capable Baseline JIT

## Purpose

Define the completion boundary for extending the current callable scalar tier
to references, allocation, recursion, and versioned host/runtime calls.

## Status

The allocation-free Unit/Bool/I64/F64 tier and the host-independent source
allocation/recursion slice are **Current** on Linux x86-64. The complete target
in this record remains an **Accepted Target**: Handle/host-capability calls,
lexical `Owned`/`Ref`/`RefMut` adapters, and native/VM reference transitions are
not Current. This status does not claim an optimizing tier or OSR.

## Selected Delivery Slices

The implementation proceeds through separately honest boundaries:

1. **Current:** native canonical native contract typed references, exact non-empty stack maps,
   bounded active generated frames, and a safe collection-dispatch boundary;
2. **Current:** source-to-generated host-independent allocation for Str,
   products, List, Option, Result, and monomorphic the canonical source contract
   enums, plus noncollecting unique bytes and byte vectors,
   including construction, field/tag/read/write operations, direct and mutual
   recursion, forced collection, and VM/evaluator/native equality;
3. **Accepted Target:** versioned Handle and host-capability calls, native/VM
   reference continuation, and same-commit allocation workload measurement.

Slices 1 and 2 are Current without claiming the complete target in this record.
Slice 3 and every uncovered item in **Required Surface** remain required before
“full allocation-capable baseline JIT” is a valid unqualified claim. `Owned`,
`Ref`, and `RefMut` lexical values are not silently relabeled GC references;
the ownership safe island retains a deterministic generated-tier rejection
until a separate exact adapter is proved.

## Pipeline

The only semantic path remains:

```text
resolved typed HIR
  -> ownership/trait-resolved verified typed SSA
  -> verified baseline normalization
  -> owned Linux x86-64 machine plan
  -> validated bounded code object
  -> W^X install
  -> actual generated call
```

The JIT does not interpret source names and does not create a second memory IR.
Linux x86-64 is the only acceptance platform. IR, ABI, layouts, and metadata
use target-independent identities where a later Linux AArch64 backend will need
them.

## Current Host-Independent Slice

`GcHeap` is the pure stable-index mark/sweep heap in `lkjscript-core`; VM and
forced JIT sessions use that implementation with exact allocation counts,
deterministic estimated object-byte accounting, collection counts, estimated
peak-live bytes, and stress-collection APIs. The independent evaluator mirrors
these allocation and estimated-live-byte limits for its heap-producing
operations. Bytes conversion and slicing use separately bounded unique storage;
traced error and Result envelopes retain their ordinary charges.
Because a language `Value` has no generation field, swept object indices are
never reused within a session, and all publication paths reject before the
stable `u32` handle space is exhausted. Mutation is transactional and
layout-preserving: growth is estimated and
checked before accounting commits, and closure, layout, or limit failure
restores the prior object. canonical native contract images
retain bounded `HeapDispatchV1` sites with canonical operation-specific
input/result/layout/allocation/store facts, including nominal product field
facts and collision-free interned full structural List/Option/Result identities
plus their payload identities, source, safepoint, and verified frame homes. The
sys trampoline alone reads raw homes, copies typed
arguments and roots into a safe service, writes exact roots/results back,
re-materializes arguments after any moving root writeback, and propagates
structured status. Empty List and None use only the exact zero niche; other references
reject zero and every nonzero handle is category/layout checked.

Forced lowering covers unique bytes and byte vectors plus Str, Product, List,
Option, Result, and monomorphic host-independent enums, their listed constructors/accessors/
mutations/conversions/equality families, and recursive SCCs. Runtime ABI calls are generated execution, not fallback.
Automatic mode deliberately keeps reference-signature entries in the VM
because native/VM reference transfer is not Current. Such a helper may still
be installed inside a supported generated direct-call group, but its per-
function auto-entry eligibility remains false, so a later direct VM call cannot
be mislabeled native. Symbol, Handle/host IO, indirect
calls, and lexical ownership references reject deterministically.

## Required Surface

The tier supports directly or through exact versioned runtime calls:

- Unit, Bool, I64, and F64;
- products, Option, Result, Str, bytes, byte-vector, List, and typed resources;
- construction, field/tag access, immutable replacement, current byte
  operations, and exact equality families;
- allocation, initialized object publication, and classified heap stores;
- direct calls, direct and mutual recursion, and native-to-native calls;
- VM-to-native entry and explicitly allowed native-to-VM continuation;
- structured return, trap, exit, deadline, resource limit, and host failure;
- bulk bytes, durable files, SHA-256, and SQLite through runtime calls.

`copy-bytes-slice` is a checked owned copy. SQLite remains a runtime
capability; SQLite code is not generated machine code.

## Recursion

The blanket recursion rejection is removed. Compiled strongly connected groups
use bounded native frame accounting and exact roots across calls. Direct and
mutual recursion preserve poll/deadline behavior and structured status
propagation. Proof-based self-tail-call elimination is an optimizing pass, not
a condition for baseline support or a source-specific shortcut.

## Forced And Automatic Behavior

`--engine baseline-jit` compiles every required reachable supported function
before source effects, installs baseline code objects, invokes them, and never
falls back to the VM. A versioned runtime call is not fallback. Any reached
unsupported semantic or resource failure is a visible engine error or
structured execution outcome as specified.

`auto` may keep unsupported functions in the VM, compiles synchronously for
later entries, and suppresses bounded same-epoch retries. It does not claim
transfer into an already-running invocation and is not OSR.

## Resource Ownership

Code objects, runtime sessions, heaps, handles, frame records, and pinned
resources have one bounded owner. Generated code cannot execute after session
or executable ownership ends. Every terminal edge unwinds registered native
frames and releases resources before CLI status translation.

## Metrics

Current retained metrics include allocation counts, deterministic estimated
object bytes, collections, estimated peak-live object bytes, root count,
barrier count, native frame depth, distinct attempted and successful heap
runtime calls, and transition counts in addition to compiler/native/code-cache
accounting. Normal execution remains silent; metrics are opt-in and never use
stdout. Collection pause distribution remains an acceptance target and is not
currently measured or emitted.

## Acceptance

This target becomes Current only when:

1. non-empty stack maps validate against active native frames;
2. collection is forced while generated frames hold live references;
3. recursion with live references is exercised;
4. products, Option, Result, strings, buffers, and lists have exact generated or
   runtime-call paths;
5. host capability smokes remain exact;
6. forced mode records native entries and zero fallback;
7. VM/evaluator/native values and structured outcomes agree;
8. W^X, limits, malformed metadata, and repeated ownership tests pass;
9. at least one declared allocation workload is measured against same-commit VM.

Machine bytes, non-empty-looking metadata, or a helper called only from Rust do
not satisfy this decision.

## Deferred And Rejected

OSR, background compilation, compiler threads, speculative guards,
deoptimization, persistent profiles/caches, a concurrent collector, and Linux
AArch64 code generation are **Deferred**. Conservative roots, silent forced
fallback, RWX, post-RX patching, raw source pointers, Brainfuck-specific
lowering, and substituting a second backend are **Rejected**.
