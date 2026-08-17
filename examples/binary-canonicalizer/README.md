# Binary canonicalizer

This retained production-path example creates an incomplete typed program, rejects an invalid repair, and refines the same byte placeholder into `bytes_concat`. The repaired program validates a one-octet marker, scans the payload, removes zero padding, and carries an immutable `bytes` accumulator through a counted loop. Its successful result nests the exact bytes in a named record and variant.

The driver uses exact machine-schema roots and one CLI session, checks sparse and dense vectors, exercises deterministic failure and fuel boundaries, renames presentation metadata, restarts the daemon, and reruns both historical and current revisions.

`bytes_concat` retains immutable value semantics and charges the complete result length as logical fuel. The production ownership route can reclaim dead intermediate values and reuse a verified unique accumulator; shared or unsuitable storage uses the allocate-new fallback. The example exposes no ownership, retain, release, or allocator operation.

Run it from the repository root:

```sh
./examples/binary-canonicalizer/run.sh
```
