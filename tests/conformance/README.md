# Conformance

`node tools/conformance.mjs target/release/lkjscript` runs source/IR case data.
`LKJSCRIPT_ENGINES=native,interpreter` shares source checking and runs both independent execution
routes. The routine gate uses those routes, Rust reference comparisons, and
seeded value/verdict/output/destruction/sugar/closure generators.

`cases.json` declares expected values, output bytes, trap/diagnostic codes and
optional trap lines. Source values expose a public `probe`; capability cases have
a World-taking `main`. Direct IR fixtures reach the independent safety gate.
The Rust oracle checks comparable source behavior with overflow checks enabled.
Pending source cases count as pending only for E_UNSUPPORTED/E_PARSE; a wrong
result, unrelated diagnostic or incomplete process result fails the suite.

Cases are regression evidence, not a score for language width. New features follow
real compiler/tool consumers and fresh outside programs. Migrated backend cases
preserve arithmetic edges, storage/copy/drop behavior and UTF-8 boundaries after
removal of the WebAssembly host. Engine layout assertions disappeared with that
engine; ordinary allocation/refusal checks use the native boundary fixtures.

Each case receives fresh execution state. The shared watchdog detects stalled
processes; LKJSCRIPT_GATE_JOBS selects the number of parallel workers. Seeds,
expectations and reduced failures stay in Git; working artifacts and cached Rust
comparisons stay under target/.

`node tools/backend.mjs` compares the cargo-built independent backend with B0 over
this data, checking verdicts and executable bytes.

World output records may also declare `stderrHex` (default empty) and normal
`status` (default zero). Rust World runs use a dedicated control pipe; neither
program stderr nor an intentional status 101 can impersonate a trap. Selectors
are harness environment metadata, so World sees only application arguments.
The separate `tools/world-io.mjs` suite exercises declared binary input/arguments,
filesystem state and process descriptor conditions that do not fit a batch row.

For an explicit panic, expected code `R0004` includes the artifact-local message
codes `R0016` and above. Their slot numbers can move when unrelated library
functions change. Drop-order cases also assert the source line of the first
panic, so a different destructor cannot satisfy the observation.
