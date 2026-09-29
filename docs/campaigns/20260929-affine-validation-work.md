# Affine validation work admission

Date: 2026-09-29 (Asia/Tokyo).

## Request and starting point

The user explicitly selected `https://github.com/lkjsxc/lkjscript`, delegated current
engineering judgments, permitted major changes, and requested implementation with a
very-long-term view. This is the language repository, not the Nostr client `lkjstr`.

Local main and the independently fetched GitHub main both selected
`7aaf924f1dde7f91578b279e32b7aef8624b43d9`. Work uses
`lkjsxc/tomato-ocelot-73`, `/home/coder/workspace/lkjscript`, with the pinned Rust
1.98.0 toolchain. Existing untracked campaign/CLI files, the history-prototype stash,
other worktrees and running services are outside this change. No binary publication,
production deployment, access-control change or history rewrite is selected.

## Selected boundary

The next ownership/type increments depend on finite, truthful validation. Inspection
found a concrete gap in the existing affine validator rather than a need for another
ownership representation: only expression visits consumed its work admission. Root,
type, signature and imported metadata reads did not. Repeated paths through an ordinary
type graph could consequently expand without consuming this budget.

The old expression counter incremented before testing its limit and used saturating
addition. Zero admission was exceeded before rejection, and a saturated maximum counter
could no longer signal exhaustion. Only body evaluation converted the internal work
error into the separate `Steps` outcome. The full-snapshot affine wrapper discarded that
outcome entirely. These are phase-local findings: later full-validation phases may
independently reject, so this record does not claim that an invalid program was admitted
end-to-end or that the runtime ownership rules were bypassed.

The selected correction places all affine metadata reads behind one request-local
metered `ExpressionRead` wrapper and shares its counter with expression visits. Each
unit is admitted before delegation; exhausted work neither reads the next record nor
overflows the counter. Every phase propagates work exhaustion independently of the
semantic-diagnostic sink. The whole-snapshot wrapper emits the existing resource
exhaustion diagnostic instead of silently dropping the result. Existing incremental
and extraction callers retain their own budget-error mapping.

This changes proof accounting, not accepted ownership/effect meaning, canonical graph
or package/artifact encoding, runtime instruction quotas, resource cleanup, or deployment
grants. It does not add region borrowing, a collector, or a linear-time type traversal.
The reader preserves underlying cancellation and read errors; the budget is not a claim
of a universal wall-time, stack, memory, or hostile-code-containment bound.

## Predecessor failure sensitivity

Six new focused tests were compiled with the original product implementation, changing
only test registration. The complete foreground run failed all six tests (zero passed,
zero ignored, 923 unrelated tests filtered out). Cargo reported the exact executable
`target/debug/deps/lkjscript-d2d332ca4c5ae98a`. Its retained log is
`/tmp/lkjscript-affine-budget-before-complete.log`; compile time was 6.44 seconds and
the test process returned 101.

Independent expected counts enumerate one minimal task's root, result type, shape
signature, initial-state signature, expression visit and expression record. Other cases
exercise repeated type paths, shared counters, integer ceilings, separate diagnostic
admission, whole-snapshot propagation and stopping before another root. The original
implementation reported zero charged units for 64 metadata reads, incremented beyond a
zero limit, conflated the requested work/diagnostic boundaries, and completed a selected
root sequence despite the new read-work limit. Assertions that failed early do not claim
that subsequent assertions in the same predecessor case were executed.

An earlier background attempt retained compilation output but no terminal test result;
it is not test evidence. The subsequent complete foreground run above is the predecessor
observation. No unrelated work was reverted: only this campaign's product edit was restored
to the exact starting file while obtaining that result.

## Current proof identity and focused checks

Affine validator feature 9 supersedes feature 8 because the previous proof admission could
omit metadata work or lose an exhaustion outcome. The contract is now
`validator_contract_acb29f7085474c4d7be4f9b74b21c13a52da78973379612f79f56552064938c7`.
This invalidates predecessor proof reuse, not the graph encoding. The contract test retains
an explicit inequality against the feature-8 identity and pins the complete new identity.
The initial run against the old golden failed as expected; its original output remains at
`/tmp/lkjscript-affine-budget-contract.log` rather than being relabeled a success.

After the initial implementation, the original six cases passed. The expanded nine-case
suite also passed (zero failed/ignored): `/tmp/lkjscript-affine-budget-expanded.log`,
`TEST_EXIT=0`. It adds all four metadata reader methods, cancellation at each of five read
phases, and a sweep of every incomplete incremental-change budget through exact acceptance.
The sweep uses a valid ordinary change, an empty semantic-diagnostic sink and an unchanged
base; it must observe affine-specific resource exhaustion rather than semantic rejection.
Explicit checkpoints retain cancellation without becoming extra read-work units. The
integer-ceiling case admits the last representable unit and rejects before the next read.

The broader `cargo test --locked --lib affine` selection then passed all 22 cases in
2.73 seconds, including the independent private/package/generic/effect/resource flow oracles
and maintained extraction/CLI observers. Its log is
`/tmp/lkjscript-affine-budget-focused.log`. The corrected exact-contract test passed separately:
`/tmp/lkjscript-affine-budget-contract-corrected.log`. Both retain `TEST_EXIT=0`.
These focused observations are not a full-profile receipt or binary-release acceptance.

## Acceptance

Executable and test source was frozen at
`6aa625822ec27282c1735c2c47f73271b39dc6f5`, tree
`2db5ebf5c6d0ecebbcc3ea15f2b3948acd97f7f6`. The pinned Rust contributor checker was
rebuilt before running:

```text
target/release/lkjscript-dev check full --fresh --jobs 2 --machine
```

The full profile passed all 26 selected gates, with 26 fresh executions and zero reused
results, in 1027.718580926 seconds. The terminal command result was `CHECK_EXIT=0` in
`/tmp/lkjscript-affine-budget-full.log`. Its authoritative receipt is
`.artifacts/lkjscript-dev/check/1790687675535381354-2379470-0/receipt.json` (64,531 bytes),
with identity
`verification_d47cff799782d199c6327cd8e4f3cee7a7df56f285d00937c6f2c8feaa802064`.
The receipt records the exact source commit above and `input_stable=true`; both input
identities are
`verification_01de8221854c7728ef1ff15f3b21a56a73d89acd5ac7c3ef9cac358aae52e47d`.
Fresh means verification results were not reused; it does not claim an empty compiler cache.

The full gates include workspace/all-target/all-feature tests, static analysis, copied
command lifecycle, offline packages, distributed/outbound/stateful HTTP, service acceptance,
generated guides and retained standard/application artifact and package comparisons. No
existing test was removed or newly ignored, and no gate timeout or acceptance threshold was
relaxed. This source changed proof accounting while the unchanged artifact/package comparison
gates continued to pass.

The release command-lifecycle test passed separately inside that full profile: one passed,
zero failed/ignored, 166 filtered, 185.72 seconds. Cargo's observed test executable was
`target/release/deps/public_cli-1487f05fc811cdd1`. Its last produced application executable
is retained at
`.artifacts/lkjscript-dev/check/1790687675535381354-2379470-0/retained/release_command_lifecycle/0`.
That retained file was byte-compared with `target/release/lkjscript` before the additional
native checks; the earlier release-build producer was not substituted for it.

From `/tmp`, the same observed release test executable was run with the retained application
as `LKJSCRIPT_RELEASE_CANDIDATE`, selecting `native_effect_generic_resources` and two test
threads. All five selected tests passed, zero failed/ignored, 162 filtered, in 21.12 seconds.
The exact retained log is `.artifacts/affine-validation-work-20260929/native-effects.log`,
with `TEST_EXIT=0`. These cases preserve repeated shared-borrow suffixes, empty task callbacks,
rejection of mismatch/escape without publication, callback grants before effects, and detached
cross-package callback authority. They are preservation observations, not a new ownership or
effect capability claim.

The original predecessor failure log also has a byte-for-byte retained copy at
`.artifacts/affine-validation-work-20260929/predecessor.log`; `cmp` against the original
`/tmp/lkjscript-affine-budget-before-complete.log` succeeded. Ignored evidence files were the
only additions after the frozen source during verification.

## Delivery boundary

This acceptance belongs to source `6aa625822ec27282c1735c2c47f73271b39dc6f5`. The subsequent
reporting commit updates this campaign only, not executable source, tests or generated
artifacts. Normal fast-forward delivery to main is selected after acceptance; it does not
turn source acceptance into a binary-release claim. No release tag, binary publication,
runtime selection, running-service replacement, access-control change or history rewrite
is part of this campaign. Existing untracked files, the prior stash and other worktrees
remain outside the change.
