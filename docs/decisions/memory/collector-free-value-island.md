# Collector-Free Value Island

## Status

<!-- LKJ-STATUS id=collector-free-value-island status=accepted-contract -->

**Accepted contract; not Current until all listed types execute through every
required engine with zero collector interaction and zero fallback.**

## Type Set

The minimum island is:

- never, unit, bool, complete-range i64, and exact-bit f64;
- capability values;
- all eleven typed external resources;
- byte-vector, byte-slice, byte-slice-mut, bytes, and path.

Dynamic strings, symbols, products, enums, option, result, list nodes, closures,
and other registered structural families remain outside the island.

## Eligibility

A function is eligible only when every parameter, result, local, constant,
instruction result, call signature, runtime operation, owner, loan, allocation,
and cleanup edge has an island plan. No legacy-traced value may be reachable.
Eligibility is computed before source effects.

Missing operation support is a verifier or engine error. VM, baseline JIT, and
proof JIT cannot use tracing or fall back after eligibility succeeds.

## Execution Domains

The independent evaluator uses abstract deterministic owners and fake providers.
The VM uses typed scalar slots, unique storage, resource slots, and explicit
cleanup. Native tiers consume the same memory-verified SSA and closed runtime
call table. Island native frames contain owners, loans, flags, resources,
budgets, deadlines, outcomes, and transitions, but no collector root map or
collection poll.

## Required Evidence

Each island execution records zero deltas for:

- `GcHeap` allocations;
- collections;
- root materializations;
- collector barriers;
- VM fallback;
- collector fallback.

Completion additionally requires zero untransferred unique owners, live loans,
owned resource obligations, cleanup flags, and release backlog.

Representative fixtures cover scalar limits and exact f64 bits, byte mutation
and slicing, freeze/thaw, path construction, file write and read, explicit and
implicit close, trap and early-return cleanup, stale resource keys, and
allocation failure. Forced tiers report nonzero generated entries.

## Remaining Collector

The existing collector remains only for exact registered non-island structural
families. This capability does not promote the whole runtime to collector-free.
