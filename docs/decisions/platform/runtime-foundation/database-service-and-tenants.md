# Coordinator Database Service And Application Tenants

## Status

**Current on Linux x86-64** for the typed tenant provider, coordinator lifecycle
attachment, daemon-owned service, durable recovery, and focused evidence named
in the Current-state record. Process-cell database proxy operations and source
language database operations remain an Accepted Target.

## Decision

`lkjscriptd` opens one coordinator-owned ordered database service after the
independent bootstrap control store is available. Database failure never hides
or replaces bootstrap identity, registry, local control, or recovery
diagnostics.

Each stable application registry identity maps to exactly one tenant name. Each
running application incarnation receives a fresh tenant-bound typed
`DatabaseProvider`; no database file, native handle, transaction object, or
other tenant identity crosses the provider boundary.

## Provider Identity

A provider has one nonzero process-local provider identity, stable tenant, and
nonzero application incarnation. Every transaction handle binds all three plus
a monotonic nonzero slot. Wrong-provider, stale-incarnation, unknown, closed,
or wrong-kind handles fail before database effects. Slots are not reused.

The provider implements bounded begin-read, begin-write, get, put, delete,
ordered range, commit, abort, and abort-all. Dropping or detaching a provider
aborts all active transactions. Write commit remains the engine's exact
single-writer serializable commit; reads retain immutable snapshots and do not
observe later commits.

## Coordinator Lifecycle

The daemon composition opens or creates the database service only after
`MachineCoordinator::start` has opened and validated the control store. It then
attaches the service to the coordinator. Running recovered applications receive
providers for their current incarnations.

Start attaches only after a new incarnation exists and before the lifecycle
operation is reported successful to local control. Stop, restart, remove,
shutdown, and cell failure abort all transactions and release that
incarnation's provider. Clean coordinator shutdown checkpoints the database
after every provider is detached; abrupt death retains WAL recovery. Restart attaches a provider with the replacement
incarnation; stale transaction handles cannot cross the boundary.

Local application status reports whether the current incarnation has a database
tenant attached. The stable tenant name derives from the durable registry
identity, not runtime-local application identity.

## Current Engine Semantics

The initial engine is an ordered multi-tenant key/value kernel with one global
writer and immutable reader snapshots. WAL commit synchronizes before publishing
the new index. Recovery applies only complete committed sequences after the
checkpoint and rejects malformed committed data. Tenant prefixes preserve exact
ordered isolation.

The provider does not reinterpret SQLite operations or preserve SQLite aliases.
Application-visible database language operations and process-cell provider
proxy messages remain a separate required cut; attaching a provider alone does
not claim those source operations Current.

## Bounds

Tenant names are at most 128 bytes, keys 4 KiB, values 1 MiB, ranges 4,096
entries, provider transactions 4,096, and one logical write buffer defaults to
8 MiB. The coordinator registry bound limits attached tenant providers.

## Rejected Alternatives

- Opening the database before validating bootstrap control is rejected.
- One provider shared across tenants or incarnations is rejected.
- Persisting process-local transaction handles is rejected.
- Treating the new engine as a SQLite implementation or compatibility layer is
  rejected.
- Silently continuing a lifecycle transition after abort-all failure is
  rejected.
