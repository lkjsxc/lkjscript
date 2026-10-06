# Share exact immutable dependency interface records within one admission

Decision: each complete source-admission operation owns a lazy pool of immutable
public-interface record maps, keyed by the exact logical package revision. Every
importing snapshot retains a shared handle to that map. Keep structural type maps,
local owners, dependencies and mutable application state separately owned.

## Boundary and motivation

A diamond or fan-out dependency graph repeatedly attaches the same supplier's
public records to different importing snapshots. Previously each attachment copied
all records and their variable-sized children. KernelSnapshot cloning repeated
that copying again. Existing aggregate copy-work admission bounded the growth but
did not remove it. The prepared-witness sharing mechanism addresses a later stage
and cannot eliminate these source-admission copies.

Materialize each needed record map once from its independently admitted interface
inventory. Reuse the whole map across direct import edges and snapshot clones.
The immutable inventory borrow binds the pool to one admission: it cannot be
retargeted to another input set or used as a process-wide revision cache. Drop the
pool after attachment; returned snapshots own their retained maps. The last
snapshot releases them. There is no independent global retention root.

## Meaning and authority do not come from a cache hit

The exact revision remains the lookup key. Equal names, equal contract shapes,
matching owners from another revision, and process-local addresses are not package
identity. Dependency unification, declared direct visibility, interface admission,
canonical source reconstruction, full semantic checks, private-body admission,
interface/body comparison and complete composed-callable validation remain at
their original owners. All selected inputs are read and checked for each load.

The pool shares record storage, never successful validation decisions, normalized
programs, application values, grants, secrets, borrow state or cancellation state.
The independent package oracle still reconstructs its own record maps rather than
using the production pool. A previous successful load cannot make corrupted input
acceptable. Source integrity, semantic validity and execution authority remain
separate.

The bootstrap uses Arc around the complete map. A provisional mutation must obtain
its own copy through copy-on-write; it cannot alter other snapshots. Such a mutable
logical fixture is not independently accepted merely because its previous records
were shared. This representation is internal and replaceable, not a new language
memory-management policy or an address-based semantic identifier.

## Bounded ownership and failure

The existing aggregate validation-visit budget charges each incoming attachment
and each newly materialized map/catalogue entry before their growth. Record copies
and their variable children are charged once, before copying. Publish a map to the
operation-local reuse catalogue only after its complete record projection exists.
Exhaustion cannot expose a partial record map as a successful shared projection.
Later failures discard the incomplete overall admission and publish no readiness.

Flattened dependency type maps remain snapshot-private. Their type and child-copy
work continues to be charged separately for every importing snapshot, even when
records are shared. No existing limit is increased and no per-package budget reset
is introduced. Tiny or rarely reused interfaces can require more bookkeeping work;
fewer record copies do not imply uniformly cheaper validation.

## Evidence and remaining costs

The maintained fan-out fixture measures actual record-copy counts and admission
visits on serialized containers retained from the source-matched predecessor.
It compares the same bytes and retains unchanged type-copy/read-byte observations,
not only newly generated similar source. Exact/N-1 budget cases, cross-snapshot
isolation, separate admission identities, reclamation and corrupted-input controls
exercise the real attachment and source-admission owners.

The [public diamond workload](../guides/native-shared-dependencies.md) exports a
recursive generic library before two independent concrete readers exist. Both
readers share its contract but retain distinct private implementations. A consumer
borrows one cell through both readers, then consumes it, and runs after all four
authoring projects and their transports have been removed.

No semantic graph, package interface, type object, bytecode or artifact encoding
changes. Shared type tables, cross-operation caches, further runtime deduplication
and complete source-validation cost are separate work. Measurements do not establish
RSS, total heap, runtime speed or API-cost savings. [Status](../status.md) owns
actual source and finalized-byte acceptance and publication.
