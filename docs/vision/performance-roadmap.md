# Performance Roadmap

## Purpose

Define a measured path toward category-leading runtime performance without
turning aspiration into a current release claim.

## Status

The reference interpreter, exact I64/F64 execution, precise mark-sweep, and
tail-frame reuse are **Current**. Typed HIR/SSA, native AOT, direct Wasm,
hybrid memory management, PGO, and JIT are **Accepted Targets** or **Deferred**
until their recorded correctness and measurement gates pass.

## Sequence

```text
truthful semantics and safety
  -> reproducible category scorecard
  -> resolved typed HIR
  -> AI-first semantic core migration
  -> typed SSA and differential evaluator
  -> early Linux x86-64 native AOT
  -> ownership, escape, allocation, and memory candidates
  -> PGO AOT
  -> direct Wasm
  -> baseline JIT
  -> optimizing JIT and runtime specialization
```

AOT is intentionally early: it exposes the native performance ceiling before
server/framework breadth hardens weak representations. The compact VM remains
the conformance oracle and cold-execution candidate. Both lower from one typed
semantic pipeline.

## Current Interpreter

The VM uses dense bytecode, contiguous stacks, tagged small I64 values, boxed
wide I64/F64 values, precise non-moving mark-sweep collection, and
return-adjacent frame reuse. Source is compiled on every CLI invocation. Host
effects block synchronously.

Historical debug figures and single-shot C comparisons lack preserved machine,
variance, and artifact data and therefore remain diagnostic rather than a
baseline.

## Immediate Foundation Work

1. Establish the category and metadata rules in
   [performance-scorecard.md](performance-scorecard.md).
2. Use the current resolved typed HIR and canonical operation registry as the
   sole semantic boundary for every migration and backend.
3. Continue the semantic-core migration after the landed Unit, strict-if,
   Option, and typed-empty-list slices: comparison split; explicit main; local
   mutation and immutable global data.
4. Validate public chunks and return process-safe VM outcomes.
5. Lower HIR into typed SSA with explicit block parameters, traps, and effects.
6. Differentially test a minimal owned Linux x86-64 AOT backend against the VM.

## Representation Direction

Reference tagged Value is not the native hot-path ABI. Typed lowering uses
native I64/F64/Bool values, typed pointer/length views, flattened products, and
specialized Option layouts. Generic code is monomorphized where measured code
growth permits. Dynamic dispatch is explicit rather than a default call path.

Vec, Slice, Bytes, Str, views, and fixed products are performance-default data
shapes. Linked List remains explicit. Candidate memory strategies combine
unboxed scalars/products, unique owned buffers, regions for temporary data,
worker-local generational collection, immutable shared bytes, and explicit GC
references only where cycles require them.

## Native And Wasm

The first native candidate is an owned baseline x86-64 assembler/backend with
portable, x86-64-v2, x86-64-v3, and native target modes. Build-time mature
backends may be evaluated later under separate dependency and performance
records; no choice is permanent without evidence. Linux AArch64 ABI checks
begin before x86-specific assumptions become structural.

VM-in-Wasm is a reference path. Direct typed-SSA-to-Wasm is the browser
performance path and follows native AOT closely enough to keep IR design
portable.

## PGO And JIT

Local profiles bind to source hash, IR/compiler version, target CPU, and
workload and are never telemetry. PGO AOT precedes JIT. Baseline JIT requires
process-safe outcomes and callable code objects. Optimizing JIT requires a
deoptimization contract and must beat PGO AOT on declared warm workloads after
including warmup and code-cache costs.

## Resource Modes

Normal safety mode checks deadlines/epochs at loop backedges, calls,
allocations, host calls, and yields. Deterministic metering is a separate,
explicitly slower basic-block or instruction-counted mode. Heap, stack, native
code cache, handles, tasks, queues, IO volume, wall time, and allocation volume
all receive host-configurable limits; unlimited execution is explicit trusted
local mode.

## Adoption Rules

Every candidate follows [experiments.md](experiments.md) and the
[performance scorecard](performance-scorecard.md). Correct output, trap
behavior, and ABI conformance are mandatory before timing. Results record
isolated and combined variants, median and dispersion, code size, RSS,
allocation/copy counts, target CPU, and cleanup. No geometric mean hides a
material workload regression.

## Deferred Product Work

Package/update, server/framework, browser product APIs, and GUI remain later
product layers. Their designs may proceed only when they do not freeze current
semantic, ownership, or native-representation defects into public contracts.
