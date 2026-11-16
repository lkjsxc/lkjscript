# Prospective scalar trial release: PENDING

This is a small preparation proposal. The candidate full cold gate rootv1382 is still pending; all four compiler workers remain reserved. No workload, compression, deletion or producer-policy change is authorized by these files. The existing harness and protocol remain immutable.

## Input review

The installed harness is SHA-256 092f0187705dc5136284d2cb9deed82d6c35664138e3c631a9887c01c280aad3; the installed plan is 5e7c5b605250bb3cc5e5153f1f2fac9a95a686cae5495bebfe5c6ee5798716d4. Both were read in full again. The pin schema matches the actual validator.

Candidate source 5b5ce7759c035ae0437e3512e18efc440aa50d2e has source tree 6277f3cec86791e7f297a95ff0a70bed60b951eb and library tree 25bdb8ff8567b941790d3951c78e896b23e26e8e. Its actual producer f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64 is 32,761,734 bytes and passed the three focused commands. The exact focused summary and 65-record manifest are bound by the draft pin. This supplies focused integration coverage; the full gate must additionally supply the independently checked IR, conformance, generated and mutation routes.

The draft full_gate status is the literal PENDING and its evidence path/hash are null. The frozen harness requires status exactly passed and will reject this draft before any compiler phase. Do not run that rejection as a test: preparation would still create the results directory. Root must issue a separate final pin after actual gate success, preserving this pending draft.

Main 7716f0d51ead33d17f5a85c0bb8e4f79d57ccf23 and baseline CI 43904619997 are separate context. That CI was pending when root assigned this preparation and cannot validate the experimental source. Candidate CI remains pending until/if main adoption, exactly as the frozen protocol states.

## One-cohort launch checklist

All checks are prospective; none is marked complete by this checklist.

1. Require the completed candidate full-gate result and raw gate log. Check its source commit, source/library trees, exact final producer SHA/size and all relevant route outcomes. Do not convert a pending or failed result to passed. If a gate repair changes source or producer, obtain new matching validation and a separately reviewed final pin.
2. Recheck the focused summary and its complete manifest. Bind the passed focused and full-gate evidence in the final pin, hash that pin and preserve the explicit root execution release separately. A reviewer must not treat the pin's extra execution_authorized field as enforcement: the actual frozen validator enforces the passed validation fields, and root release remains a process requirement.
3. Retain the experimental source recovery bundle described below before any later source/ref cleanup. This is separate from the eight timed compiler phases. Bundle creation/verification remains unexecuted here and must wait until the full-gate resource reservation ends.
4. Confirm the existing harness/plan hashes without editing either. Use one compiler worker, external 4 GiB/900 s/10 s limits, at least 3 GiB free on target, and a previously absent results directory. Proposed one-cohort name: target/compiler-scalar-simplify.20261116-1. Do not create it during preparation.
5. Capture the exact outer launcher argv, cwd, stdout, stderr and status in a separate owned capture path before invoking the frozen harness once. The intended command is bash bench/compiler_scalar_simplify.sh --pin ABSOLUTE_FINAL_PIN --results target/compiler-scalar-simplify.20261116-1. Root must bind the final pin path and this invocation in the actual release.
6. Preserve A then B construction and exactly A1 B1 B2 A2 A3 B3 execution. A is inline64; B is inline64 plus scalar. All six output builds remain inline0, no scalar flag, no store and no profiling, using the same frozen source/library/runtime paths. No warmup, extra control, adaptive retry or threshold search is introduced.
7. Stop on the first failed phase/control and retain every row/partial artifact. Do not resume, retry, replace a phase or change a resource limit. If transport disconnects, recover the same gateway request/session rather than launching again. A process observation or cleanup scan is not part of this protocol.
8. After complete success, require all six output byte comparisons and exact digest/size, unchanged source/emitter/producer controls, and well-formed eight rows. Recompute the primary rule from all three GNU wall values per policy: mean(B)/mean(A) <= 0.90, exactly 10*sum(B) <= 9*sum(A). Median is secondary; no outlier removal.
9. Retain all artifacts for independent review. Report a threshold result before any adoption. A qualifying observation does not automatically change bootstrap, seed, default CLI policy or candidate CI status. A miss remains a retained experimental result.

## Exact source recovery requirement

The worktree is detached at the experimental commit. That commit's sole parent and common ancestor with main 7716f0d5 is e0b21e66659dc1830a5bebddf0c1f30fd2d8fa94. Frozen workload 26bf2c3c021dd79810a81fa1d9054d59f3768b06 is already an ancestor of the candidate and the published parent.

A small incremental Git bundle may preserve only the experimental commit/object delta, using a temporary compiler-prefixed ref that points exactly to 5b5ce7759c035ae0437e3512e18efc440aa50d2e and excludes parent e0b21e66659dc1830a5bebddf0c1f30fd2d8fa94. Record the bundle's exact SHA/bytes, advertised ref, prerequisite and verification output. This is not a standalone repository: recovery requires the exact published parent objects. A recipient must verify that prerequisite, import the bundle into a recovery repository, and verify the candidate commit plus both tree IDs. Preserve the prerequisite identity and a source-backed recovery record; do not claim a hash alone reconstructs source.

Prospective commands, not executed by this proposal:

    git update-ref refs/heads/compiler-scalar-experiment-5b5ce775 5b5ce7759c035ae0437e3512e18efc440aa50d2e
    git bundle create OWNED/compiler-scalar-source-5b5ce775.bundle refs/heads/compiler-scalar-experiment-5b5ce775 ^e0b21e66659dc1830a5bebddf0c1f30fd2d8fa94
    git bundle verify OWNED/compiler-scalar-source-5b5ce775.bundle
    git bundle list-heads OWNED/compiler-scalar-source-5b5ce775.bundle

The final source record should identify that this bundle includes the tracked experimental source/tests/plan, not untracked focused results, the live candidate executable or future performance artifacts. Keep the ref until the retained bundle/recovery proof is accepted; deleting refs is not authorized here.

The candidate commit's seed/lkjscript.gz is Git blob bdd99cabbb0a2154257c9a536eb002be8fa32a1f. It has not been decompressed in this review and must not be described as containing producer f85624e3. Preserve actual emitter bytes separately. Frozen workload seed/lkjscript.gz is blob af5341ff14f1e978bc50e2d3209146664f04203d; the harness independently decompresses and checks its expected output at launch.

## Performance artifact retention and recovery

The harness leaves everything in the declared results scope; no cleanup mode exists. Preserve the following until independent acceptance, including failures:

| Evidence/artifact | Retention requirement |
|---|---|
| Final pin, protocol copy, focused/full-gate evidence, outer launcher capture | Retain exact bytes, hashes and source/producer relationship. These are not reconstructed from a success summary. |
| raw directory, rows.tsv, parity.tsv, summary.json, metadata, checkpoint, outcome, endpoints/load | Retain every record, including empty stderr, failed rows and any missing-completion condition. Construction and runtime costs remain separate. |
| fixture/emitter | Retain the actual f85624e3 bytes or an explicitly verified later archive. The source bundle and an old seed do not substitute for this artifact. |
| fixture/source.tar, source manifests, input-artifacts.sha256, source/seed/lkjscript.gz | Retain archive identity and exact source/runtime selection evidence. The extracted source is reconstructible only after the archive/Git controls are accepted; no deletion is authorized now. |
| fixture/producer-A and fixture/producer-B | Retain both actual constructed executables plus exact construction argv/stats/times. Path-sensitive construction bytes cannot be claimed recoverable from source hashes alone. |
| fixture/expected and fixture/output-A1, output-B1, output-B2, output-A2, output-A3, output-B3 | Retain all seven files through raw byte-comparison acceptance. Every observed output must equal SHA-256 8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132 and 16,228,028 bytes. Only after acceptance could root choose deduplicated retention/recovery from the exact frozen seed; all six observation/parity records still remain. |
| final-input-check.*, final-artifacts.sha256, constructed-producers.sha256 | Retain exact final unchanged-input/artifact checks, not merely their reported pass. |

The cohort uses no private key or store. No key retention issue is introduced. No source bundle, image archive, performance fixture or compression has been created by this preparation; the final release must record their actual identities when they exist.

## Static outcome

No new concrete harness defect was found. The draft is intentionally non-runnable, the primary mean estimator is unchanged, and the historical inline16/64/128 observations remain separate. The source/producer linkage and pending full-gate/CI limits are explicit. This is not a measured speedup or a claim that the candidate passed its full gate.
