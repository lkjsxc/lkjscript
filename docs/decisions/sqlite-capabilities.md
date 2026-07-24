# SQLite Capabilities

## Status

**Accepted Target** for the Candidate A vertical slice. This record defines a
generic runtime facility, not an application storage model.

## Decision

`lkjscript-sys` will call the Linux system SQLite C library directly through a
small owned wrapper. No third-party Rust crate or application SQL enters the
runtime. The current development image exposes only
`/usr/lib/x86_64-linux-gnu/libsqlite3.so.0`; the wrapper links that exact system
SONAME. Deployment images must provide compatible `libsqlite3.so.0`.

The language exposes handles for connections and prepared statements. A
statement records its parent connection. Closing a connection with a live
statement is rejected; finalizing a stale, finalized, cross-connection, or
closed handle is an explicit failure. Resource-table teardown uses SQLite's
safe deferred close so statement finalization cannot outlive connection state.

## Surface and limits

The capability covers open flags, close, busy timeout, prepare, finalize,
reset, clear bindings, null/I64/F64/text/bytes binds, step row/done status,
column count/type and nullable values, changes, last inserted ID, extended
result code, static SQL execution, and online backup. Extension loading is not
exposed. Arbitrary pointers never cross into the language.

Returned text and bytes copy before the next step/reset and are capped by the
existing buffer limit. UTF-8 is validated exactly; invalid SQLite text is a
structured operation error. SQLite codes are retained but error messages are
operation-qualified and bounded.

## Boundaries

The runtime does not define table names, SQL statements, migrations,
transactions, sessions, auth, or backup retention. Language-level wrappers own
transactions and cleanup. Candidate A will use VM execution: SQLite, strings,
buffers, allocation, and host I/O are outside the scalar JIT subset.

## Verification

Focused tests cover in-memory and file databases, prepared statements,
bind/reset, nulls, I64 boundaries, UTF-8, blobs, syntax/constraint errors,
connection/statement lifetime, busy timeout, online backup, and restoration.
Application durability and migration evidence remain consumer responsibilities.
