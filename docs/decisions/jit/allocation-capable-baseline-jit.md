# Allocation-Capable Baseline JIT

## Purpose

Define the completion boundary for extending the current callable scalar tier
to references, allocation, recursion, and versioned host/runtime calls.

## Status

The Unit/Bool/I64/F64 tier and deterministic host-independent aggregate
allocation/recursion slice are **Current** on Linux x86-64. The former
collecting implementation described by older evidence is superseded and
removed. The complete target remains an **Accepted Target** for additional host
capabilities, ownership adapters, and native/VM transitions. This status does
not claim OSR.

## Selected Delivery Slices

The implementation proceeds through separately honest boundaries:

1. **Current:** bounded active generated frames, typed homes, structural
   services, and invocation-region runtime-value dispatch;
2. **Current:** source-to-generated deterministic storage for String, products,
   List, Option, Result, and monomorphic enums, plus unique bytes and byte
   vectors, including construction, field/tag/read/write operations, direct and
   mutual recursion, and VM/evaluator/native equality;
3. **Accepted Target:** versioned Handle and host-capability calls, native/VM
   reference continuation, and same-commit allocation workload measurement.

Slices 1 and 2 are Current without claiming the complete target in this record.
Slice 3 and every uncovered item in **Required Surface** remain required before
“full allocation-capable baseline JIT” is a valid unqualified claim. `Owned`,
`Ref`, and `RefMut` lexical values are not silently relabeled runtime keys;
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

Bounded structural images own strings, enums, options, results, errors, and
copy products. Segmented invocation regions own selected lists; typed ordinary
regions own selected acyclic products. Unique storage owns bytes and byte
vectors. The evaluator, VM, baseline JIT, and proof JIT consume the same
verified storage plan and enforce logical construction, allocation, and
reserved-byte limits.

Native images retain bounded structural and runtime-value sites with canonical
operation-specific input/result/layout/allocation/store facts, source identity,
and verified frame homes. The executable trampoline alone reads those homes,
copies typed arguments into the safe service, writes the exact typed result,
and propagates structured status. Empty lists use the exact zero niche; every
nonzero invocation key is category/layout checked and cannot escape its
invocation.

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
- allocation, structural publication, and checked invocation-region writes;
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

Code objects, runtime sessions, structural/region stores, handles, frame records, and pinned
resources have one bounded owner. Generated code cannot execute after session
or executable ownership ends. Every terminal edge unwinds registered native
frames and releases resources before CLI status translation.

## Metrics

Current retained metrics include deterministic structural/list/region/unique
operations and reserved bytes, native frame depth, distinct attempted and
successful runtime-value calls, transitions, and compiler/native/code-cache
accounting. Normal execution remains silent; metrics are opt-in and never use
stdout. No collection metric is emitted.

## Acceptance

This target becomes Current only when:

1. typed homes validate against active native frames;
2. generated frames exercise structural and invocation-region values;
3. recursion with deterministic aggregates is exercised;
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
deoptimization, persistent profiles/caches, and Linux AArch64 code generation
are **Deferred**. Tracing collectors, conservative roots, silent forced
fallback, RWX, post-RX patching, raw source pointers, Brainfuck-specific
lowering, and substituting a second backend are **Rejected**.
