# Scalable Structural And Repository-Index Limits

## Purpose

Define the accepted limit and ownership contract for the first structural-scale and complete-index cut.

## Status

**Accepted Contract.** This record binds the implementation described below but does not claim it Current until the
runtime, graph, query, machine inventory, all-tier tests, documentation, and retained evidence pass together.

## Limit Classes

Every migrated production limit has one stable identity and exactly one class:

1. semantic law;
2. representation or addressability maximum;
3. implementation safety maximum;
4. resource-profile quota;
5. query or output budget;
6. implementation geometry; or
7. test fixture boundary.

A machine limit record names its unit, scope and lifetime, authority, default or profile source, lowerability, safety
maximum, responsible operation, typed failure, atomicity, metrics and evidence, and source/wire observability. Equal
numbers do not imply shared authority.

Cumulative work never decreases. Live domains, roots, objects, views, destinations, and retained bytes reserve before
publication and release exactly once. Peak use is monotonic. Rollback restores live capacity without erasing cumulative
work. Output budgets govern one publication and do not truncate hidden authority.

## Structural Values

`u32` local node IDs own the addressability maximum. A selected runtime profile owns a lower semantic-tree-node quota.
Domain dependency release work is a different unit and does not bound flat-image internal nodes. The default production
runtime must accept at least 65,536 nodes while preserving checked arithmetic, fallible growth, private construction,
atomic publication, iterative traversal, and deterministic release.

The Current contiguous node, field-cell, and byte vectors remain the baseline. Segmented and hybrid images are accepted
only if measured scale, allocation-failure, sharing, or locality evidence beats the flat form without changing source or
wire semantics. Profile maxima are never preallocated.

A domain is a coarse independently releasable owner. More than 4,096 sequential publications must reuse released slots;
`max_domains` remains a live-capacity quota and must not be raised to conceal retained owners.

## Calls And Tail Transfer

A verified `borrow-shared` immutable structural parameter reuses the caller's physical owner for the bounded call. It
does not allocate a clone or domain. The callee cannot retain, consume, mutate, return, or independently release the
owner unless another verified mode authorizes an explicit owner operation.

Evaluator, VM, baseline JIT, and proof JIT consume the same pre-backend parameter mode. A backend may not invent a copy.
Malformed mode, type, representation, place, or cleanup metadata rejects before effects.

A tail transfer preserves argument owners, transfers moved affine arguments once, and releases every outgoing dead copy
owner before frame replacement. Failure before replacement leaves the original frame valid. This cut does not claim
general native tail-call elimination.

## Repository Graph And Queries

A successful repository graph is complete for its declared tracked input and extraction classes. Construction either
publishes the complete canonical graph or returns typed exhaustion. Node and edge maxima never select a successful
prefix, and every edge in a successful graph has both endpoints. Identity covers the complete canonical nodes, edges,
contract, and base revision.

Graph work and retained-byte limits remain implementation safety maxima. Query work and serialized-byte limits are view
budgets. A bounded query result reports a closed completion status, exact stop reason, consumed limits, deterministic
omitted frontier, ordering contract, graph identity, and whether continuation is supported. This cut may report
continuation as unsupported; a bare truncation boolean is insufficient.

## Atomicity And Failure

Allocation, arithmetic, live-capacity, graph-construction, and query-publication failures preserve their original typed
cause. No partial root, object, graph, identity, or output becomes live. Generated index output remains under `target/`
and is never semantic, status, or evidence authority.

## Acceptance

Promotion requires:

- deterministic machine-readable affected-limit inventory;
- 4,095, 4,096, 4,097, 16,384, and 65,536-node boundaries;
- exact profile plus-one and arithmetic-overflow rejection;
- iterative publication, projection, clone, equality, export, codec where applicable, and release;
- more than 4,096 sequential publish/drop cycles with slot reuse and zero final obligations;
- the reproduced Brainfuck `Domains` failure removed by general borrow/tail ownership behavior;
- evaluator, VM, forced baseline, and forced proof agreement without fallback for the selected workload;
- complete deterministic current-revision graph above 4,096 nodes;
- typed graph exhaustion and structured query completion tests; and
- one coherent contract, public-fact, platform-revision, documentation, and evidence cut.

## Deferred And Rejected

Incremental graph rebuilds, graph sharding, resumable cursors, persistent sealed list segments, residual codecs, and
indirect generic callables remain separate accepted or deferred work. This reordering does not complete them.

Rejected shortcuts are broad numerical replacement, unbounded trusted mode, successful partial indexes, hidden query
omissions, benchmark-specific ownership, universal per-node reference counting, tracing fallback, and process-lifetime
retention.
