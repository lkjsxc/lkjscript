# Current status

Snapshot: 2026-10-10. Product identifiers are opaque. Accepted source, mainline
delivery, tested host executables, finalized publication and running applications
remain separate boundaries.

## Delivered language and compiler source

Development **0.1.90** was integrated through PR #12. Accepted source is
`5f46e96a8f0d648f286c58ac1617bbd9bf4e4515`, tree
`2f832367988dc8391157d721f56119a6f4506ad4`; its normal merge is
`a065f740a6aa4ee90c477dfe3900963a4a8c94bc`. The subsequent reporting main is
`b2bc935ed25c90c23a2b61a86271c2f792e7789f`.

That original source passed all **26 fresh full gates**, with **2,296 workspace
passes, zero failures and 29 existing ignored**, followed by **81/81** exact-host
native-public cases. Its complete source/byte identities, original reader paths
and retained archives remain in the
[preceding immutable status](https://github.com/lkjsxc/lkjscript/blob/b2bc935ed25c90c23a2b61a86271c2f792e7789f/docs/status.md).
These proofs are not relabelled as acceptance of a later source or static archive.

The delivered implementation retains exact compiler-unit projection,
nonblocking derived-cache admission, direct bulk-List construction and ordinary
native generic folds. Issue #9 is repaired in main. No graph, type, interface,
bytecode, artifact or application-data encoding change is selected by this follow-up.

## Failed distribution candidate

Candidate [38034392926 / attempt 1](https://github.com/lkjsxc/lkjscript/actions/runs/38034392926)
is terminal **failed**, not running. Its selected source was the `a065f740` merge.
The original source summary reports **19/20 fresh passed gates**, no reuse, and
`workspace_tests: timeout`. The gate's existing 3,600-second allowance included both
Cargo compilation and execution of every workspace test target. Compilation used
15 minutes 17 seconds; the completed dependency-plan and outcome-verification
executables alone used 515.79 and 803.15 seconds. The public-CLI target was still
incomplete when the owner timed out. Completed cases cannot stand in for its
unrun remainder. This is not a successfully finalized candidate.

The original source receipt digest is
`verification_6a571eb8619093ed1f8c7c00ed74c40a8355a732f32d793f4a64d463a9639c8f`.
Downloaded diagnostic originals are retained under
`.artifacts/release-38034392926/` in the worktree below, including the original
`1791617687971589536-6499-0` receipt and process logs. Asset production, finalized
candidate acceptance and publication did not complete. No tag or public assets
were created by this failed attempt.

## Selected source-verification profile

The current follow-up changes Cargo's source-test build configuration, not the
language implementation or distribution profile. The test profile selects level-2
optimization, line-table debug information and no Rust incremental build store,
while keeping **debug assertions and integer overflow checks enabled**. A separate
`test-debug` profile keeps unoptimized code, full debug information and incremental
Rust builds for interactive debugging. The
[verification guide](development-verification.md#test-execution-versus-interactive-debugging)
explains the tradeoffs and exact selection.

No test, assertion, reference comparison, gate, deadline or cleanup obligation is
removed. Full and release-source remain their existing 26- and 20-gate profiles.
This does not disable lkjscript's derived compiler cache or speed up an already
published Release executable. No compiler bridge or new native graph planner is
implemented by this follow-up.

The owned implementation worktree is
`/home/coder/workspace/lkjscript-compiler-dependency-bridge-20261010`, branch
`work/economical-verification-20261010`, based on `b2bc935e`. Its directory name
predates the narrowed change. Evidence is in `.artifacts/verification-profile/`.

## Matched focused observations, not whole-source acceptance

The preliminary comparison recorded in `ac167709` used `cargo test --no-run` for
the level-1 product but `cargo build --profile test` for the level-2 product. A
subsequent same-command audit compiled successfully but returned a **failed byte
comparison**: the two level-2 selections were different. Those original passing
behavior observations and timings are preserved, but their claim to isolate only
optimization level is withdrawn. The audit failure is not relabelled as a pass.

The corrected comparison selects both products with
`cargo test --locked --test public_cli --no-run --message-format=json`, retains
line-table debug information and disables Rust incremental compilation on both.
The test profiles select different optimization levels. One immutable public-CLI harness runs
the same seven unique native-fold cases, with CPU affinity 2,3 and two test threads.
Every case passed in all four runs, including complete returned results, retained
nested histories, source deletion and negative admission. Order was A1/C1/C2/A2:

| Selection | First observation | Reverse-order observation | Executable bytes |
| --- | ---: | ---: | ---: |
| Level 1 control | 110.067 s | 106.779 s | 210,385,536 |
| Level 2 matched candidate | 96.858 s | 97.542 s | 182,230,088 |

These are two observed pairs on a shared host, not a timing gate, a statistical
performance guarantee or a matched cold-build comparison of the entire preceding
default profile. Complete source verification concurrently used the separately
selected CPUs 0,1; affinity is not whole-host isolation. The previously published
Release executable and whole-language performance are not changed by this result.

The shared harness SHA256 is
`2fc8d119f9d50dc54f2eb486f0d5f05e24ca6cd712af3446730402a6fee37699`;
the matched candidate SHA256 is
`2b30d05d81fba1645314e8d4dad61ebb4aecd8cd7475a955f4c02e0c080ad0db`.
Source/copy bytes were compared before selection and rechecked unchanged after
both pairs. Original `matched-*` logs, selected Cargo observations, the failed
same-command identity audit and both old and corrected product copies are retained
under `.artifacts/verification-profile/` in the owned implementation worktree.
The independently compared `copied-byte-witnesses.tar.gz` has SHA256
`fdaac87452de99b0066b8738671be322a900c34bb0ff2c62ab0064c251768013`.

The independent tiny configuration probe exercised a failing debug assertion and
runtime integer overflow under both test and test-debug settings; both guards
trapped as required. These are configuration observations, not additional unique
language-product cases. With the corrected matched candidate, maintained native
checks passed standard **96**, lkjournal **44**, guides **79** and policy **62**,
all with production/reference equality and unchanged selected executable bytes.
The frozen new source still requires completed source-matched full acceptance
before main integration. The hosted release-source timeout is not declared repaired
merely because these focused observations passed.

## Public binary

An independent live API read still selects immutable public/latest **v0.1.89**,
release ID `407859944`, source `1b7e95badb13e5ddcc9d557027c1977fd4f2a271`,
producer `37910899478/1` and promotion `37925038733/1`. Its genuine generation-4 acceptance does not include the later compiler/cache and
bulk-List changes. Existing publication, applications and operational data remain
unchanged.

## Recovered terminal and renewed original custody

The previously uncollected full run for `ac167709` returned exit zero and a
`passed` summary: 26 selected, 26 fresh passed, zero reused. Its original receipt
was 65,356 bytes with digest
`verification_796e6051816f0f03dcc37b12577261bea485b8ebdf5bde068495e8771d801374`.
The terminal was recovered through its original execution session, not inferred
from the absence of a process. Its original tmpfs checkout and receipt are no
longer present. That recovered summary is retained as an observation, not a
replacement for the missing original source and per-gate evidence.

A new complete source run is selected in the persistent worktree above, including
the corrected profile comparison and specification. No previous result is renamed.
Capacity was recovered only from the inactive Rust incremental directory of the
original owned lkjscript checkout, after checking for build owners. Source, Git
state, executables, evidence, other worktrees and running applications were not
removed. Exact before/after observations are in
`.artifacts/verification-profile/resumed-full/`.

## Next acceptance boundary

Freeze and accept this follow-up through the unchanged complete source owner,
then integrate normally and independently reread main. Select a fresh configured
non-publishing 0.1.90 producer only after source acceptance; retain the failed
producer as failure rather than replaying or relabelling its incomplete proof.
Finalized static bytes, all target owners, both pinned userlands, installation,
complete native-public coverage and original-reader admission remain mandatory
before any unchanged-asset promotion under [the release procedure](release.md).
