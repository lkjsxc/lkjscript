# Scalar simplification: predeclared producer experiment

This is a preparation protocol, with no compiler execution or timing result yet. It tests one opt-in, independently rechecked scalar policy. It changes no bootstrap, default CLI, seed or release policy. The prior inline16 miss, inline64 gain and inline128 miss remain separate observations.

## Hypothesis and fixed workload

A single pinned candidate emitter constructs two executables from the identical archived compiler source. A uses `--inline-limit 64`; B uses `--inline-limit 64 --scalar-simplify`. Both constructions disable the store and enable statistics. A is the current producer policy, and B adds only the candidate scalar stage. Neither construction is part of the runtime mean.

| Input | Exact identity |
| --- | --- |
| Frozen revision | `26bf2c3c021dd79810a81fa1d9054d59f3768b06` |
| `src` tree | `3431671024d0a550b3130f2401bde067026d9bd3` |
| `library` tree | `e442fb3920db7eda2377f92fde0d92761f0562be` |
| Physical runtime SHA-256 | `eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14` |
| Expected output SHA-256 | `8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132` |
| Expected output bytes | 16,228,028 |

The source argument is `src/main.lkj`, with the same explicit absolute source root, physical library root and physical `library/runtime.lkj` selector for every construction and execution. The archive includes only `src`, `library` and the frozen `seed/lkjscript.gz`; the seed is decompressed as the exact output oracle. Source/runtime/expected/emitter hashes are checked before the two constructions, and source plus emitter identity is checked again after all six executions. The retained paths and trees identify this cohort. Paths can affect diagnostics and image footprint, so old construction-image identities are not substituted for these artifacts.

## Admission before measurement

The candidate must first pass the focused scalar CLI/cache and independent IR tests and the existing complete local gate. A later, separately reviewed producer pin records exact emitter SHA-256, byte count, source commit and source/library tree IDs, plus the passed focused and full-gate evidence for those exact producer bytes/source. The immutable harness and this protocol are hashed in that pin. The example pin is intentionally incomplete and cannot launch a campaign.

**Candidate CI is pending until/if the candidate qualifies for main.** Passing CI for an older baseline does not validate the new scalar source. This protocol does not assert candidate CI success. Root's explicit later release is still required; authoring this harness is not an execution release.

The independent IR tests own algebra, signed widths, consumption and aliasing, effect/control invalidation, trap code/message/location preservation, and the last-admitted-selection fallback after rejection of a second candidate. The small CLI suite checks the integration boundary. No production rejection hook is added. This does not claim that interpretation runs the entire compiler witness.

## Fixed commands and order

The candidate emitter is copied once into the retained fixture and verified against the pin before use. The only compiler workload phases in the harness are these eight, sequentially:

| Ordinal | Phase | Executable | Output policy |
| ---: | --- | --- | --- |
| 1 | Construct A | Pinned candidate emitter | inline64, scalar disabled, no store, stats |
| 2 | Construct B | The same copied emitter | inline64 plus scalar, no store, stats |
| 3 | A1 | Constructed A | inline0, no scalar flag, no store |
| 4 | B1 | Constructed B | inline0, no scalar flag, no store |
| 5 | B2 | Constructed B | inline0, no scalar flag, no store |
| 6 | A2 | Constructed A | inline0, no scalar flag, no store |
| 7 | A3 | Constructed A | inline0, no scalar flag, no store |
| 8 | B3 | Constructed B | inline0, no scalar flag, no store |

The six measured executables run the frozen old compiler logic, whose CLI does not need to know the new scalar option. The scalar option is used only by the new emitter while constructing B. Each runtime command builds the same frozen source with output `--inline-limit 0 --no-store`, without `--stats` or profiling. Every output is compared byte for byte with the frozen seed, and its full digest, byte count, stdout and stderr are retained. Unique output filenames preserve all six artifacts. There are no warmups, extra runtime controls, adaptive retries, threshold searches, cache resets or profiling phases.

Construction controls require accepted inlining for both A and B, zero inline rejections, all nine scalar statistics present as integers, no scalar work for A, and a nonempty accepted scalar selection for B. B must have positive attempts, accepted count and changed-function count; the sum of constants/identities/branches must be positive. Scalar candidate rejection or a message-inventory mismatch refuses this campaign. Trap-guard rejections remain reported and do not alone refuse it: they count unsafe folds that were deliberately left unchanged. A and B must have distinct image bytes. Construction stdout/stderr, phase statistics, time, RSS, image sizes and hashes remain separate from runtime results.

## Resource and capture contract

One compute worker runs each phase under `/usr/bin/time`, `prlimit --as=4294967296` and `timeout --signal=TERM --kill-after=10s 900s`. These are external experiment limits, not language policy. The initial preflight requires 3 GiB of free space on the fixture filesystem. All limits and the order are fixed; refusal does not increase a limit or select a replacement source.

Each phase retains exact NUL-delimited payload and wrapper argv, cwd, status, full stdout/stderr, GNU elapsed/user/system/peak-RSS fields, UTC start/end, nanosecond start/end and all three load averages immediately before/after the command. The primary wall is GNU time's elapsed value and includes its `prlimit`/`timeout` child launch, as in the accepted inline64 campaign. The separate nanosecond capture also includes the start/checkpoint writes; it is not silently substituted into the primary rule. Image hashing and equality checks occur after each timed phase. Source/artifact hashing is outside the phase timing.

The outer root launcher must retain its invocation, cwd, status and stdout/stderr, including errors before a results directory can be created. The harness refuses an existing results path. Once it creates the directory, its exit record preserves the last reached phase even on setup failure. A command failure, watchdog, signal, allocation refusal, malformed timing/statistics, changed input, wrong output or unexpected runtime diagnostic ends the cohort immediately; the row and partial artifacts stay, later phases are not launched, and no summary is promoted to a completed comparison. It never retries a phase. External interruption can still prevent a final exit record; the per-phase start checkpoint and available captures remain the evidence in that case.

No process, FD, syscall or other concurrent observer is added. The harness has no cleanup mode, process scan, background worker, kill campaign, private key or store. It leaves the copied emitter, frozen source, expected image, two producers, six outputs and raw evidence for independent review. Cleanup, if later authorized, is a separate ownership action.

## Decision and limits

After all eight phases and six exact output controls succeed, the primary rule uses every GNU-time runtime wall value:

`mean(B) / mean(A) <= 0.90`, equivalently `10 * sum(B) <= 9 * sum(A)`.

Decimal seconds are converted exactly to integer nanoseconds for the threshold. The reported medians are secondary and cannot reverse the primary verdict. CPU, RSS, construction costs and image growth are separate observations. There is no outlier deletion or selection of the fastest pair. Each row's paired load window remains visible, as does the fixed A/B order and unreset file-cache state.

A miss discards the scalar optimization for this producer policy and retains the evidence. A qualifying result is reported to root before any policy or seed change, followed by separately authorized integration, gate and CI work. The harness makes no automatic adoption. This one fixed-order cohort does not establish a randomized causal estimate, a backend floor, a query-hashing improvement or a new Rust `-O` milestone; any latter ratio needs its own comparable witness evidence.

## Pin schema and intended integration

The proposal files are intended for `tools/scalar-simplify-test.sh`, `bench/compiler_scalar_simplify.sh` and `bench/compiler-scalar-simplify-plan.md`. Root owns the serial-worker gate hook adjacent to the existing optimizer test. `producer-pin.example.json` documents the later pin: `schema`, `harness_sha256`, `protocol_path`, `protocol_sha256`, an `emitter` object, `validation.focused`, `validation.full_gate`, `candidate_ci` and the exact `execution_order`. Each validation entry records `status=passed`, the same source commit and producer digest, and a retained evidence path/hash. These fields record actual provenance; they must not be filled with invented success.

The intended command after a separate execution release is:

```sh
bash bench/compiler_scalar_simplify.sh \
  --pin /absolute/path/to/the-reviewed-producer-pin.json \
  --results target/compiler-scalar-simplify.COHORT
```

The focused script performs 21 top-level captured invocations on one tiny source (including a standalone profile image and one jobs1 expectation test). It checks both conflict orders; original IR, authority and interpreter value; a nonempty scalar-only admitted rewrite at inline0; default-off/on whole-image separation; cold/warm profiled fragment and exact companion equality; the profile header's selected-IR digest and official independent companion check; and worker flag propagation. It does not duplicate the optimizer mutation suite, broad trap matrix or rejection-injection tests. Its private key is outside all source/library grants. It deletes that disposable key on exit, removes its owned fixture after success, and retains failed fixture/command logs for diagnosis.
