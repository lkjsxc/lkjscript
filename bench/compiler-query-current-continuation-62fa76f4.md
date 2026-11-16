# Current compiler query checkpoint continuation

Status: **prepared; no question has run**. Root must separately release this
committed continuation after review. It executes only the original 50 questions
for the first time, using the two already completed builds. It never repeats a
build or performs a metadata query, map walk, census, repair, replacement
question, cache flush or retry.

## Preserved first attempt

The original harness was launched once from `c17b7ded`, request
`compiler-current50-launch-1068`, session
`pipe-elmmaojdjipcefjalmglpkmflkffimna`. It ended with status 1 during offline
build validation. Both native compiler commands had completed with status 0.
There were **zero query launches**.

| Build | Exact wrapper ns | GNU wall / user / system s | Peak RSS KiB | Load before -> after |
| --- | ---: | --- | ---: | --- |
| Cold index | 109,215,156,001 | 109.21 / 105.98 / 1.22 | 2,554,260 | 2.39/2.98/3.46 -> 2.24/2.83/3.36 |
| Unchanged index | 691,635,177 | 0.68 / 0.65 / 0.00 | 44,732 | 2.24/2.83/3.36 -> 2.24/2.83/3.36 |

Both images are 32,196,802 bytes and directly compare equal to canonical
`34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377`.
Both stdout snapshot records are byte-identical. The unchanged build reports
`build_reused=1`, `store_writes=0`; it did not run frontend/index construction.

The cold compiler's own counters report 3,654 definitions, 12,793 IR functions,
1,240,173 IR instructions, 8,336.727 ms catalog construction and
63,922.057 ms index construction. Its reported retained-store counter was
751,986,967 bytes. This is a compiler counter, not a post-run filesystem census.
The unchanged command's zero retained-store counter is not evidence of an empty
store. No current-query latency or coverage result exists yet.

All **35 first-attempt evidence files** are frozen under
`bench/compiler-query-current-data/attempt-1`, including the original
`builds.json`, partially written `builds.tsv`, validator stderr, completion,
raw outputs, statuses, timings and inventories. Their manifest SHA-256 is
`ab1b3b95fe8f14cd0202eaa6ffeeb01a274a43afd08102b4bb75ef252d91f963`.
The original target evidence remains untouched. Original controls are also
copied under `attempt-1-controls`; the original plan, harness and validator
remain at their original paths and hashes so both earlier preparation manifests
remain verifiable.

## Concrete offline correction

Three fixture assumptions are corrected in the separately named
`bench/compiler-query-current-continuation-validate.pl`,
SHA-256 `ab8df55c21037c5dae20d9b723155f113bc134d64b4a059406c073d486e47204`.

1. Common build requirements still include actual total/hash/Store timings,
   `build_reused` and `store_writes`. Frontend/index timing and IR-count
   requirements apply when the image was not reused. Missing unchanged-path
   phase/count fields stay **NA**, rather than being invented as zero.
2. `save()` uses a fresh pretty JSON encoder. It no longer mutates the compact
   shared encoder later used for TSV values and quoted query identities. The
   original mutation caused `TSV framing` even for an empty error array.
3. An explicit-pin query bypasses current-producer hashing. Its
   `producer_hash_ms` timing is optional and validated numerically if present;
   absence stays **NA**. The mandatory `producer_bytes_hashed=0` check remains.

The first correction was validated only against copied captured build records
under `offline-validation-1`. Peer review identified the optional query timing
before any question ran. The final correction was then validated offline under
`offline-validation-2`; both build results and pin bytes are identical to the
first corrected validation. Neither pass executed a compiler or query.
Each retained validation includes its exact validator bytes and invocation.

The accepted derived build JSON has SHA-256
`1c3efd8040ab9325bde44bb7338e4632d5024ccf9bec12cb7d949f0ed4344ca7`;
the accepted pin JSON has SHA-256
`df099c176412e90723cc3f26ee6601fc53e8838087951ad2d827b4e0b0ccd532`.
These derived records do not replace the failed original validation.

## Bound checkpoint and execution

The exact checkpoint is
`bench/compiler-query-current-continuation-62fa76f4.json`.

| Identity | Value |
| --- | --- |
| Snapshot | `9ff914ad9f4ef15c9bc5c61cb5d0718abcfde00322243a7a52a73f6b08d3534e` |
| Source | `d01c7eb47d92c7aefc7490b83f9bbc05025f6197871a7f90c51dfcad1576bb2b` |
| Checked IR | `4613bd709ee1d9238dca31e51b25d091abcd21a77f018090fedb8db8032c0fa2` |
| Producer | `34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377` |
| Fixture | `/home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-query-current-run.TDPm4g` |
| External private-key directory | `/tmp/compiler-query-current-key.seAq58` |
| Original evidence | `/home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-query-current-62fa76f4` |
| New continuation evidence | `/home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-query-current-continuation-62fa76f4` |

No key contents are copied. After explicit root release, use:

```sh
bash bench/compiler-query-current-continue.sh \
  --checkpoint bench/compiler-query-current-continuation-62fa76f4.json \
  --results target/compiler-query-current-continuation-62fa76f4
```

The harness refuses an existing result directory, changed frozen controls,
changed original evidence, any prior query record, changed pins, image/producer
mismatch or ambiguous fixture/key ownership. It reuses the existing store and
never constructs or repairs a snapshot. A missing/corrupt pin is recorded by the
scheduled requests; it does not authorize a rebuild.

Timed query argv, cwd, timing boundaries and resource limits are copied from the
original harness. The source cwd is the owned archive; the entry is
`src/main.lkj`, library selector `library`, and source grant `.` with no
`--source-dir`. Every query uses the exact explicit pin, 96 facts and 32,768
total output bytes. Core depth is 1; supplemental depth is 2. Each command has
2 GiB address space, 30 seconds and a 10-second termination grace. The default
per-extent reader cap remains unchanged. Refusals remain rows while the
predeclared remaining questions run once each.

After all 50 requests, validation processes only captured outputs. Source
inventory equality compares canonical path/digest mappings and retains both raw
inventories: the original locale-sensitive ordering differs from the gate's
ordering without any source-byte difference. This prevents an ordering-only
false failure. The executable identity must also remain unchanged.

## Pause and cache qualification

The actual sequence is **cold indexed build, unchanged indexed build,
validation stop, offline inspection/correction pause, then a separately released
50-question segment**. It is not an uninterrupted original harness execution.
The final build ended at nanosecond timestamp
`1794826898674428422`; the continuation captures its own first-query timestamp
so the pause can be reported exactly afterward.

During the pause, reviewers read raw outputs, source/inventory text and native
images, and offline validators hashed/compared the images. Root also completed
cleanup of seven unrelated repair artifacts before the continuation release.
No query, map walk, census or snapshot-node validation was used to warm a timed
question. OS page-cache state and host load may nevertheless change during this
pause. No causal latency comparison with historical producers is supported.

Keep the original two build timings unchanged and report the pause explicitly.
Keep successful latency counts separate from empty/unavailable/partial answer
coverage. Fixture/store/key cleanup remains prohibited until root live audit
acceptance. This addendum supplies no query outcome and makes no timing-target
claim.
