# Conditional reference capture: prospective cohort 2

**Preparation only. No comparison has run.** This is the separately proposed cohort 2. Its twelve commands are fixed before observations. A release.json is deliberately absent; release.pending.json is non-executable. Root must bind accepted setup/input recovery, candidate gate evidence and the actual CI observation in a separately committed release, then explicitly authorize one launch.

## Boundary from the closed first cohort

Cohort 1 stopped during input recovery because Archive::Tar 2.40 did not apply two per-file PAX paths in the pinned Git archive. It executed no generator/compiler/native command and no A/B build. All twelve original comparison rows are terminal not-run.

The recorded setup2 amendment repeats input recovery after repairing that preparation reader, with identical source, generator, seed, shape, population, order and limits. It does not erase the original failure or select/repeat a measured sample. Corrected setup2 uses measurement-2 and the new comparison uses a separate comparison-2 root. No subsequent automatic retry, larger envelope, alternate workload or resumed cohort is permitted.

The original prospective design is plan.md SHA-256 249af04e480480490f059b89af48b7469730c8e98c49c5627d21514ee11b9f80. The correction is separately released in 5e1b82f3cf23654e04d0904e948c202c96723c0d, setup control manifest fe8321b36a88864b33dd3c8bda9a71ab954f9dae14926d3ba9e45fbb564af847. The setup’s five phases and independent acceptance have completed. Exact inputs and acceptance are retained at1404b4af81b2afa979993b0c426921321a6801f0; the separate comparison release must bind those records.

## Immutable inputs and producers

All input reads use /home/coder/workspace/lkjscript/target/compiler-reference-capture-measurement-2. The harness never writes there. All new files go under the initially absent sibling /home/coder/workspace/lkjscript/target/compiler-reference-capture-comparison-2.

| Input | Exact source and entry | Frozen recovery |
| --- | --- | --- |
| new-100k | generated-100k/main.lkj;169 source files,100000 lines,3923070 source bytes;170 regular members including workload.txt | generated-100k.tar.gz SHA-256 57ead08d82ea53ba4d833b2689cbf1158550931b8cf288aac3363644d2dab07c,405428 B; member JSON cfc023b8d4224c66853c1545b6cdca391203f32c94654d7d9c51c77f6b694cb7 |
| compiler | compiler-input/src/main.lkj;1742 source/library/tests members from c887b7c5aca7bb80d70ca26d7dfdf1a44213a163 | compiler-source.tar SHA-256 79aa24a9eb6f8e7940b4424fc20839759a79dd8d480b1b746f7e29b17171d806,10741760 B; member JSON 2e2e0ec2121c90767780b063cfba0cbe048f9d39914a3c0294e4e42641dcde4c |

Both inputs use the same explicit physical compiler-input/library directory and compiler-input/library/runtime.lkj, runtime SHA-256 eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14. Source selectors and output pathname are identical across all six builds of an input.

A is the exact baseline 6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc,32584542 B, source d461c2d98e93aeed6061508ee80ab36b9eeaa04f, source tree c710586b4d1b7ab476fd179a7d2c699477a2054a. Its independently recovered setup copy is verified and copied to comparison-2/producer-A.

B is b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db,32588975 B, source bd8a9050a3786de9f227c6c4037f5acd43205f71, source tree 626c9c1f234f4d003a08820d9b69840535eff8b8. It is recovered directly from e5294c339d9aefbbafc2e951c997f4fefabcc9ad:seed/lkjscript.gz, compressed SHA-256 c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae/6551428 B. Both compressed and decoded identities are checked before the owned producer-B is created. No mutable target/lkjscript is executed and no producer is rebuilt.

Both producers were constructed with the established producer inline64 policy. Both have library tree 25bdb8ff8567b941790d3951c78e896b23e26e8e. Measured outputs always request inline0; no scalar-simplify or additional pass is enabled.

B’s bootstrap fixed point, focused query checks and full cold gate passed; controls.json records the exact source/image and capture hashes available during preparation. Candidate CI 43925403668 was in_progress at preparation, with its initial observation retained. This is not a CI success claim and baseline CI does not validate B. The separately pinned release must carry a retained current observation for this exact candidate run and the accepted gate evidence.

## Fixed command population

Run new-100k first, then compiler. Each uses exactly A1,B1,B2,A2,A3,B3. The twelve complete argv/cwd/output/reference records are frozen in order.json. There are no warmups, profiles, extra seeds, retries, 1M commands, adaptive branches, store/index/edit operations, performance observers, cache resets or cleanup mode.

The command template is:

    PRODUCER build ENTRY --source-dir INPUT
      --library-dir SETUP/compiler-input/library
      --runtime-source SETUP/compiler-input/library/runtime.lkj
      --no-store --inline-limit 0 --stats -o FIXED_OUTPUT

Cwd is that input’s exact source root and stdin is /dev/null. Fixed outputs are comparison-2/outputs/new-100k.image and comparison-2/outputs/compiler.image. Completed slots are copied to distinct images/NN-input-label.image before another build can reuse the fixed pathname. Each input’s first successful A1 establishes its output reference. Every later image is compared using full bytes, as well as recording its size/digest.

The source/runtime/output recipe differs from a canonical seed build. A’s executable digest and the old 26bf compiler-output digest are not output references for this experiment. Equal images prove agreement for these two computations; they do not establish a bootstrap fixed point or generated-code speed milestone.

## Preflight and capture

The future run requires exact release and control-manifest hashes. Its root must be absent. Preflight verifies all 25 recorded setup artifacts/metadata, exact input file/directory/mode inventories and runtime, both producers/source trees, pinned tools, and the committed acceptance/recovery records named by the release. All writes are in comparison-2. The fixed inventories are checked again after all twelve builds. Mutable main source and HEAD are not a workload selector.

The preflight necessarily reads/hashes input and producer bytes, so file caches may be warm. This is a no-store comparison, not a cold-file-cache experiment. No source or producer identity scan is added between measured builds. Per-row output retention/validation reads output bytes outside the primary interval; that fixed protocol and alternating order do not eliminate scheduling/cache effects.

One worker. Each compiler command is wrapped by GNU time, timeout 900 s with 10 s kill grace, and prlimit address space 4294967296 B. Require 3221225472 free bytes before each captured command. The same limits also bound preflight, per-row validation and postflight metadata helpers; those timings are explicitly not compiler observations.

Each measured record retains exact argv, wrapper argv, declared/actual cwd, stdin, free space, UTC/nanosecond start/end, three load averages before/after, stdout/stderr, original status and GNU wall/user/system/RSS/exit. Row validation requires complete matching timing/cwd/argv, exactly one completed compiler stats object, zero declared reuse/inline-attempt fields, no store activity, ELF output, matching emitted byte count, and full-byte equality to A1. Missing optional store fields are not treated as measured zeros; no-store’s required reuse fields must be present and zero.

The primary wall is GNU time around timeout/prlimit/compiler. Hashing, copying, JSON validation and archival work occur outside it. Outer launch/capture timestamps remain separately recorded. Actual producing-compiler authentication performed inside an ordinary compiler invocation remains in its measured cost.

A failed command’s post-command fixed slot is retained where available, but it may still contain an earlier successful image; its freshness is explicitly not established. If validation cannot run, the fixed slot and prior retained phase images remain untouched for audit. No failure output is counted as a successful equality proof.

## Stop and interpretation

The first nonzero command, resource refusal, pin mismatch, detected incomplete/invalid capture, invalid statistics or output mismatch stops the entire cohort. Metadata needed to retain the failed slot may finish; no later compiler build starts. Original command status is preserved even if its metadata validator also fails. The EXIT trap writes all twelve rows as passed,failed,preflight-refusal,incomplete or not-run, with overall status and stopping phase separately retained. An interruption is incomplete, never permission to continue.

All observations and outliers remain. Only a complete cohort with all identity/output controls may publish aggregate comparisons. For each input separately, summary.json reports each sample, arithmetic mean B/A ratios for GNU wall, user+system CPU and peak RSS, and medians/ranges. There is no 10% adoption threshold for this frontend work-avoidance change. Do not pool inputs, claim statistical significance, attribute every RSS byte to reference vectors, derive exponents, or diagnose the earlier 1M R0008.

The measured treatment is the exact reviewed A versus B executable. Source recording avoidance is the intended difference; whole-build timings still include all ordinary work and may reflect host scheduling, cache state and compiler image differences. Peak RSS is a process maximum, not an allocation census. Correctness of full reference capture and checkpoint promotion belongs to the separately retained tests.

## Release and retention

release.pending.json is deliberately false/pending and cannot authorize a run. Root must make a separate release.json that binds this control manifest/order, exact B/source/setup summary, durable setup acceptance/input recovery/gate acceptance/CI observation records, and an explicit once-only release. Its hash is the sole launch argument:

    bash run.sh --execute-after-root-release EXACT_RELEASE_JSON_SHA256

No such invocation is authorized by preparation. The root’s outer capture must preserve refusals that occur before comparison-2 can be created.

After a run, retain every raw capture, both exact producers and B’s original seed gzip, all phase images and fixed slots, input before/after inventories, setup/source recovery records and release/control files until independent acceptance. No automatic compression or deletion runs during the cohort. Source archives and producer seeds remain recoverable through their explicitly committed pins; a digest alone is not a recovery artifact.
