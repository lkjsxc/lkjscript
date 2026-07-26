# Callable Baseline JIT: Implemented Pipeline

[Authority](../baseline-jit.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Implemented Pipeline

Canonical source is resolved and normalized once:

```text
canonical source -> typed HIR -> VerifiedProgram SSA
  -> deterministic baseline adapter -> verified lkjscript-native machine plan
  -> encoded image and symbolic relocations -> bounded lkjscript-sys RW/RX install
  -> actual System V AMD64 entry call
```

The adapter consumes only `VerifiedProgram`. It does not read source syntax,
HIR, or bytecode. One installed group contains the requested function and its complete reachable
direct-call SCC closure. Compatible generated callers use direct relocated
native calls and unboxed I64/F64/Bool/Unit values. Forced reference results are
marshaled as owned `GcHeap` snapshots; auto does not transfer references.

Current generated coverage is Unit, Bool, I64, F64, Str, legacy Buf, nominal
Product, List, Option, Result, block parameters, local mutation represented as
machine-plan locals, multi-block branches, loops, checked numerics, exact
supported equality families, constructors/accessors/immutable replacement,
string and legacy-buffer operations, direct and mutual recursion, return, trap,
exit, and structured outcome propagation. Indirect calls remain unsupported.

Native entry, call, and loop transitions use enum-identified versioned runtime
calls. canonical native contract frame registration records exact per-source-function native entries
and consumes the mandatory entry poll before body effects without duplicate
source-level `EnterFunctionV1` or entry `PollV1` calls. Explicit and backedge
`PollV1` calls consume bounded native poll fuel, check a monotonic deadline,
count polls, and propagate deadline, resource-limit, or host-clock status through the shared
invocation state. Native canonical native contract prologues register initialized frames; verified
transitive may-collect summaries publish dense caller safepoints only where a
callee closure can collect, and every structured edge unregisters. Generated
code never exits the host process.
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
statically reachable direct-call group before main effects. Unsupported semantics, compilation
failure, ABI failure, or native resource
exhaustion before invocation is an `EngineError`; structured execution limits
remain outcomes; forced mode never silently falls back to the
VM.

`auto` begins in the VM. Saturating function-entry counts synchronously compile
a hot function whose VM/native entry signature has an implemented scalar
adapter, and that call still runs in the VM. A compiled group may contain a
reference-signature helper for generated direct calls, but that helper remains
explicitly auto-entry-ineligible; later direct VM entries stay in the VM. Later calls use installed
native code. Unsupported or
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
safepoints with exact derived stack maps, source maps, exact explicit trap-site
identities/messages, and outcome maps,
compile/install statistics, invalidation state, metadata bytes, and native entry
counts. Limits cover per-object and aggregate code, metadata, work, object
count, diagnostic bytes, and compilation wall time. There is no persistence,
eviction, background compilation, concurrent mutation, or post-RX patching.

Safe sys APIs accept only opaque images emitted from verified closed plans.
Mappings transition RW to RX, are never RWX, expose no raw entry address, and
remain owned for every invocation. Scalar paths retain exactly empty maps. Reference paths home values and carry
verifier-certified non-empty exact maps. Generic bounded `HeapDispatchV1` sites
retain arbitrary argument homes (up to the language/backend bound), canonical
operation-specific input/result/layout facts (including product field and
collection payload identities), operation/source identity, allocation/store
class, and safepoint. Sys alone accesses raw frames; safe services receive
copied typed arguments/roots and return an exact typed value. After a service
rewrites moving roots, sys writes them home and re-materializes arguments before
the heap operation. No raw stack or object pointer
crosses that boundary.
## Unsupported Native Semantics

Forced baseline compilation rejects Symbol, Handle and host IO, function
references/indirect calls, lexical `Owned`/`Ref`/`RefMut`, polymorphic or
unsupported signatures, and any operation without an exact lowering. Auto
leaves reference-typed functions and all unsupported semantics in the VM with
same-epoch retry suppression. This is intentionally not a full host-capability
allocation JIT claim.

Machine diagnostics are opt-in and go only to stderr. They include normalized
SSA, generated bytes, relocations/code-object metadata, and native counts.
`LKJSCRIPT_JIT_DUMP_DIR` writes generated binary images and prints an external
`objdump` command where that tool is available.

Separate low-overhead `LKJSCRIPT_METRICS` records one machine-readable JSON line
on stderr; `LKJSCRIPT_METRICS_FILE` writes that line to an explicit file instead.
The versioned record includes exact scalar outcome bits, every compiler phase,
VM/engine/native durations, time to first native entry, first-call duration,
compile/install durations, tier states/counts/failures/fallbacks, code and
metadata/cache accounting, direct calls, PollV1 calls, allocation counts,
deterministic estimated object bytes, estimated peak-live bytes, collections,
and distinct attempted/successful heap calls. It does not currently report a
collection-pause distribution. Collection is
conditional; ordinary native-call hot paths do not read the clock. Normal
program stdout is never used for JIT diagnostics or metrics.
## Accepted Automatic Promotion Boundary

This boundary is selected but **not yet Current**. Existing auto baseline
compilation remains at exactly 64 VM entries. A separate CLI opt-in will enable
candidate proof-optimizing thresholds of 64, 256, 1,024, or 4,096 exact
baseline-native entries of the promotion root; optimizing remains disabled by
default until retained adoption evidence exists. Source main stays in the VM,
and the auto VM/native adapter stays scalar-only.

The Nth counted baseline entry synchronously proof-optimizes, lowers, and W^X-
installs while invoking the exact captured baseline object. The optimized
object is pending and non-selectable until a later root entry. Exact opaque
entry tokens bind function, object, and tier. Baseline and optimizing objects
coexist under one process-local session with one current and at most one
pending object; stale invalidated mappings are bounded and retained until
session drop but are never selectable.

The selected state names are `BaselineCandidate`, `BaselineCompiling`,
`BaselineNative`, `OptimizingCandidate`, `OptimizingCompiling`,
`OptimizingPending`, `OptimizingNative`, and `Disabled`. One attempt is allowed
per explicit epoch under a bounded total; same-epoch retries are suppressed.
Failure records a structured tier reason and leaves baseline current. A newer
epoch allows one bounded retry and invalidates pending/current optimized code
back to the retained baseline. Generated reference-signature helpers may call
and allocate within a native group, but cannot be auto entry roots or transfer
references across VM/native entry. Forced tier behavior remains unchanged and
fallback-free. The authoritative metrics, limits, transition rules, and
predeclared benchmark are in [Proof-Based Optimizing
JIT](../../decisions/jit/proof-based-optimizing-jit.md).

Loop OSR, background compilation, speculation, guards, deoptimization,
Handle/host allocation calls, native/VM reference transitions, and persistent
profiles or code caches remain future or rejected work as classified by the
active decisions. Forced proof-optimizing execution is Current; automatic
optimizing promotion is only the accepted selection above.
## Current Slice Evidence

On the containing commit based on `0daa7a0`, Linux x86-64 with Rust/Cargo
1.96.0 passed strict affected Clippy, focused core/native/sys/JIT/VM/app tests,
separate docs/tree/source checks, `quiet verify` (182 unit/integration tests plus
one compile-fail doctest), a locked workspace release build, scalar
VM/forced/auto smokes, Brainfuck smoke, and a forced allocation-graph smoke.
The graph smoke returned exact I64 `1` with 3 native entries, 7 allocations, 6
collections, maximum 3 roots, 14 successful heap calls, 6 barriers, and zero VM
fallback. This is historical evidence for that commit; attempted/successful
heap-call counters were separated later.
Docker, performance, and full Brainfuck Mandelbrot were not run.
