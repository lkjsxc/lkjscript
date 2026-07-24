# Callable Baseline JIT

## Purpose

Describe the implemented Linux x86-64 baseline tier without implying full-
language native coverage or future OSR behavior.

## Status

**Current on Linux x86-64** for the exact allocation-free scalar subset below.
Ordinary `run` uses automatic tiering; the reference VM remains the cold tier,
full-language oracle, and deterministic explicit engine.

## Implemented Pipeline

Canonical source is resolved and normalized once:

```text
canonical source -> typed HIR -> VerifiedProgram SSA
  -> deterministic scalar adapter -> verified lkjscript-native machine plan
  -> encoded image and symbolic relocations -> bounded lkjscript-sys RW/RX install
  -> actual System V AMD64 entry call
```

The adapter consumes only `VerifiedProgram`. It does not read source syntax,
HIR, or bytecode. One installed group contains the requested function and its
acyclic direct-call closure. Compatible generated callers use direct relocated
native calls and unboxed I64/F64/Bool/Unit values. VM/native adapters alone box
or unbox scalar values, including wide VM I64 and F64 heap values.

Current generated coverage is Unit, Bool, I64, F64, block parameters, local
mutation represented as machine-plan locals, multi-block branches, loops,
checked I64 arithmetic/division, bit operations, I64-to-F64 conversion, IEEE
F64 arithmetic and ordered comparisons, exact F64-bit equality, `not`, direct
user calls, return, trap, exit, and structured outcome propagation. Recursive
or indirect user calls are explicit unsupported boundaries.

Native entry, call, and loop transitions use enum-identified versioned runtime
calls. `EnterFunctionV1` records exact per-source-function native entries.
`PollV1` consumes bounded native poll fuel, checks a monotonic deadline, counts
polls, and propagates deadline, resource-limit, or host-clock status through the
shared invocation state. Native ABI 2 prologues also register initialized
frames, collecting calls publish dense safepoints, and every structured edge
unregisters. Generated code never exits the host process.

## Engine Behavior

The CLI accepts:

```text
lkjscript run --engine vm file.lkjscript
lkjscript run --engine baseline-jit file.lkjscript
lkjscript run --engine auto file.lkjscript
```

Ordinary `run` selects `auto` with a 64-entry threshold. Explicit `vm` never
compiles or invokes machine code. `baseline-jit` verifies, compiles, installs,
and invokes main plus its complete
statically reachable direct-call group before main effects. Unsupported
semantics, recursion, compilation failure, ABI failure, or native resource
exhaustion is an `EngineError`; forced mode never silently falls back to the
VM.

`auto` begins in the VM. Saturating function-entry counts synchronously compile
a hot supported acyclic scalar function at its entry safepoint, and that call
still runs in the VM. Later calls use installed native code. Unsupported or
resource-limited compilation remains VM-correct and is retry-suppressed in the
same epoch. A changed epoch permits only the configured bounded retry. Auto can
be disabled or given a deterministic threshold. This is whole-function entry
tiering, not OSR; one long invocation does not accelerate until a later OSR
cycle.

Each function has `VmOnly`, `Observed`, `BaselineCompiling`,
`BaselineNative`, or `Disabled` state plus saturating calls, bounded attempts,
last structured failure, code-object identity, epoch, and native entries.
Compilation is synchronous and non-reentrant. The retained scalar result at
implementation commit `025cbb2feadbb18fbae51e68e38b9c849798d068`
measured auto process wall 1.653x faster than VM at threshold 64. Thresholds 1,
64, and 1,024 were statistically close on the 100,000-call workload; 64 retains
63 cold calls in the VM without delaying native entry to the 1,024-call point.
This workload-specific evidence selects the default but is not a universal
threshold optimum.

## Code Objects And Safety

A bounded non-Send execution session owns installed mappings and accounting.
An installed code object retains function-group identity, baseline tier,
semantic/native/runtime ABI versions, entries, code and page-accounted
allocation sizes, relocations, runtime-call identities, typed frame homes,
safepoints with exact derived stack maps, source/trap/outcome maps,
compile/install statistics, invalidation state, metadata bytes, and native entry
counts. Limits cover per-object and aggregate code, metadata, work, object
count, diagnostic bytes, and compilation wall time. There is no persistence,
eviction, background compilation, concurrent mutation, or post-RX patching.

Safe sys APIs accept only opaque images emitted from verified closed plans.
Mappings transition RW to RX, are never RWX, expose no raw entry address, and
remain owned for every invocation. Scalar eligibility rejects every reference
and allocation path, so its ABI-2 maps remain exactly empty. Independent closed
machine-plan tests carry typed Buf handle words in GPRs, derive non-empty exact
maps, register caller/callee frames, and invoke a safe copied-root collection
service; no raw stack or object pointer crosses the sys boundary.

## Unsupported Native Semantics

Forced baseline compilation rejects Str, Symbol, List, Buf, Handle, products,
Option, Result, function references, references of any kind, allocation,
memory/reference operations, host IO, indirect calls, recursion, polymorphic or
unsupported signatures, and any operation without an exact scalar lowering.
Auto leaves these semantics in the VM and suppresses repeated same-epoch
failure. This is intentionally not a full-language JIT claim.

Machine diagnostics are opt-in and go only to stderr. They include normalized
SSA, generated bytes, relocations/code-object metadata, and native counts.
`LKJSCRIPT_JIT_DUMP_DIR` writes generated binary images and prints an external
`objdump` command where that tool is available.

Separate low-overhead `LKJSCRIPT_METRICS` records one machine-readable JSON line
on stderr; `LKJSCRIPT_METRICS_FILE` writes that line to an explicit file instead.
The versioned record includes exact scalar outcome bits, every compiler phase,
VM/engine/native durations, time to first native entry, first-call duration,
compile/install durations, tier states/counts/failures/fallbacks, code and
metadata/cache accounting, direct calls, and PollV1 calls. Collection is
conditional; ordinary native-call hot paths do not read the clock. Normal
program stdout is never used for JIT diagnostics or metrics.

Loop OSR, background compilation, optimizing tiers, speculation, guards,
deoptimization, source-level native allocation, shared VM/native heap objects,
and persistent profiles or code caches remain future or rejected work as
classified by the active decisions.

## Native-Reference Boundary

**Current below source lowering; Accepted Target at source level.** Native ABI 2
provides typed stable handle words, exact non-empty maps, bounded registered
generated frames, and enum-identified runtime-ABI-1 frame and collection calls.
The Current collecting slot is `CollectReferenceV1` for exact Buf-reference
identity. Layout identities are extensible to Str, List, Option, Result, and
nominal products, but those source values are still rejected by the scalar JIT.

The next source slice supplies a shared stable-handle heap and host-independent
Str, Buf, product, List, Option, and Result allocation plus direct/mutual
recursion. Host capabilities, lexical ownership references, OSR, and
optimization are not smuggled into this machine-plan foundation.
