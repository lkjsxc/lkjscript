# First-Party Relational Database Roadmap

## Purpose

Define a vertical path to a PostgreSQL-class first-party relational server
without mistaking an interoperability wrapper or toy store for that product.

## Status

Generic bounded SQLite capabilities are **Current** for interoperability,
bootstrapping, migration tools, test oracles, and differential checking. No
first-party storage manager, WAL, B+tree, MVCC engine, SQL engine, or database
server is Current. Those components are **Accepted Targets**.

## Decision

The first-party database is relational from its first architectural slice and
uses a B+tree-oriented storage architecture. The accepted sequence is:

```text
versioned page format and checksums
  -> page manager and buffer pool
  -> write-ahead log and deterministic crash recovery
  -> B+tree search, insert, split, merge, and recovery
  -> catalogs
  -> MVCC, transactions, snapshots, and isolation levels
  -> vacuum or equivalent version cleanup
  -> relational operators
  -> typed query IR and cost-based optimizer
  -> SQL parser
  -> network server protocol
  -> backup, restore, replication, and fault injection
```

Every early slice belongs to this final architecture. Page identity, checksums,
WAL ordering, torn-write assumptions, recovery state, transaction visibility,
and resource limits are explicit and versioned. Recovery and fault injection
must be deterministic and testable.

Application schemas, migrations, transactions, and domain policy remain in
lkjscript. SQLite stays outside the first-party implementation and may serve as
an oracle; it is not the internal storage engine.

## Current-Cycle Kernel

A bounded in-memory B+tree node/search/update workload may be added as a general
ownership, allocation, bounds-check, and optimizing-JIT acceptance kernel. It
is performance evidence only and must not be called a database implementation.

## Future Requirements

The final system includes checksummed pages, buffer management, WAL, crash
recovery, B+tree indexes, catalogs, MVCC, transactions, snapshots, isolation,
version cleanup, typed query IR, cost-based planning, relational execution,
SQL, server protocol, backup/restore, replication, fault injection, and
recovery testing.

## Deferred

All database implementation beyond a general B+tree kernel is **Deferred**
until ownership, packages, exact native allocation/GC, async I/O, and durable
storage capabilities are sufficiently Current.

## Rejected

Calling the SQLite capability, a SQLite wrapper, a disposable key-value store,
or a B+tree benchmark PostgreSQL-equivalent is **Rejected**. Using SQLite as
the first-party database's internal engine is also **Rejected**.
