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

## Focused observations, not whole-source acceptance

One immutable public-CLI harness selected two explicitly copied products, with
unchanged source tests, CPU affinity 0,1 and two test threads. Every run passed all
**seven unique native-fold cases**, including complete results, retained nested
histories, source-deleted execution and negative admission. The paired order was
baseline/candidate, then candidate/baseline:

| Selection | First observation | Reverse-order observation | Executable bytes |
| --- | ---: | ---: | ---: |
| Level 1 control | 108.198 s | 108.751 s | 210,385,536 |
| Level 2 candidate | 99.155 s | 99.144 s | 182,226,760 |

Both comparison products used line-table debug information and disabled Rust
incremental compilation. This isolates optimization-level behavior; it is **not**
a matched measurement of the entire preceding default test profile, cold-build
cost, CI completion or whole-language performance. No timing predicate replaces
correctness checks. Original favorable and unfavorable observations are retained.

The shared harness SHA256 is
`2fc8d119f9d50dc54f2eb486f0d5f05e24ca6cd712af3446730402a6fee37699`;
the candidate product SHA256 is
`dbe545f2da6fd210195cb7d53d4fc6b150d0e9e830df182c83e427add379acef`.
Source/copy identities were compared before selection and checked unchanged after
both pairs. The independent tiny profile probe also exercised a failing debug
assertion and runtime integer overflow under both test and test-debug settings;
both guards trapped as required. It is not an additional language-product case.

Maintained native checks passed standard **96**, lkjournal **44**, guides **79**
and policy **62**, all with production/reference equality and no new tracked
source packs. Formatting and whitespace checks passed for the focused change.
The new source still requires frozen, source-matched full acceptance before main
integration. The hosted release-source timeout is not declared repaired merely
because this focused comparison passed.

## Public binary

An independent live API read still selects immutable public/latest **v0.1.89**,
release ID `407859944`, source `1b7e95badb13e5ddcc9d557027c1977fd4f2a271`,
producer `37910899478/1` and promotion `37925038733/1`. Its genuine generation-4 acceptance does not include the later compiler/cache and
bulk-List changes. Existing publication, applications and operational data remain
unchanged.

## Next acceptance boundary

Freeze and accept this follow-up through the unchanged complete source owner,
then integrate normally and independently reread main. Select a fresh configured
non-publishing 0.1.90 producer only after source acceptance; retain the failed
producer as failure rather than replaying or relabelling its incomplete proof.
Finalized static bytes, all target owners, both pinned userlands, installation,
complete native-public coverage and original-reader admission remain mandatory
before any unchanged-asset promotion under [the release procedure](release.md).
