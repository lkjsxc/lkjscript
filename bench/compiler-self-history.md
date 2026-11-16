# Supplemental compiler self-use replay window

This supplemental campaign replayed **compiler-scoped source changes from
30 commits through 206 structural operations**, after attempting the first
**103** source-changing candidates in the frozen window. Every counted transaction was published and passed the
independent desired/candidate canonical checked-meaning comparison. Every
changed historical `src/` or `library/` filename appeared in its published
transaction's file facts.

The [original campaign](compiler-history.md), frozen at
`1f48bd24c90f98e833c3647d902ea0b4515d4d90`, remains **8/179**, with its requested
thirty unmet. This newer window includes this task's own operation-authored
compiler changes and measures contemporaneous self-use. Its thirty successes
include all eight original successes. The two windows and producers differ;
their counts are separate and are not additive.

**Thirty whole-commit replays remain unmet.** Seven of the counted revisions
also changed 30 `.lkj` or `.rs` paths outside `src/` and `library/`, in tests,
tools, benchmarks or witness code. Those changes are outside this campaign's
acceptance condition and were not replayed. The [outside-language-path table](compiler-self-history.outside-language.tsv)
records them exactly. The remaining 23 counted cases have every changed
`.lkj`/`.rs` path within the compiler scope; this does not add a claim about
non-language documentation or host-tool changes.

## Frozen population and execution

The new population is the last 300 commits at
`a4fd685a9bb2e7736110a4ed2a43651dfe040d56`, including merged history.
Each candidate is compared with its first parent. Of the 300 commits, 197
changed `src/` or `library/`; 103 did not. The unchanged policy orders the
197 candidates by added/deleted line count in those trees, then commit identity.
The campaign stopped at success thirty, at ordinal 103
(`28c1fda0b315b44489bdea9856aa7fac0d74feb6`). The remaining 94 candidates were
unattempted; the population was not expanded.

The immutable current compiler was copied before execution and checked again
afterward. Both copies have SHA-256:

```text
4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09
```

The run used one sequential compiler worker, a 4 GiB address-space limit,
a 900-second timeout per compiler command and a 2 GiB authenticated store.
There was no producer change, retry, resume, source rewrite for compatibility,
threshold adjustment or eligibility change during the campaign. A clean-stop
request arrived while ordinal 103 was running; that command supplied success
thirty and the campaign exited normally, without interruption.

The root policy remains `src/main.lkj` when both revisions contain it,
otherwise the common `src/lib.lkj` root. The filename projection retains each
historical `.rs` file byte for byte and adds a byte-identical `.lkj` copy;
compiler string literals remain untouched. Both historical library roots are
used when present. Otherwise both sides use the same frozen compiler's embedded
library. Only `src/`, `library/`, `tools/` and `tests/` are materialized.
No service, data directory, repository metadata or generation-one tree is copied
or run.

All thirty counted cases used `src/main.lkj` and historical library roots.
Twenty-two had the native `.lkj` projection; eight recorded the legacy
`.rs`-copy projection. Across all attempts, 59 selected the CLI root and 44
the library root; 82 used historical libraries and 21 the frozen embedded
compatibility environment. These distinctions remain in every measurement row.

## What the proof establishes

A counted case has a nonempty replay proof of this form:

```text
replay desired=HEX candidate=HEX verified=true operations=N
transaction source-before=HEX source-after=HEX mode=apply
```

The first line compares the driver's **canonical checked-meaning identity** in
domain `lkjscript-checked-meaning-1`. It is not a claim of byte-identical
serialized `.lkjir`, whose diagnostic paths can differ between the two
checkouts. Both sides were checked under the same frozen current compiler and
selected libraries; historical compiler binaries and historical behavioral
gates are outside this experiment.

The second line records actual publication after that comparison. Exact
source snapshot hashes retain their path-bound stale-input role. The separate
coverage check requires every changed historical `src/` or `library/`
filename, with only the stated `.rs` filename projection, to occur in the
transaction's exact file facts. A successful compilation or equality for a
smaller root alone is insufficient.

The [retained-proof auditor](compiler-self-history-audit.pl) checks the
retained population/order against their frozen hashes, the recorded producer
and bounds, nonempty proof, equal desired/candidate hashes, apply marker,
operation count and changed-file coverage. It verified all thirty successes.
The [proof table](compiler-self-history.proofs.tsv) preserves full revisions,
parents, canonical identities, source hashes and stdout/stderr hashes.

A separate read-only audit compared the complete population/order with Git and
checked the actual retained executable digest. The reproducible
[file auditor](compiler-self-history-file-audit.pl) reads Git blobs and the
published file facts: all 63 historical-before hashes match. Published after
hashes match exact target text for **48 of 63 files** and differ for **15**.
Those results are retained in [file proofs](compiler-self-history.file-proofs.tsv)
and [the file-audit summary](compiler-self-history.file-audit.log).
Canonical checked-meaning equality is the acceptance proof; exact target-text
reproduction is not established for those fifteen files.

## Measurements and resources

The campaign ran from 02:30:08 to 03:25:56 UTC on 2026-11-16. Overall elapsed
time was **3,347.85 s** (load 1.84→2.84), with 3,225.62 s user CPU and 49.99 s
system CPU under that same load interval. Compiler-command walls sum to
3,245.87 s; the remaining elapsed time includes archive, selection, filesystem
and harness work.

| Scope | Wall seconds | Peak KiB | Observed one-minute load |
| --- | ---: | ---: | ---: |
| Thirty counted compiler commands | 48.23–73.02; median 59.625; sum 1,794.48 | 1,351,096 | 1.51–5.00 |
| All 103 compiler commands | 0.32–97.96; median 35.04; sum 3,245.87 | 1,351,096 | 1.51–5.00 |
| Whole sequential campaign | 3,347.85 | 1,351,096 | 1.84→2.84 at campaign boundaries |

Per-command wall, user/system CPU, peak memory and both load readings are
retained in [the measurement table](compiler-self-history.tsv). The store is
shared across the sequential revisions under the fixed producing compiler;
these timings are not independent cold builds.

There were 104 free-space preflights: one before fixture creation and one
before each attempted expansion. The initial check found 118,626,676,736 B
available against 4,584,030,108 B required. The minimum recorded availability
was 118,062,620,672 B; the maximum required reservation was 6,465,306,524 B.
The formula remains twice the store budget, a block-rounded two-tree allowance
doubled for legacy copies (at least 128 MiB), the compiler bytes and a
128 MiB log reserve. [Every check is retained](compiler-self-history.tsv.space).

No setup failure, memory refusal, timeout or implementation-failure row occurred.
The worker has exited. After acceptance, the final owned source trees, store,
private key, internal compiler copy and armed stop marker were removed. The
[cleanup record](compiler-self-history.cleanup.meta) retains that result. One
verified compiler copy remains outside Git solely as the frozen input for a
separately assessed continuation; intentional text evidence remains preserved.

## Refusals and exclusions

All 73 non-counted attempts and their original diagnostics remain recorded.
Ten transactions proved their selected root equal and were actually published,
but changed source files outside their applied file facts; their `.scope`
records list those files, and they are excluded from the thirty.

| Recorded reason | Count |
| --- | ---: |
| `E_UNSUPPORTED_MACRO` | 21 |
| `E_IMPORT` | 15 |
| `E_REPLAY_FILES` | 11 |
| Changed source outside applied operation files | 10 |
| `E_PARSE` | 5 |
| `E_REPLAY_NO_OPERATIONS` | 5 |
| `E_UNSUPPORTED` | 4 |
| `E_REPLAY_DATA` | 1 |
| `E_REPLAY_MEANING` | 1 |

The include-data refusal names changed `library/prelude.rs` bytes. Those
historical include bytes were preserved; the harness did not rewrite the
compiler's string literals or introduce an unapproved data-file operation.

Ordinal 74, `cc9e7b83374a9e09a8aa7816d5353e578e09327c`, reached the independent
equality guard and produced this observed refusal:

```text
transaction E_REPLAY_MEANING desired=8d56c213f6c47430eb7098d532dcd23a4356be69c591bdd1e2826a318e31eba0 candidate=ee9f906876f41f7bd65902c4a47ea643ae58068c4b8389888f21caa8e72da2c1; no source was published
```

It remains an equality-guard refusal, with no causal attribution or retry.
The [exact diagnostic](compiler-self-history.tsv.logs/074-cc9e7b83374a9e09a8aa7816d5353e578e09327c.err)
and empty stdout are preserved.

## Counted commits

This table follows the actual attempt order. Full identities and proofs are in
the linked TSV files. The operation counts describe the derived replay request;
they need not match how the original author decomposed the change.

| Ordinal | Revision | Operations | Changed source files | Seconds (load before→after) | Commit subject |
| --- | --- | ---: | ---: | ---: | --- |
| 6 | `a52c3046` | 1 | 1 | 65.43 (2.38→2.26) | Use resolver intrinsic inventory for retained annotations |
| 7 | `d4901e46` | 1 | 1 | 73.02 (2.26→2.11) | Check index retention after the final semantic memo writes |
| 10 | `cac9d51f` | 4 | 2 | 69.69 (2.62→2.80) | Reject embedded-source edits even when a physical include has the same path |
| 11 | `6c270fd6` | 3 | 1 | 59.55 (2.80→2.53) | Reuse fully validated semantic fact pages within one query |
| 12 | `77469a10` | 2 | 1 | 53.91 (2.53→2.39) | Distinguish runtime lines from numeric file identities in mutation results |
| 13 | `a71ba370` | 1 | 1 | 68.32 (2.39→1.76) | Recover exact borrowed shorthand field labels from parser prefixes |
| 15 | `a9622bec` | 2 | 2 | 63.26 (1.73→2.62) | Check session deadlines between ready I/O chunks and document the boundary |
| 16 | `d654a960` | 4 | 1 | 65.64 (2.97→3.23) | Recover exact nominal reference sites after borrowed type prefixes |
| 25 | `9465e9e0` | 2 | 2 | 57.78 (2.92→2.73) | Record resolved Self constructor identities and target types |
| 26 | `a48d0c70` | 2 | 2 | 63.62 (2.73→3.33) | Prove narrow lexical bindings in undemanded library bodies |
| 27 | `20d5f7e9` | 3 | 1 | 67.37 (3.33→5.00) | Dispatch pinned structural queries before executable hashing |
| 28 | `5fe8d1ff` | 2 | 1 | 49.52 (5.00→3.63) | Explain borrowed-array and ASCII trim rewrites with safe source edits |
| 29 | `6d3185ac` | 2 | 2 | 68.35 (3.63→3.27) | Expose existing intrinsic spelling inventories for semantic edit guards |
| 39 | `d127f09c` | 8 | 2 | 53.88 (2.20→1.88) | Compare complete custom traps in optimizer and mutation oracles |
| 41 | `263f6a8f` | 3 | 2 | 59.70 (1.88→1.51) | Separate intrinsic value types from builtin predicates |
| 42 | `f307bb3e` | 3 | 1 | 53.20 (1.55→2.04) | Offer match-scope edits only for independently resolved owned results |
| 43 | `896ce05f` | 10 | 2 | 66.57 (2.04→3.17) | Bind semantic snapshot v2 to its producing compiler |
| 48 | `d2044ada` | 13 | 6 | 55.77 (2.91→3.19) | Reuse listener addresses and prove origin restarts under its real unit |
| 49 | `498345ee` | 3 | 1 | 56.60 (3.19→2.60) | Check undemanded library definitions before publishing structural edits |
| 50 | `1fd44221` | 10 | 2 | 65.58 (2.60→2.31) | Use declaration catalogs for source replacement and prove rename preservation |
| 51 | `d884c0fd` | 5 | 1 | 56.39 (2.31→2.13) | Prove namespace roles and retain grouped self import bindings |
| 53 | `4eef8920` | 13 | 2 | 49.27 (3.41→2.20) | Keep compiler generations warm and reuse closed semantic traps |
| 57 | `f9e8eb6e` | 9 | 3 | 63.17 (3.15→2.80) | Select exact rotations and canonical scalar results from measured hash costs |
| 62 | `9e64aa7f` | 5 | 3 | 63.17 (2.37→2.46) | Unify checked renames, prove move subsets and bound useful review |
| 64 | `c1baa3fe` | 3 | 2 | 48.23 (2.53→2.68) | Make four more consumer diagnostics directly rewritable |
| 73 | `ab13f540` | 16 | 6 | 53.61 (2.17→2.07) | Resolve qualified and aliased source sites and retain expression types |
| 78 | `1d50974f` | 9 | 3 | 60.09 (3.49→3.49) | Bound catalog string validation and expose declaration-only capture |
| 88 | `7f363514` | 6 | 2 | 53.94 (3.57→3.34) | Compare checked type graphs, layouts, instances and authority |
| 96 | `d317f002` | 21 | 1 | 59.11 (3.00→3.01) | Read pinned query snapshots through authenticated producer locators |
| 103 | `28c1fda0` | 40 | 6 | 50.74 (2.67→2.84) | Measure the compiler triple and emitted instruction kinds |

## Reproduction and retained evidence

Use the exact frozen compiler above and a private 32-byte key outside the
materialized source scopes. Choose a fresh output prefix:

```sh
mkdir -p target
LKJSCRIPT_HISTORY_HEAD=a4fd685a9bb2e7736110a4ed2a43651dfe040d56 \
LKJSCRIPT_HISTORY_MEMORY_BYTES=4294967296 \
LKJSCRIPT_HISTORY_TIMEOUT_SECONDS=900 \
LKJSCRIPT_HISTORY_STORE_BYTES=2147483648 \
LKJSCRIPT_HISTORY_KEEP_FIXTURE=1 \
bash bench/compiler-self-history-harness.sh replay . /path/to/frozen-compiler \
  /private/replay.key target/compiler-self-history.tsv 30 300
```

The [frozen harness copy](compiler-self-history-harness.sh) retains the exact
executed bytes. Its changes from the shared harness add optional fixture retention, record the harness digest
and apply the same requested resource bounds to recovery. The ordering,
selection, completeness condition, checked-target comparison and normal
publication path are unchanged. The executed harness SHA-256 is
`5fea2592cf9dd13fe311ef7858dc455efc97a8f5b35a7e66e786ca8cec4af795`.
The frozen revision-list SHA-256 is
`dfe6dd8098ed931ce5ccd847cab1ab6411d769c6b4c22a80bc8682babbc0cc4b`;
the ordered-candidate SHA-256 is
`1bc2a477d3655301f30edef8a3d18b468d3af2489b06d8879838ebc224bc89e4`.

Retained records include the [300 revisions](compiler-self-history.revisions),
[197 ordered candidates](compiler-self-history.order),
[measurements](compiler-self-history.tsv),
[campaign metadata](compiler-self-history.tsv.meta),
[publication/equality proofs](compiler-self-history.proofs.tsv),
[audit output](compiler-self-history.audit.log),
[run metadata](compiler-self-history.run.meta),
[overall timing](compiler-self-history.run.time),
[progress log](compiler-self-history.run.log),
[raw-record SHA-256 manifest](compiler-self-history.raw.sha256) and all per-attempt
[stdout, stderr, timing, changed-path and scope records](compiler-self-history.tsv.logs).
No private key, store record, source fixture or executable is part of this report.
