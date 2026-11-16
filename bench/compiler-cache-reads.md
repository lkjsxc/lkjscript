# Cache reads without creating entries

`store::cache_file` now uses the existing `Directory::open_regular` operation.
A missing cache record remains absent; a final symlink or special file is a
miss; a readable regular file is opened read-only. The previous adapter used
`open_append` and then enabled nonblocking reads, so a miss could create an
empty file before authentication rejected it.

This is a read-contract repair. It changes neither authenticated envelope
validation nor the full first-writer budget reconciliation and checkpoint
rules. Invalid regular envelopes still fail authentication and follow the
existing disposal path. Special leaves cannot be opened for payload data and
remain subject to the existing writer reconciliation. No header scan, capacity
check, orphan cleanup or producer binding has been removed.

## Reproduction and acceptance

The direct pre-fix control reproduced the unwanted write: `Store::get` returned
None in an initially empty directory, but left a 114-byte `.budget-index` and a
zero-byte `.budget-lock`. The test deliberately failed its unchanged-directory
assertion. Its before/after manifests and status are retained in
[validation evidence](compiler-cache-reads-data/validation/).

The [new focused script](../tools/cache-read-test.sh) passes **25 cases**:

- ordinary missing record and missing historical locator leave no entries;
- readonly ordinary, historical root and historical page records remain usable;
- historical root/page misses occur after a valid authenticated locator, using
  the exact pinned root identity and a valid page digest;
- ordinary, root, page and locator reads reject symlinks, dangling symlinks and
  FIFOs without blocking, changing the store manifest or changing their targets;
- forged envelopes and authentic envelopes substituted from another namespace
  remain misses.

The historical probe is a copy of the publisher with appended bytes, so its
executing-producer digest differs. The direct probes test the authenticated
transport API; their small payloads are not claims to be decoded Snapshot v2
metadata. The separate existing semantic-query suite checks actual snapshots,
cursors, provenance, admission separation and hostile index records.

| Validation | Outcome | Wall s | User + system s | Peak RSS KiB | Load before→after |
| --- | --- | ---: | ---: | ---: | --- |
| Pre-fix no-create control | intended unchanged-directory assertion failure | 1.00 | 0.97 | 76,264 | 8.79→8.79 |
| New direct contract suite | 25 cases pass | 1.12 | 1.13 | 77,504 | 8.74→8.74 |
| Existing full store suite | pass | 52.04 | 57.97 | 68,400 | 7.89→8.93 |
| Existing semantic-query suite | pass | 7.54 | 7.30 | 78,968 | 8.61→9.06 |

The store suite includes exact byte reproduction, authentication failures,
capacity and oldest-generation eviction, interrupted publication, unindexed
orphans, stale size accounting, hostile entries, unavailable storage and
concurrent publishers. Its existing two-worker section received a temporary
reservation; that reservation was released before the query suite. All commands
ran under an external 4 GiB address-space limit and 900-second watchdog. The
focused hostile-leaf probes additionally use a 30-second watchdog. The retained
load files contain all three load-average components. No full gate was run by
this worktree.

## Fixed small observations

The [protocol](compiler-cache-reads.protocol.md) was frozen before construction
and timing, SHA-256
`9a0262333aae0d0b9a8adfb6e044b52ea85a44cc008ade989d92543a95039d77`.
The [harness](compiler-cache-reads.sh) was frozen before its only launch,
SHA-256 `bb7e8e96f063a724da9e7fd5790be089cccde4e0781dffa86ff76b96d99204b4`.
It runs two setup commands, exactly twelve observations, then two separately
labelled controls. There were no retries or omitted observations.

Both compiler executables were constructed at the same physical source path
with the same immutable emitter, explicit library/runtime source, inline limit
64 and no store. Baseline is source revision
`71e821dfcaaf6a7940ed67bb7d2d3a721c169922`; the candidate changes only this
adapter and its comment. The unchanged emitter is
`71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7`.

| Construction | Wall s | User + system s | Peak RSS KiB | Image bytes | Load before→after |
| --- | ---: | ---: | ---: | ---: | --- |
| Baseline | 43.45 | 42.37 | 1,048,892 | 31,731,800 | 9.12→9.17 |
| Candidate | 38.80 | 38.00 | 1,072,220 | 31,727,704 | 9.19→9.92 |

Executable hashes are:

- baseline: `39692138d3cf8eef4fefe301cbc7aaf8752821a3f0ad3dc5c1818b6210dc66aa`;
- candidate: `9da04d786c383d7114b28e3aeef37cd5e748d430cfda6b78002a06ad5fdfe0c8`.

The scalar specimen has one file: main calls answer, which returns 42. Both
executables independently build and index that same physical source path into
separate empty stores. The specimen commands use `--no-library` and the
unchanged embedded runtime from the two construction manifests; they do not
pass `--runtime-source`. Output inlining is fixed at zero. Historical queries
from both executables use the baseline's authenticated snapshot and store.
Every absent-locator observation uses a new empty store and the same valid
64-hex absent identity.

The table shows all twelve observations in execution order. Wall milliseconds
come from the nanosecond timestamps around argv-metadata recording and wrapper
launch through completion; this is not a syscall-only interval. GNU wall,
CPU, RSS, exact arguments, output hashes and file counts are retained in the
[complete TSV](compiler-cache-reads-data/observations.tsv) and
[raw records](compiler-cache-reads-data/observations.raw/).
Every row had load **8.64→8.64**; the three components were
**8.64 / 9.05 / 9.51** before and after each command.

| Case | Executable | Wall ms | Exit | Files in observed store |
| --- | --- | ---: | ---: | ---: |
| Unchanged build | baseline | 472.800058 | 0 | 28 |
| Unchanged build | candidate | 533.228469 | 0 | 28 |
| Unchanged build | candidate | 313.032032 | 0 | 28 |
| Unchanged build | baseline | 302.305962 | 0 | 28 |
| Pinned outline | baseline | 2.386545 | 0 | 28 |
| Pinned outline | candidate | 2.080050 | 0 | 28 |
| Pinned outline | candidate | 1.856721 | 0 | 28 |
| Pinned outline | baseline | 2.302367 | 0 | 28 |
| Absent locator | baseline | 1.457372 | 1 | 1 |
| Absent locator | candidate | 1.786650 | 1 | 0 |
| Absent locator | candidate | 1.650204 | 1 | 0 |
| Absent locator | baseline | 1.446482 | 1 | 1 |

Each unchanged build reports cached-image reuse. All emitted specimen images
match exactly, including the no-store control:
`fd9b50060e47d72c783f54f9abdd35e740604d205af0cfc465d200cf7d2c84fb`.
All four historical-query outputs match exactly. Both reader generations report
the baseline producing compiler, as the pinned-query contract requires. The
original interpreter control returns 42. Every expected locator refusal has
empty stdout. The two baseline misses each leave one zero-byte locator; the
two candidate misses leave no files.

The two-observation means are 387.553010→423.130250 ms for unchanged builds,
2.344456→1.968385 ms for pinned queries, and 1.451927→1.718427 ms for absent
locators. These small fixed-order observations move in both directions. They
do not isolate syscall cost, establish a throughput gain or qualify an
optimization milestone. Compiler hashing, host contention, wrapper work and
ordering remain in command time; the candidate image is only 4,096 bytes
smaller. The separate construction rows likewise do not establish a gain.

This repair is outside the frozen producer used by
[the indexed-store attribution](compiler-cache-index-condition.md). It cannot
explain away that experiment's global header/index work, and no large-program
or indexed-edit measurement was repeated here.

## Checked edit and retained scope

One `replace-item` operation changed `store::cache_file` and passed independent
candidate checking before publication: **48.93 s**, **1,130,612 KiB** peak,
load **9.01→9.90**, with 12,692 freshly checked instances. It removed the
reachable file-nonblocking effect and gained no entry authority. The request,
full 215-file source manifests, file before/after record and `mode=apply`
publication output are retained in
[the adoption evidence](compiler-cache-reads-data/adoption/).

Request SHA-256:
`fe38db9701b9f08b2cd91a94b5c333630a7a8d3ca6ebb949c8f87ca2076a86ff`.
Final `src/store.lkj` SHA-256:
`79a5502d8d478540917cf22caafba8fcf8d314d2a2dbae828f2235d7e460d4e4`.
The source chain was independently checked against the parent Git blobs and
current bytes. The retained src/library diff has nine changed `.lkj` lines; seven are
backed by the operation. Deleting two preceding stale ordinary comments was
outside the item's AST span and remains explicitly uncredited. There were no
failed source-edit attempts.

All generated test and observation trees, stores and private keys have been
removed. Textual proofs and operation records remain. The two compiler
executables and their common emitter are retained only until integration
review; no background compiler jobs remain.
