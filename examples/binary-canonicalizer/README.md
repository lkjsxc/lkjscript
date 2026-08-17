# Binary canonicalizer

This retained production-path application repairs a typed byte placeholder into `bytes_concat`,
validates a marker, scans a payload, removes zero padding, and carries an immutable byte accumulator
through a counted loop. Its workspace entry returns a named record/variant; its standalone
`bytes_stream` entry maps raw input to canonical output bytes.

The driver uses exact contract roots and two direct CLI sessions for 43 logical calls. It checks
sparse and dense vectors, deterministic failure and fuel boundaries, durable rename continuity,
workspace reopen, and historical/current revisions. It then validates three immutable release
cases, performs validate-only and two byte-identical application builds, proves a failing release
case publishes nothing, deletes the source workspace, and exercises standalone validate, inspect,
test, typed run, stream run, and corruption rejection. Its dense input boundary accepts 1,445 bytes
and rejects 1,446. Lock contention and corrupt reopen are tested without a socket service.

`bytes_concat` preserves immutable value semantics and charges full result length as logical fuel.
The production ownership route may reclaim dead values and reuse a verified unique accumulator; the
allocate-new route is its differential oracle. Ownership and allocator operations are not language
syntax.

The generated `.lkja` artifact is a run-only semantic closure, not workspace history, a reusable
package, serialized Core IR, or a sandbox. The driver uses private temporary paths and removes all
artifacts when it exits.

Run from the repository root:

```sh
./examples/binary-canonicalizer/run.sh
```
