# Compiler structure: frozen history campaigns

The last fifty comparisons all succeeded and printed at most 23 lines. Replay
exhausted the original three hundred commits with **8 strict complete-source
successes, comprising 84 operations**. The requested thirty replays were not
reached. Another five transactions proved the desired compiler root identical,
but left changed semantic-test sources outside their applied files; they are
listed separately and are not counted as complete replays.

## Population and proof boundary

The population is frozen at `1f48bd24c90f98e833c3647d902ea0b4515d4d90`.
`git log -n 300` supplies the replay population, and each candidate is compared
with its first parent. There are 179 commits changing `src/` or `library/` and
121 without a change in those trees. Candidates were ordered by changed-line
count, then commit identity. All 179 were attempted; the population was not
expanded to obtain more successes.

Both campaigns used producer SHA-256
`0a4ab39752523a29f7936a589a5c53fd029d8a466f638b6d4f951fababee6aa0`.
This is the tested operations candidate containing the duplicate-input and
checkout-prefix comparison repairs. The binary was copied before each campaign.
These are current-checker proofs of historical source, not claims that a
historical compiler binary was reproduced.

The selected root is `src/main.lkj` when both trees have that CLI root, otherwise
`src/lib.lkj` when both have the library root. Legacy `.rs` files are retained
byte for byte and also copied under `.lkj` filenames for the current loader.
Compiler string literals are never rewritten. When both historical library
roots exist, each side uses its own root. Otherwise both use the same frozen
compiler's embedded library; that compatibility environment is explicit in
every data row. Only `src/`, `library/`, `tools/`, and `tests/` were materialized.
No `.generation-one` directory, repository metadata, service, data directory,
release or unrelated project was copied.

A counted replay requires all of the following:

- A nonempty transaction applied through the structural edit command.
- The candidate is elaborated and checked by the normal compiler path.
- The driver prints equal desired/candidate checked-meaning identities in
  domain `lkjscript-checked-meaning-1`.
- Every changed historical `src/` or `library/` filename, with only the stated
  `.rs` filename projection, appears in the transaction's exact file facts.
  This last test excludes compiler-only changes whose accompanying test sources
  were not part of the selected root. Documentation and host-tool changes are
  outside this compiler-source campaign.

The source-byte snapshot digest remains bound to its source paths for stale
transaction checks. Comparison uses separately paired input bytes across the
two checkouts. Exact checked-IR identities, which retain diagnostic metadata,
are recorded separately from checked operational meaning; the latter is not
substituted into exact store proof reuse.

## Bounds and resources

Each compiler subprocess ran under a 4 GiB address-space limit and a 240-second
timeout, with a 2 GiB authenticated store budget. There was one sequential
compiler subprocess per campaign. The two campaigns overlapped after the root
released a second worker; their summed command walls are not combined elapsed
time. Each command's wall, user/system CPU, peak memory and before/after load are
preserved in the TSV files.

Before initial fixture creation and each expansion, the harness checked free
space against twice the store budget, a block-rounded two-tree allowance doubled
for legacy copies (at least 128 MiB), the binary, and a 128 MiB log reserve.
An excessive-reservation witness refused before creating a store or logs:
156,133,253,120 bytes were available versus 2,000,287,235,609 required.
Temporary source trees and stores were removed after completion. No campaign
subprocess remains running.

| Campaign | Result | Command wall, seconds | Peak KiB | Observed load range |
|---|---:|---:|---:|---:|
| Replay | 8/179 strict; 84 operations | strict range 50.66-64.04, median 58.385; all commands sum 2143.63 | 977300 | 3.66-12.84 |
| Comparison | 50/50; maximum 23 lines | range 12.12-54.24, median 33.84; commands sum 1763.66 | 970884 | 4.38-13.57 |

## Replay results

The eight strict successes are:

| Revision | Operations | Wall seconds | Peak KiB | Load before -> after |
|---|---:|---:|---:|---:|
| `77469a10` | 2 | 59.69 | 935292 | 5.40 -> 6.39 |
| `5fe8d1ff` | 2 | 54.21 | 922104 | 7.19 -> 6.18 |
| `d127f09c` | 8 | 59.68 | 924632 | 5.04 -> 5.67 |
| `f307bb3e` | 3 | 57.09 | 912204 | 5.67 -> 5.58 |
| `d2044ada` | 13 | 64.04 | 925880 | 6.42 -> 7.40 |
| `4eef8920` | 13 | 55.63 | 851716 | 7.13 -> 7.56 |
| `c1baa3fe` | 3 | 50.66 | 915584 | 9.83 -> 7.89 |
| `28c1fda0` | 40 | 61.26 | 851696 | 9.54 -> 8.84 |

All eight use the historical CLI root and historical libraries; their source
files use the explicitly recorded legacy filename projection. Full parent and
desired/candidate identities are in [replay proofs](compiler-history-replay-proofs.tsv).

| First recorded outcome | Commits |
|---|---:|
| `E_IMPORT` | 25 |
| `E_PARSE` | 13 |
| `E_REPLAY_DATA` | 1 |
| `E_REPLAY_FILES` | 21 |
| `E_TYPE` | 1 |
| `E_UNSUPPORTED` | 15 |
| `E_UNSUPPORTED_MACRO` | 90 |
| `changed-source-outside-applied-operation-files` | 5 |
| `desired-checked-meaning-identical` | 8 |

The 144 older library-root candidates all refuse current-language checking:
90 first report unsupported macros, 25 imports, 15 unsupported constructs,
13 parsing, and one typing. Examples include `env!`, host Rust I/O and async
constructs; historical source was left unchanged. The 35 CLI-root candidates
split into eight strict successes, five narrower equalities, 21 file-pairing
refusals and one changed include-data refusal.

The 21 `E_REPLAY_FILES` cases expose a concrete missing operation path: mostly
newly loaded files such as `src/native/storage_plan.lkj`, diagnostic modules,
`src/reuse.lkj`, and `src/host_network.lkj`. One extension-transition case lacks
an unambiguous old/new include-file counterpart. These are refusals at the first
unsupported step; adding file creation alone is not evidence that all later
framing checks would pass. The single `E_REPLAY_DATA` case changes
`library/prelude.rs` as embedded source data. A source-item transaction cannot
silently update that separately loaded data file. Full first diagnostics are
preserved in [refusals](compiler-history-refusals.tsv).

The five narrower proofs are:

| Revision | Operations | Changed source outside applied files |
|---|---:|---|
| `931e8a5c` | 3 | `src/semantic_tests/optimizer.rs` |
| `df5cf3e4` | 4 | `src/semantic_tests.rs`, `src/semantic_tests/scalar_clone.rs` |
| `8af5fd89` | 2 | `src/semantic_tests/optimizer.rs`, `src/semantic_tests/optimizer/wrappers.rs` |
| `f7f2d7ea` | 3 | `src/semantic_tests.rs`, `src/semantic_tests/panic_messages.rs` |
| `81048410` | 13 | `src/semantic_tests/optimizer.rs`, `src/semantic_tests/optimizer/forwarding.rs` |

Together the thirteen valid compiler-root equalities contain 109 operations.
Only the eight with complete source-file coverage count toward the requested
replay target. This is the measured result for this producer, planner, root
selection and frozen population, not a claim that no richer operation could
reproduce the refused commits.

## Fifty bounded comparisons

All fifty comparisons passed. Thirty-two report `checked IR differs`; eighteen
report `text identical, checked IR identical`. Every response is either three
or twenty-three lines under `--limit 20`. This population contains no
`text differs, checked IR identical` result, so historical evidence does not
establish a comment-only or rename-only example. Those properties belong to the
separate focused comparison tests and must not be attributed to this table.

[Raw comparison responses](compiler-history-comparisons.txt) preserve each
bounded answer. [Comparison proofs](compiler-history-compare-proofs.tsv) record
both checked-meaning identities, both exact IR identities, the verdict, fact
counts, and omitted detail count for every revision. The independent
[comparison timings](compiler-history-compare.tsv) retain load beside every
measurement.

## Repairs exposed by the campaign

The initial diagnostic trials are excluded from the final counts. They found
two implementation defects before this fixed run:

- Module discovery probes a file before loading it, so a snapshot can contain
  repeated reads of the same path. The planner now coalesces equal-byte copies
  and refuses conflicting copies.
- Different checkout prefixes initially produced 14,106 irrelevant
  `projection-file` facts and a false source-text difference for an unchanged
  program. Comparison now pairs stable files and compares their bytes while
  preserving the path-bound transaction snapshot digest.

Include-only files are paired using directories inferred from the root and
stable source identities. Longest-prefix correspondence is required, ambiguous
directory mappings refuse, and reverse file pairing remains bijective.
Byte-identical binary data bypasses source-token framing. The real historical
compiler witness `d632a233 -> 77469a10` passed after these repairs: two operations,
equal checked target/candidate, 57.49 seconds, load 3.99 -> 3.71, 948,220 KiB peak.
It was rerun in the final population and counted there only once.

## Reproduction and pausing

`bench/compiler-history.sh` is the reusable harness. Pass an available binary
matching the recorded producer hash to reproduce this exact producer run; a
new binary yields new evidence and needs a distinct output basename. Example:

```sh
LKJSCRIPT_HISTORY_HEAD=1f48bd24 bash bench/compiler-history.sh \
  replay /absolute/repository /absolute/compiler /private/store-key \
  target/compiler-history-replay.tsv 30 300
LKJSCRIPT_HISTORY_HEAD=1f48bd24 bash bench/compiler-history.sh \
  compare /absolute/repository /absolute/compiler /private/store-key \
  target/compiler-history-compare.tsv 50 300
```

Creating `OUTPUT.stop` pauses after the active bounded subprocess. Resume with
`LKJSCRIPT_HISTORY_RESUME=1` and the same arguments. The harness requires the
same history head, producer hash, limits and schema, skips completed revisions,
and records that the resumed store is cold. A completed-success resume witness
runs no new compiler process; a different-producer witness refuses before store
creation. The final campaigns did not need a pause or resume.

Supporting data: [population inventory](compiler-history-population.tsv),
[replay measurements](compiler-history-replay.tsv),
[replay metadata](compiler-history-replay.meta), and
[comparison metadata](compiler-history-compare.meta).
