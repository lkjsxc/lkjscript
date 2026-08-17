# Binary canonicalizer

This retained production-path application repairs a typed byte placeholder into `bytes_concat`,
validates a marker, scans a payload, removes zero padding, and carries an immutable byte accumulator
through a counted loop. Its successful result nests exact bytes in a named record and variant.

The driver uses exact contract roots and two direct CLI sessions for 43 logical calls. It checks
sparse and dense vectors, deterministic failure and fuel boundaries, durable rename continuity,
workspace reopen, and historical/current revisions. Its dense input boundary accepts 1,445 bytes and
rejects 1,446. Lock contention and corrupt reopen are tested without a socket service.

`bytes_concat` preserves immutable value semantics and charges full result length as logical fuel.
The production ownership route may reclaim dead values and reuse a verified unique accumulator; the
allocate-new route is its differential oracle. Ownership and allocator operations are not language
syntax.

Run from the repository root:

```sh
./examples/binary-canonicalizer/run.sh
```
