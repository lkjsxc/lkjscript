# Native References, Frames, And Exact GC Stack Maps: Collection Acceptance

[Authority](../native-references-and-gc-stack-maps.md)

## Status

**Historical, superseded acceptance record.** The remaining sections retain
pre-cutover collection evidence and do not describe Current runtime behavior.
The Current runtime has no collection path or GC sequence.

## Collection Acceptance

The Current closed-plan acceptance forces `CollectReferenceV1` with an exact
traced-reference argument/local map, proves a dead traced home is absent, observes a
caller/callee active chain, writes back copied handles, covers callback failure
and every structured epilogue, enforces frame bounds, and repeats W^X
install/invoke/drop. It does not allocate a language object.

Source-level host-independent native allocation is Current after tests force collection with roots
in arguments and spill slots, across native/native and native/runtime calls,
through recursion, immediately before return, and around a native/VM
transition. Tests include live/dead product, Option, Result, string, and list
graphs plus independently noncollecting bytes and byte vectors, tiny heap limits,
barriers, traps, exits, deadlines, and repeated
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
