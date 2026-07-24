# Allocation-Capable Baseline JIT

## Purpose

Define the completion boundary for extending the current callable scalar tier
to references, allocation, recursion, and versioned host/runtime calls.

## Status

The allocation-free Unit/Bool/I64/F64 baseline tier is **Current**. This record
is an **Accepted Target** until forced generated execution allocates, survives
collection with active exact native frames, and passes the gates below. It does
not supersede the current scalar contract prematurely.

## Selected Delivery Slices

The implementation proceeds through separately honest boundaries:

1. native ABI 2 typed references, exact non-empty stack maps, bounded active
   generated frames, and a safe collection-dispatch boundary, tested first with
   closed machine plans;
2. source-to-generated host-independent allocation for Str, Buf, products,
   List, Option, and Result, including field/tag/read/write operations, direct
   and mutual recursion, forced collection, and VM/evaluator/native equality;
3. versioned Handle and host-capability calls, native/VM continuation, complete
   metrics, and same-commit allocation workload measurement.

Slices 1 and 2 may become Current without claiming the complete target in this
record. Slice 3 and every item in **Required Surface** remain required before
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

## Required Surface

The tier supports directly or through exact versioned runtime calls:

- Unit, Bool, I64, and F64;
- products, Option, Result, Str, Buf, List, and Handle;
- construction, field/tag access, immutable replacement, current buffer
  operations, and exact equality families;
- allocation, initialized object publication, and classified heap stores;
- direct calls, direct and mutual recursion, and native-to-native calls;
- VM-to-native entry and explicitly allowed native-to-VM continuation;
- structured return, trap, exit, deadline, resource limit, and host failure;
- bulk bytes, durable files, SHA-256, and SQLite through runtime calls.

`buf-slice` remains copying. SQLite remains a runtime capability; SQLite code is
not generated machine code.

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

Retained metrics add allocation count/bytes, collections, pause distribution,
peak live heap, root count, barrier count, native frame depth, runtime calls,
and transition counts to the existing compiler/native/code-cache accounting.
Normal execution remains silent; metrics are opt-in and never use stdout.

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
