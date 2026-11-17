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

The runner also checks/builds the compiler through its ordinary source path and,
when available, the provenance-backed levenshtein specimen. These rows remain
separately labelled; they are not evidence of partitioning the compiler itself.
The levenshtein row is labelled `levenshtein-normalized` because its currently
accepted source normalizes the open range and Rust 2015 integration header.
The exact upstream provenance is copied with the row. Revisit that label when
the unmodified upstream form is admitted.
