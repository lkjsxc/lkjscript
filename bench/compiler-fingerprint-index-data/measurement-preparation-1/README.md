# Prospective stored fingerprint comparison

Preparation only. No input has been extracted or decompressed, no store/key created, and no compiler, generator or native image run. The plan is not runnable: producer B is now root-reported, but independent gate acceptance, seed recovery, a concrete reviewed harness, setup proof and an explicit release remain pending.

## Fixed inputs and producers

Reuse the exact two specimens from the closed reference-capture comparison. Its twelve no-store observations are historical; they do not measure this store-enabled fingerprint path and will not enter the new timing table.

All recovery coordinates are pinned to d84cac6edcee59e2c2ad631fb6ca15d63340a1c8 in input-pins.json.

| Input | Exact retained recovery | Population |
| --- | --- | --- |
| Compiler | Git c887b7c5aca7bb80d70ca26d7dfdf1a44213a163; compiler-source.tar SHA 79aa24a9eb6f8e7940b4424fc20839759a79dd8d480b1b746f7e29b17171d806 | 1,742 files, 9,356,858 bytes; src/library/tests only |
| Generated 100k | generated-100k.tar.gz SHA 57ead08d82ea53ba4d833b2689cbf1158550931b8cf288aac3363644d2dab07c | Seed 20261116; 100,000 lines, 169 source files / 3,923,070 bytes, plus the 262-byte workload record |

The generated archive's decoded tar identity and complete 170-member manifest are retained already. No regeneration is proposed. The compiler archive's two PAX pathname records must be handled by the previously reviewed exact-member recovery logic; extraction must verify the complete Git/member sets before release.

A is the existing b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db compiler, 32,588,975 bytes. Its owned golden-capture copy remains a valid source for a future verified copy; recovery also exists at e5294c339d9aefbbafc2e951c997f4fefabcc9ad:seed/lkjscript.gz. Historical recovery recipe B denotes this new cohort's baseline A. Root reports B as a92e81e2d2c18fe1bd645a9b5e287c3a1a2af5eaffa1be16e400289590e7625a, 32,593,071 bytes, from source31f412a0a7df47d897b6b39e794a9b152b4fa29e. The local gate passed once; its independent acceptance, seed recovery and actual CI observation remain separate pending release inputs.

## Thirty-six builds, fixed before observations

Run generated-100k first, then compiler. Each input has six row groups in this exact order:

A1, B1, B2, A2, A3, B3.

Each row uses a distinct initially empty 2 GiB authenticated store and exactly three sequential builds:

1. Cold at the original state S0.
2. Unchanged at S0, using only that row's store.
3. One-body at S1, using that same row's store.

This is 12 stores and 36 builds, with no extra build, warmup, retry, query, index, profile or larger rung. The original input is restored only between row groups. Each input uses one identical source path and output path across both producers. Both use explicit frozen library/runtime selectors, --inline-limit 0 and --stats. Exact command templates and all 36 ordinal rows are in plan.json; the ROLE/ROW placeholders are not executable.

Use one private cohort key outside both source grants and the physical library/runtime roots. Do not use a global key or record key contents. Store contents are never copied between rows or producers. Record store metadata counts/bytes after phases without reading payloads just to gather metrics.

## Exact body changes

Both are single-byte substitutions inside one authored body; lengths, signatures and all other source bytes stay unchanged.

| Input / identity | File | Byte offset, zero based | Change | Complete file SHA before → after |
| --- | --- | ---: | --- | --- |
| generated / edit::target | edit.lkj | 43 | value ^ 1u64 → value ^ 2u64 | b580c07ebeb918056105792f08191869adb72a5d7799afa37fd3aaaa1d4e7a0c → 04e1031f9b7e0c7c933ad815f07879694fd7ec33ebf221fa80673bcedead0722 |
| compiler / driver::inline_limit | src/driver.lkj | 18378 | let mut value = 0usize → 1usize | 333d6ec927416917fa32269730a9a507fcfe31072a2191403a24161b5cf6c412 → 7acceb488b8a47a328ce269d880134eadfa7b92dee788f7b8ac6751e60ea33c3 |

The generated edit is the existing scale/cache-I/O specimen. Its exact 50-byte preimage is a literal in the retained generator source and hashes to the recorded archive member, without decompressing or regenerating it during preparation. Its body is byte range [33,49). The compiler function body is [18240,18817), 577 bytes. Its before/after body text and hashes are retained here, derived from the exact Git blob. The compiler change alters the decimal-parser accumulator in the compiled specimen. Neither frozen executing producer changes; every measurement build explicitly uses inline0, and none of the emitted specimen images is executed.

The initial driver::usage String edit is statically rejected and preserved byte-for-byte under v1-rejected-string-edit. Every String/Bytes payload participates in fingerprint topology, so that proposal could not test the required resume path. No setup or workload ran. The replacement integer has the same width/suffix, lies outside all type nodes and leaves the body/node boundaries unchanged. The include scan finds no embedding of src/driver.lkj. numeric-eligibility.md records the exact fingerprint/checkpoint rules and qualifications; static eligibility does not establish actual resumption.

A future harness must require the exact whole-file preimage before each patch/restore, exact postimage after it, and complete S0/S1 input-member correspondence around builds. The changes are fixed benchmark-input perturbations; no maintained compiler source is edited by this plan.

## Equality and path coverage

Retain every actual output. Each unchanged image must equal its own cold image. All twelve S0 images per input must equal that input's A1-cold image in full bytes; all six S1 images must equal A1-body. S1 must differ from S0. No diagnostic/path normalization or historical-output hash substitution is permitted.

The explicit 2 GiB cap follows the accepted compiler-query-container.sh 100k body protocol. Its two indexed v3 snapshots occupied 1,232,718,650 bytes, and all store files totaled 1,264,032,113 bytes. That historical total is semantic snapshot storage, not the frontend checkpoint alone; this new ordinary-only cohort never requests --index. The cap is therefore a documented conservative choice, with actual capacity/resumption still checked. store-budget.json retains the exact source, interface and cost rationale.

Record and validate actual reuse: cold has no previous row proofs; unchanged must report whole-image reuse; body must report build_reused=0 and positive front_reused. Retain the actual definition/function reuse counts. A body image that does not change, or a path that does not resume the frontend, is a protocol failure rather than grounds to choose another edit.

## Limits, measurements and stopping

Each command has a 4 GiB address-space ceiling, a 900-second watchdog and 10-second TERM grace. Proposed free-space guards are 28 GiB at setup and 2.25 GiB before each build; they do not increase limits or delete prior stores to force later rows. Twelve 2 GiB stores account for 24 GiB of maximum retained payload; the initial guard also reserves 2 GiB for one active staging operation and 2 GiB for images, inputs and records. This is a conservative capacity forecast, not measured usage.

Set LC_ALL=C and TZ=UTC, use /dev/null stdin and unset LKJSCRIPT_STORE_IO. Capture exact argv/cwd/environment, stdout/stderr, raw exit/signal, GNU wall/user/system/RSS, UTC/nanoseconds, all three load averages, image size/hash, store file/directory/byte counts, and every stats field. Retain key_hash_ms as the existing aggregate phase, not as isolated fingerprint time. Keep front_reused and the check/emission/store counters. Missing optional fields remain NA.

Full input checks, patch/restore, output comparisons/retention and store metadata collection are outside compiler timing and separately recorded. These reads can warm OS file pages. Cold describes an empty authenticated store; no disk-cache reset or extra warmup command is performed.

The first setup/preflight/build/capture/coverage/equality failure closes the whole cohort. Keep every completed and failing record, mark remaining fixed rows not-run, and do not resume, repeat, replace the case or change its bounds.

Report all raw rows and, separately for each input and state, the three A/B values, means and medians, wall/CPU ratios, and RSS with load. Three samples and fixed order do not establish statistical significance. There is no imposed 10% adoption rule for this frontend/cache change, no inference about the one-million-line refusal, and no performance result before the cohort runs.

## Review and release boundary

Root has accepted the generated edit and the numeric replacement in principle, subject to the retained include/checkpoint checks and final body review. This deliverable freezes prospective source/archive/selector/edit coordinates; it does not authorize recovery or performance execution. A concrete reviewed harness, recovered-member proof, immutable B source/bytes, completed fixed-point/gate proof, actual CI observation and a separate explicit release are still required. The release template is false with pending fields.
