# Binary canonicalizer

This retained production-path application repairs a typed byte placeholder into `bytes_concat`,
validates a marker, scans a payload, removes zero padding, and carries an immutable byte accumulator
through a counted loop. Its workspace entry returns a nominal record/variant; its exported
`canonicalize_stream` function maps raw input to canonical output bytes.

The driver uses exact contract roots and two direct sessions for 43 logical authoring calls. It
checks sparse/dense vectors, fuel and bounds traps, rename continuity, workspace reopen, and
historical revisions. It then constructs a workspace-independent reusable release with five
explicit exports, private implementation, and three cases. Validate-only and two publications
produce canonical-equal release bytes.

Application contract 2 consumes that exact release, runs the three release and three application
cases, and publishes a format-2 embedded graph bundle twice with equal bytes. A failing case
publishes nothing. The driver removes the workspace state and then validates, inspects, tests,
typed-runs, stream-runs, and corrupts the immutable release/application inputs. Its dense input
boundary accepts 1,445 payload bytes and rejects 1,446. Lock contention and corrupt workspace
reopen are tested without a socket service.

`bytes_concat` preserves immutable value semantics and charges full result length as logical fuel.
The production ownership route may reclaim dead values and reuse a verified unique accumulator; the
allocate-new route remains its differential oracle. Ownership and allocator operations are not
language syntax.

The generated `.lkjr` and `.lkja` files are exact semantic authorities for release and application
domains. They contain no workspace history, resolver state, serialized Core IR, provenance,
signature, or sandbox. The driver uses private temporary paths and removes all artifacts on exit.

Run from the repository root:

```sh
examples/binary-canonicalizer/run.sh
```
