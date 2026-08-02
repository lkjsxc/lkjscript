# Polymorphic Deterministic Value Plane

## Status

**Accepted Target; not Current.** Concrete structural witness identities and
copy-list regions remain Current. This record binds the next integrated public
cutover; no residual witness, package witness, sealed language value, or
structural-owner list claim follows until every named verifier and execution
boundary passes.

## Decision

Typed HIR memory planning is the sole semantic authority. A concrete hot generic
instance may be specialized under deterministic profile limits. Every retained
residual generic body receives ordered hidden static memory-witness slots. The
source language exposes neither witnesses nor lifetime, region, allocator,
reflection, retain, release, pointer, or memory-engine syntax.

The value decision order is: borrow for nonescaping observation; move at last
use; reuse or fuse a unique private builder; clone a small independently owned
value; otherwise seal and share one immutable domain. Failure never selects a
dynamic box, tracing, collector fallback, unchecked operation table, or universal
reference count.

## Executable Witnesses

Every exact installed type has one nonzero content-addressed descriptor. Its
canonical facts close semantic type and runtime layout identity, aggregate mode,
deterministic closure, storage/domain and root projection, checked size and
alignment, copy/borrow/move/clone/fuse/share/drop behavior, equality and codec
eligibility, list-element behavior, portability, contention, allocation/release
limits, closed operation identities, and ordered child witness IDs.

One executable closure installs a sorted immutable table. Validation rejects a
zero ID, conflicting duplicate, missing dependency, illegal static cycle,
unsupported operation, stale route, unbounded table, and canonical-ID mismatch
before effects. A validated artifact may use a dense local slot but remains bound
to the full ID. Descriptors contain no runtime root, address, provider, resource,
owner token, process state, or mutable cache.

Operation identities are compiler-owned closed enums, never source names or
function pointers. Backends consume the verified descriptor and route; they do
not synthesize behavior.

## Generic ABI And Specialization

A memory-complete function signature records only the minimal ordered witness
capabilities reconstructed from its body. A direct call either selects a checked
specialized identity or binds exact caller/concrete slots. Hidden slots remain
separate from source locals, operand stacks, ownership roots, and wire values.
Indirect generic calls remain rejected unless an existing callable carries the
exact witness signature.

The selected policy is hybrid. Initial hard limits are 16 witness parameters per
function, 16 witness arguments per call, 32 specializations per declaration,
1,024 per package closure, 16,384 installed descriptors, 65,536 dependency
edges, 64 dependency depth, 16 MiB specialized code, and 1,000,000 planner work
units. Exhaustion retains a valid residual body or rejects when the operation
requires specialization. Selection depends only on typed input, exact package
closure, resource profile, stable costs, and canonical order.

Specialized and residual HIR/SSA semantics are identical. Independent checking
binds each specialization to source function, substitutions, witness IDs, body,
and call rewrite. Proof certificates preserve the complete witness table,
requirements, bindings, and specialization identities.

## Package Contract

A public generic export binds type and trait parameters, minimal witness
requirements and order, parameter memory signatures, result ownership, codec
constraints when public, and the exact semantic contract digest. Imports verify
the complete record before compilation or effects.

The exact package lock binds each exported generic interface digest, witness
requirement digest, package-closed witness ID, and emitted specialization
identity. It carries no subsystem version and supports no stale compatibility
path. Cross-package source remains generic and cannot observe hidden arguments.

## Sealed Values

A fully initialized immutable value without affine resource, mutable owner,
escaping short borrow, or illegal dependency cycle may use selected sealed
storage when cloning is materially inferior. Build is private and unique. Seal
preflights memory, release work, side drops, complete layout/type initialization,
loan absence, and deduplicated acyclic dependencies before atomic publication.

Ownership is one checked non-atomic worker-local region count or exact owner set.
Scope borrows add no owner traffic. Independent lifetime divergence adds one
coarse owner. Internal fields, nodes, strings, enum payloads, and list entries
have no counts. Final release iteratively executes side drops and dependency
releases without payload traversal. This retained region count is reported as
coarse reference counting.

## Structural Segmented Lists

Selected persistent lists accept copy scalars and exact immutable witnesses for
string, path, deterministic product/enum, option, result, and recursively nested
list. Bytes require a separately proved immutable independently shareable
witness. Byte-vector, resource, exclusive borrow, unique mutable owner, affine
closure, and every other affine element reject.

A segment entry is exactly inline, static artifact, fused payload, sealed-domain
dependency root, or nested-list root dependency. It never copies a session root
key. One segment retains one entry per independently owned child domain after
deduplication. A unique moved front may be reused.

Prepend selects borrow, move/fuse, clone/embed, or coarse share from the element
witness and use plan. First borrows when nonescaping and otherwise copies,
transfers, clones, or acquires one coarse owner. Rest borrows a cursor, transfers
a consumed owner, or acquires one segment/tail owner. Equality remains restricted
to the existing statically eligible family and dispatches only through the exact
element witness. Release is iterative.

## Process Boundary

Process snapshots contain bounded semantic DAG nodes and local IDs only. They
never encode dense witness slots, runtime roots/keys, domain/segment identities,
owner tokens, loans, or addresses. Exact semantic/witness identity appears only
when required to validate the wire type. Decode validates all bounds, resolves an
installed local witness, privately constructs domains, and publishes only after
complete success.

The prerequisite core snapshot is one closed reverse-topological table. Every
node carries exact nonzero semantic-type and layout identity plus one matching
closed payload kind. Child IDs are table-local `u32` values and must name earlier
nodes; the sole root is the final node and every table node must be reachable
from it. Lists use exact empty or nonempty nodes, and every nonempty tail has the
same list type/layout identity. A self edge, forward edge, cycle, missing edge,
unreachable node, kind disagreement, invalid text/path, trailing bytes, or any
node, edge, depth, byte, or work bound rejects. Decode builds an unpublished
private table; ordinary ownership of partial vectors provides deterministic
cleanup on every failure, and an `OwnedValue` is created only after complete
validation.

This table and its process-outcome codec are an experimental prerequisite. They
permit mixed product-list-product snapshots and shared immutable subgraphs in
the core boundary model only. They do not select sealed language storage,
promote structural list elements, resolve an installed witness, or import the
snapshot into compiler, evaluator, VM, or native runtime ownership domains.

## Predeclared Selection Evidence

Generic candidates are full specialization, residual dispatch, and hybrid.
Value candidates are detached clone, fuse, one-node sealed domain, coarse sealed
dependency, and private immutable non-atomic per-node RC control. The production
per-node candidate is adopted only if every non-node candidate breaches one of:
2.0x operation p99, 1.5x peak logical bytes, 2.0x copied bytes, or an exact
semantic/safety gate. Threshold changes require a new recorded experiment.

Promotion records same-commit compile time, native code bytes, descriptor/table
bytes, validation work, dispatches, copied/fused/shared bytes, dependency and
owner operations, atomic operations, logical live/RSS/over-retention, list and
release p50/p95/p99, process encode/decode p50/p95/p99, and malformed rejection.
Losing production candidates are deleted; compact negative evidence remains.

## Promotion Gate

Current status requires independent HIR and SSA verification, validated bytecode,
evaluator and VM residual execution, forced baseline and proof execution with
nonzero native entries and zero VM/semantic fallback, exact package import/lock
rejection, all selected list classes and nesting, key-free isolated-process
return, complete cleanup accounting, unconditional no-tracing verification,
private no-per-node-RC falsification, bounded metrics, and one integrated
platform-revision cutover. Partial slices remain explicitly experimental.
