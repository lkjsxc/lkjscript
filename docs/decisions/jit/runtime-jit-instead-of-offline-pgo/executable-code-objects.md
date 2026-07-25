# Runtime JIT Instead of Offline PGO: Executable Code Objects

[Authority](../runtime-jit-instead-of-offline-pgo.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Executable Code Objects

A callable code object records at least:

- entry address and code size;
- semantic and native ABI versions;
- source function or region identity;
- tier identity;
- relocation metadata;
- safepoints and precise GC stack maps;
- OSR entries when present;
- trap and side-exit metadata;
- accounted metadata and executable sizes;
- invalidation state.

Linux x86-64 executable memory follows W^X: allocate writable, emit and
relocate, transition to executable and non-writable, then execute. Pages are
never writable and executable simultaneously. Unsafe OS interaction remains in
`lkjscript-sys`; its safe API upholds Rust safety for every caller.

The code cache has configurable limits for executable bytes, code-object count,
compilation time, active or queued work, and metadata bytes. Start with bounded
retention. Eviction is **Deferred** until return addresses, direct calls, OSR
relationships, and invalidation are modeled safely.
## GC, Safepoints, And Outcomes

Native safepoints are required at allocations, allocation-capable host and
function calls, loop backedges, yields/deadline checks, and VM/native
transitions. Every safepoint has an exact map of live references. Arbitrary
machine words are never conservatively scanned as pointers.

Native frames become visible to the collector before allocation-capable native
code is accepted. Baseline code may initially call the existing allocator and
collector; escape analysis, regions, stack allocation, and generational
strategies remain later measured candidates.

Callable native execution does not begin until chunk validation, structured
process-safe VM outcomes, process-safe exit, explicit traps, exact local/stack
initialization, and the still-required semantic migrations are complete. The
JIT returns the same structured outcomes as the VM. Generated code does not
directly terminate the host process for language exit, trap, timeout, or a
resource limit.
## Minimal AOT Test Surface

File-based native emission is retained only where it shares the native backend
and helps:

- inspect generated machine code;
- test relocations and calling conventions;
- compare output and structured outcomes with the VM;
- use external disassemblers and debuggers;
- validate code generation without executable-memory machinery;
- measure backend compilation independently.

This test/AOT harness does not require training workloads and is not the main
optimization or product strategy. It must not grow profile generation, profile
merging, profile-use rebuilds, PGO-specific decisions, or persistent profile
formats.
## Engine Selection Contract

The current CLI implements all four modes below. Ordinary `run` defaults to
`auto` with a conservative 64-entry threshold, while explicit `vm` remains the
deterministic path:

```text
lkjscript run --engine vm <file.lkjscript> [--] [script-args...]
lkjscript run --engine auto <file.lkjscript> [--] [script-args...]
lkjscript run --engine baseline-jit <file.lkjscript> [--] [script-args...]
lkjscript run --engine optimizing-jit <file.lkjscript> [--] [script-args...]
```

Semantics are:

- `vm`: never execute JIT code;
- `auto`: start in the VM, collect bounded hotness, and tier automatically;
- `baseline-jit`: require the executable entry region and every reached user
  function to compile and execute at baseline native tier when first reached;
  a reached unsupported function is an error rather than a VM fallback (host
  runtime calls are explicit ABI calls, not required user functions);
- `optimizing-jit`: require the executable entry region and every reached user
  function at the proof-based optimizing tier; if Tier 2A is unavailable the
  engine is rejected before execution, and it never downgrades to baseline or
  VM while reporting optimizing success.

Normal defaults preserve fast startup and do not compile tiny programs merely
to demonstrate JIT existence. JIT diagnostics use stderr or separate structured
output and never contaminate program stdout.
## Benchmark Contract

Retained tiered results compare applicable variants among:

```text
reference VM
baseline JIT including compilation
baseline JIT steady state
optimizing JIT including compilation
optimizing JIT steady state
```

A result records repository/tree identity, workload and oracle hashes, CPU and
target mode, OS/kernel, tool versions, warmup, trigger latency, compilation
time, first-native-execution time, end-to-end and steady-state time,
time-to-break-even, compilation/OSR/fallback/deoptimization counts, native code
and metadata bytes, peak RSS/code cache, repetitions, dispersion/tails, and
artifact cleanup.

Compilation and warmup cost are part of the result. A faster native steady state
is not an end-to-end speedup when total execution remains slower.

Required workload classes include cold hello, direct lkjscript Mandelbrot,
numeric loops, call-heavy recursion, allocation and branch stress, byte/buffer
processing, a long OSR loop, one-shot HTTP, repeated warm server requests, and
Brainfuck Mandelbrot interpreted by lkjscript. Direct and
Brainfuck-interpreted Mandelbrot remain distinct workloads.
## Adoption Gates

Baseline JIT acceptance requires:

1. byte-for-byte output and return-value equality with the VM;
2. identical structured traps, errors, malformed-input behavior, and exact
   numerics;
3. GC correctness across native safepoints;
4. identical resource-limit behavior;
5. VM/native and native/native call correctness;
6. bounded code and metadata caches;
7. forced mode evidence that native code actually executed;
8. repeated performance evidence including compilation cost.

OSR additionally requires exact locals/stack/loop-header reconstruction, GC
correctness during transfer, timeout/cancellation behavior, repeated transfers,
and a workload that enters native code during one long invocation.

Proof-based optimizing JIT additionally requires differential SSA optimization
tests, isolated and combined variants, compile-time and code-size budgets,
whole-program benefit on a declared workload, and no hidden material retained
workload regression.

Guarded specialization and deoptimization have separate gates and are not
implied by a non-speculative optimizing tier.
## Implementation Sequence

1. With explicit equality current, finish explicit-main/effect-free-import,
   local-mutation/immutable-global, fixed-point-effect, chunk-validation, and
   structured-outcome prerequisites.
2. Implement typed SSA, verifier, differential evaluator, and isolated proven
   optimization passes.
3. Implement the selected owned
   [Linux x86-64 native backend](../../execution/linux-x86-64-native-backend.md), shared native
   code-object boundary, and minimal non-PGO file emitter; add relocations, ABI
   tests, runtime calls, and precise stack maps before allocation paths.
4. Add bounded call counters and synchronous function-triggered baseline JIT,
   VM/native calls, direct native calls, and forced baseline testing.
5. Add sound ownership/coherent traits, exact native frame roots, allocation,
   barriers, recursion, and collection exercised while generated frames are
   active.
6. Add a distinct proof-based optimizing engine, then implement the selected
   process-local synchronous promotion boundary and run its retained threshold
   gate. The forced first engine is Current; automatic promotion remains
   disabled by default and unimplemented.
7. Add bounded loop-backedge counters and verified OSR in a later cycle; use
   unmodified Brainfuck Mandelbrot only as an appropriate long-loop diagnostic.
8. Consider guarded specialization and deoptimization only from measured need.
9. Validate Linux AArch64, direct Wasm, and server policies later; background
   compilation and persistent profiling require their own prerequisites or a
   new decision.
