# Explicit-unit scale ladder

Run from a quiet, fixed compiler revision after the checking, interface, object
and source-projection controls pass:

```sh
bash bench/unit-scale.sh --compiler target/lkjscript \
  --results bench/unit-scale-results-REVISION
```

This is an outside-gate experiment. It runs one compiler process at a time and
copies the selected producer once. Coordinate with other streams before running.
The default rungs are exactly 10,000, 100,000, 1,000,000 and 10,000,000 authored
source lines. `--rungs` selects an increasing subset or a small integration smoke;
`--skip-real` omits the separately labelled ordinary-program specimens.
`--repository` permits the harness branch to measure another checkout.

## Workload and counting

`unit-scale-generate.pl` makes an explicit `UNITS`/`DEPENDS`/`ENTRY` manifest
and a connected, balanced dependency DAG. Leaves contain at most 10,000 source
lines. A three-line joining unit calls two children, and a five-line entry prints
their aggregate. Maximum direct fan-in is two and dependency depth grows
logarithmically. The manifest is counted separately from the requested source
lines; metadata lists every unit's line count, byte count and SHA-256.

Each leaf contains a recursive nominal type through `Box`, a generic Copy
aggregate, a generic Copy function, enum construction and matching, reference
recursion, and ordinary function calls. All generated function bodies contribute
to the executable result. Their bounded arithmetic loops keep values small.
Remainder lines perform arithmetic in the last body: there are no blank or
comment padding lines. The independent Perl evaluator predicts baseline and
edited output, and the lower rungs run both images and compare their output.

This is an explicit package-boundary workload. The historical sampled-shape
ladder remains a separate workload, so its timings are not a baseline for these
rows. The line count gives a reproducible size; the declared DAG, exported
declarations and measured elaboration frontier describe what is actually being
checked. The maximum leaf at 100,000 and 1,000,000 lines has the same size. The
same first leaf function changes its `7i64` bias to `8i64` in both rungs.
An exact SHA-256 guard refuses an already edited or otherwise changed specimen.

`--unit-lines` varies the maximum leaf size while preserving the requested total
line count. After the baseline 100,000-line rung, a separately labelled cohort
with `--rungs 100000 --unit-lines 100000 --skip-real` supplies a fixed-total,
larger-unit control. It has two units rather than twenty; its repeated module
headers and type declarations therefore also differ. Interpret its measured
frontier and memory as a partitioning control, not an identical-IR comparison.
The default ladder always keeps `--unit-lines 10000`.

## Phases and limits

At each rung below ten million, the runner performs:

1. Cold `build-units` with an empty authenticated store.
2. Fresh `check-units` with a separate empty store.
3. Unchanged `build-units` using the first store, requiring identical image bytes.
4. A one-line, one-body, one-file edit and `build-units` using the same store.

The ten-million rung performs exactly one cold-build compiler invocation and
then stops, whether it succeeds or refuses. It is not retried, checked again or
given an edit phase. A marker records that attempt. A refusal stops progression
and records the selected envelope and diagnostic. It does not certify that
later rungs ran. Native output or unchanged-image mismatches stop with an error.

The external experimental envelope starts at 2,048 MiB of virtual address space
and 300 seconds. The next memory envelope is three times the preceding rung's
largest measured process RSS plus 512 MiB, with a 1,536 MiB floor and an 8,192 MiB
ceiling. The next watchdog is thirty times the preceding cold-build wall time
plus 61 seconds, with a 300-second floor and a 3,600-second default ceiling.
Those factors fit the default tenfold rung spacing conservatively; custom
spacing uses the same factors. `--memory-ceiling-mib` can lower the ceiling,
and `--time-ceiling` changes the watchdog ceiling. These are measurement-process
limits, not language or compiler resource policy.

Pool space is checked before generation using the previous rung's actual peak
disk use, the next line ratio and a factor of two, plus 256 MiB. The first rung
uses 128 bytes per line plus 256 MiB. A pool-space refusal is a preparation wall,
not a compiler attempt. Finished source/store/image roots and the private
authentication key are removed by the same experiment; only useful evidence
remains. The optional ordinary compiler specimens use the configured ceiling
rather than an inference from a small generated unit.

## Evidence and interpretation

`measurements.tsv` records wall, user and system seconds; GNU-time peak RSS;
compiler-reported peak RSS; load before and after; the actual external limits;
unit check/reuse/emission counts; unit counts and shape; image and store bytes;
and these maxima from the producer:

- Parsed source function definitions, IR types and IR functions.
- Simultaneously imported interfaces and source paths.
- Dependency declaration-projection bytes.

Missing producer metrics are `NA`, never zero. The maxima count the complete
local elaboration frontier, including imported declarations and the standard
library, rather than assuming that a manifest boundary is also the retained
memory boundary. Unchanged reuse can have no freshly elaborated frontier.

The complete command, compiler diagnostics/stats and time record are kept for
each phase. The copied producer, library inputs, harness and source revision are
fingerprinted; dirty source paths are recorded. Workload metadata permits exact
regeneration without retaining generated trees. Output controls and the numeric
edit record remain beside the measurements.

Compare the 100,000 and 1,000,000 `body-build` rows directly for the requested
twice-cost condition. Compare their cold-build and cold-check RSS together with
maximum frontier counts to judge whether memory is following a unit. The driver
still retains compact manifest, dependency, source-snapshot and linker metadata
for the whole program, and linked output size grows with the program. A flat
largest-unit size alone cannot prove flat memory or flat edit cost.

## Real program units

The runner also checks/builds the compiler through its ordinary source path.
Those rows remain separately labelled and do not claim to partition the compiler.

The `levenshtein-original-units` specimen has three explicit units:

| Unit | Source and adaptation | Dependency |
|---|---|---|
| `upstream` | Exact original 65-line `src/lib.rs`, stored as `upstream.lkj`; no algorithm adaptation | None |
| `corpus` | Existing `corpus.lkj`, unchanged byte for byte | `upstream` |
| `app` | Four-line public Stdout entry calling `crate::corpus::output` | `corpus` |

The helper reads the historical normalized projection, reverses its one range
replacement, and requires the complete library bytes to match SHA-256
`6f295e027783399df21c081465b2ebc2d3a499895635a839c1c00aae4455f267`.
It also authenticates the reconstructed complete upstream test section and
license against the pinned original hashes and receipt. The historical source
and normalization receipt are never changed. This cohort compiles the restored
library; the previous normalized ordinary-path specimen is not relabelled as
unmodified.

Real `check-units` and `build-units` rows use separate empty authenticated stores.
Their source-line column counts all three units; metadata separately records
the 65 original algorithm lines, each adapter's hash, manifest shape and the
exact original upstream identity. This tests the original algorithm across a
real interface boundary, including a final caller whose direct dependency is
only the corpus unit.

After a successful unit build, its native output must match every one of the
committed 7,239 oracle rows. The successful control retains image/output/oracle
hashes, the row count, empty-stderr result, command and timing. Duplicate output
bytes are removed after comparison; a mismatch preserves its output and stops
the runner. The upstream eight-test suite is covered by the separate real-program
gate, rather than counted as executed by this scale harness.

## First measured cohort: 17 November 2026

The clean source revision was `a85410fb9dd37ae76ae7d65cb833dc1050ee4825`.
The producer was SHA-256
`9db99295e4810a4afc3b622b49cf7f38239915cf0d55d0928c34da3ffa44d7f0`
(34,366,481 bytes), the repaired gate's fixed point. One serial compiler worker
ran each phase. The full commands, load, input identities, output controls,
limits, phase stats and time records are in
[the lower-rung measurements](unit-scale-9db99295-20261117-lower/measurements.tsv)
and [the larger-unit control](unit-scale-9db99295-20261117-larger-unit/measurements.tsv).

| Total source lines | Largest unit | Units | Cold build s / KiB RSS | Fresh check s / KiB RSS | Unchanged build s | Body build s / KiB RSS | Body checked / reused |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 10,000 | 9,995 | 2 | 2.79 / 89,956 | 2.50 / 92,492 | 0.43 | 2.11 / 96,016 | 1 / 1 |
| 100,000 | 9,997 | 20 | 23.98 / 111,168 | 21.37 / 102,448 | 1.25 | 3.36 / 93,580 | 1 / 19 |
| 1,000,000 | 9,997 | 200 | 250.15 / 158,252 | 220.97 / 108,396 | 13.63 | 13.35 / 119,424 | 1 / 199 |
| 100,000 control | 99,995 | 2 | 95.40 / 541,820 | 97.31 / 578,396 | 1.12 | 89.56 / 578,888 | 1 / 1 |

The one-million-line build and check both succeeded. Every lower-rung baseline
and edited native output matched the independent evaluator, and every unchanged
build reproduced the identical executable. The largest-unit frontier at both
100,000 and 1,000,000 lines was exactly 2,223 parsed function definitions,
71 IR types, 1,434 IR functions, two imported interfaces, 35 source paths and
76,172 dependency-projection bytes. Fresh-check peak RSS rose only from
102,448 to 108,396 KiB (5.8%) while total source grew tenfold. Native cold-build
RSS also contains global linker and manifest metadata; it rose from 111,168
to 158,252 KiB, so the whole build does not have constant memory.

At the fixed 100,000-line total, grouping almost all code into one 99,995-line
unit instead of 9,997-line leaves raised fresh-check RSS to 578,396 KiB and
its parsed frontier to 7,937 definitions, 75 types and 7,170 IR functions.
This supports a largest-unit checking bound, while retaining the partitioning
qualification above: the two specimens have different repeated headers and
types.

**The body-edit target failed:** 13.35 / 3.36 = 3.97, above the required 2.
Only one unit was checked and emitted in both cases. The one-million-line
body phase spent 0.324 s loading, 1.523 s elaborating and 0.177 s checking;
the corresponding 100,000-line costs were 0.252, 1.443 and 0.185 s.
The growing costs were authenticated store/link work (7.757 vs 0.848 s) and
final source validation/publication (3.089 vs 0.265 s). The 1M phase made
997 authenticated reads totalling 129,877,849 payload bytes, with 1.641 s
inside authenticated reads. Authentication alone therefore does not account
for the wall; repeated input validation, object/interface handling, full-image
relocation and publication remain in the measured path. The resulting 1M
executable was 60,128,475 bytes.

The ten-million-line attempt remains **unspent**. The runner was explicitly
limited to `--rungs 10000,100000,1000000`; it must not advance while the
body-edit criterion is unresolved. Any eventual continuation must retain a
watchdog and memory envelope fitted from these preceding rungs, rather than
using the initial 300-second envelope for a standalone 10M run.

The real-program rows also passed:

| Program and path | Source lines / units | Check s / KiB RSS | Build s / KiB RSS | Executed control |
|---|---:|---:|---:|---|
| Compiler, ordinary source path | Not partitioned | 27.26 / 714,076 | 40.69 / 1,131,720 | Successful ordinary check/build using the fixed-point producer |
| Exact original Levenshtein plus corpus and entry, explicit units | 129 / 3 | 0.86 / 48,324 | 1.00 / 46,060 | All 7,239 oracle rows byte equal; stderr empty |

The original 65-line algorithm passed the pinned upstream hash check without
algorithm changes. Its corpus output and oracle both hashed to
`56b8303689ee15edb00f079b6dfe9d289dc5501e463f5fdf7310ede62f833771`.
The four-line app is an explicitly labelled entry adapter.

## After buffered linking and command-local source validation

The next operation-produced candidate was SHA-256
`672ce5ee1383e0edc115588ac27a865b610eac9ac4b9c244a1d8c03471e89b66`.
It retained authenticated interface/object admission, used one immutable runtime
context, retained verified object code spans without a second copy, buffered
relocations, deduplicated cached source observations, and linked directly into
the unpublished output. Every fresh unit still bound the bytes it actually
loaded; conflicting cross-unit expectations refused. The final exact input and
manifest sweep ran after linking and immediately before executable publication.

[This cohort's complete measurements](unit-scale-672ce5ee-20261117-lower/measurements.tsv)
use the unchanged workload, maximum 10,000-line leaf, and explicit
`--rungs 100000,1000000`. All four original phases ran; no ten-million attempt
was included. The one-million rung used the preceding rung's fitted 1,536 MiB
and 710-second envelope.

| Source lines | Cold build s / KiB RSS | Fresh check s / KiB RSS | Unchanged build s / KiB RSS | Body build s / KiB RSS | Body checked / reused |
|---:|---:|---:|---:|---:|---:|
| 100,000 | 21.66 / 114,888 | 19.80 / 111,588 | 1.07 / 42,880 | 2.62 / 104,292 | 1 / 19 |
| 1,000,000 | 206.15 / 164,712 | 193.59 / 116,080 | 7.48 / 89,560 | 8.91 / 128,484 | 1 / 199 |

All native baseline/edited outputs and unchanged-image comparisons passed.
The exact original Levenshtein units again matched all 7,239 oracle rows:
check 0.79 s / 51,696 KiB; build 0.85 s / 50,276 KiB. The ordinary compiler
rows completed in 20.59 s / 821,168 KiB for check and 37.07 s / 1,312,112 KiB
for build. These remain ordinary-path specimens.

**The edit criterion still fails:** 8.91 / 2.62 = 3.4008; the allowed
one-million-line body-build time for this cohort was 5.24 seconds. The observed
1M edit wall fell from 13.35 to 8.91 seconds, but host load also differed
(each cohort retains its actual per-phase loads;
new 1M body: 4.75 to 4.47). Use the within-cohort ratio for the acceptance
decision, not an isolated attribution of the cross-cohort wall reduction.

The complete checking frontier stayed identical at both rungs: 2,223 source
definitions, 71 IR types, 1,434 IR functions, two imported interfaces,
76,172 projection bytes and 35 source paths. Fresh-check RSS grew 4.0%;
native cold-build RSS grew 43.4%, with compact whole-program link metadata
still retained. The largest generated unit was 9,997 lines in both rungs.

| Measured body-build stage | 100,000 lines, s | 1,000,000 lines, s |
|---|---:|---:|
| Producer identity | 0.291065 | 0.284144 |
| Unit loading | 0.220207 | 0.224338 |
| Elaboration | 1.210961 | 1.248601 |
| Independent checking | 0.149242 | 0.159140 |
| Store admission and per-unit link work | 0.621510 | 5.755873 |
| Final link, source validation and publication | 0.131100 | 1.237956 |

Command-local source validation behaved as intended: the 1M body build performed
200 initial file observations, reused 12,139 observations and independently
revalidated 261 distinct input paths at publication. The 100k counts were
20, 1,159 and 81. Fresh admission never used those earlier observations as its
input identity.

The remaining 1M path still made 997 authenticated reads of 129,877,849 payload
bytes, taking 1.274800 seconds inside those reads, and published a 60,128,475-byte
image. Authenticated read time is part of the 5.756-second store/link stage,
not an additional cost. Remaining repeated interface/projection handling,
native object metadata decoding and global linker tables are the next
implementation targets. Ten million lines remains unspent.
