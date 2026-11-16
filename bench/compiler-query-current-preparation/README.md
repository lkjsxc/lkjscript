# Current query harness preparation

This is an executable but **unexecuted** implementation of
`bench/compiler-query-current-plan.md`. No compiler, query, index, source archive,
store, or private key was created during preparation.

The prepared files are:

- `bench/compiler-query-current.sh`: fixed one-worker, two-build/50-request
  orchestration with retained fixtures and raw outputs.
- `bench/compiler-query-current-validate.pl`: offline identity, build, protocol,
  bounds, coverage, timing, and counter validation.
- `bench/compiler-query-current-questions.tsv`: exact historical 40+10 order.
- `bench/compiler-query-current-identities.example.json`: deliberately invalid
  placeholder until the combined gate supplies the actual canonical pins.

The root reviewed the shell harness, validator, question population and updated
path/timing protocol before this documentary commit. Syntax-only checks
(`bash -n`, `perl -c`) pass, and the static question audit reconstructs all
50 rows from the two historical scripts. These checks are not runtime validation.

The original absolute-source-directory suggestion was corrected **before any
launch** after reading `driver::diagnostic_source_path`,
`elaborate::source_paths`, and `native::diagnostics::data`: a source-directory
option prefixes embedded diagnostic paths. To preserve the canonical image, the
harness uses cwd exactly the owned archived source root and relative
`src/main.lkj`, `--library-dir library`,
`--runtime-source library/runtime.lkj`, with no `--source-dir`. The source
grant is exactly `.`, with store/output/bin as sibling directories and the key
outside all read grants. Both indexed inline64 images must directly compare
byte-identical to the gated canonical executable before the questions run.

The timer begins after command/cwd/load/checkpoint metadata is written.
Subprocess/resource launch and stdout/stderr capture are included; later
validation/hash work is excluded. There are no metadata prequeries, map walks,
old-reader requests, pin repair, substitutions, or retries. Refused queries stay
in the fixed sequence; validation happens only after all timed requests.

The final exit status certifies completion of the captured protocol checks, not
universal query completeness or the latency goal. The summary keeps protocol-valid
under-100-ms counts separate from empty, unavailable, partial-recorded, and
nonempty-recorded coverage. A fast empty or partial answer does not establish the
note's complete question contract. Current source/producer and output inline64
differ from the historical v2 cohort, so no causal latency-speed comparison is
authorized.

Actual identities, factory argv/cwd and gate evidence must be committed in a
separate post-gate addendum. The root's explicit release remains required before
running the harness. No preflight compiler or query may be used to tune the
population or avoid recording a failure.
