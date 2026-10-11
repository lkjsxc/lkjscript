# Current status

Snapshot: 2026-10-11. Accepted meaning, source verification, mainline delivery,
finalized executable acceptance and public availability are separate boundaries.

## Generalized owned sequences

Work is in `/home/coder/workspace/lkjscript-generalized-sequences-20261011`, branch
`work/generalized-sequences-20261011`. Implementation `8ffb023e` and normal merge
`3b54f64a9cc1835ba702c14d26f5d00fe28c2d39` preserve the native FIFO work and its
corrections through `32594261`. Ordinary first-order sequence elements, immutable
indexed reads, custody-preserving replacement and dense working storage in the
bounded native dependency planner are implemented. The [specification](spec/owned-sequences.md),
[data guide](guides/native-data-sequences.md) and [release notes](releases/v0.1.90.md)
own the semantics and intentional format cut.

The accepted standard revision is
`rev_55fb7bfafcef4ec97b158b23e5f8d49c244e8cf64d32786fa81448f136effc59`:
2,796 live owners, 290 compiler units and **116 passing graph tests**.
Maintained application, guide and policy checks pass **44**, **79** and **62**
tests. Production/reference results agree; regenerated artifacts and transports
match, with accepted application HEADs and exact suppliers retained. Required
native packs and generated reference pages are included in source.

The matched 180-execution planner study passes independent results and cleanup
checks. A bounded, program-bound lookup removes repeated generic type-table
searches, but large sequence stage cases remain **2.5–2.7 times slower** than the
map/list baseline; nontrivial complete plans remain **5–17% slower**. Both the
original unfavorable study and its identical repeat are retained in the
[planner owner](../examples/dependency-plan/README.md), including exact identities
and modeled metadata costs. This is a capability advance with a measured cost.

## Source acceptance

Fresh full verification of `3b54f64a` completed with **24/26 gates passed**, zero
reuse, zero unrun gates and stable inputs. Workspace results were **2,370 passed,
2 failed, 29 ignored**, apart from two separately filtered child probes. All five
generalized-sequence witnesses, all five FIFO witnesses, all three planner tests,
all nine independent verifier tests, 1,762 core tests and 285 developer-tool tests
passed. Source lint, maintained artifacts, offline packages, outbound/stateful
HTTP, tail execution and service acceptance passed too.

Three stale expectations blocked acceptance: the service integration test still
required artifact 37 instead of 38; public discovery omitted get/replace; and the
distributed HTTP checker still required definition projection 19 instead of 20.
The corrections retain exact admission and historical negative controls. Focused
reruns now pass all nine service integration tests, complete public discovery and
all 46 distributed HTTP commands with two runners and joined cleanup. Fresh full
acceptance of the frozen corrected source remains required before integration.

The failed full receipt is
`verification_af35c17bcdd2fadf874dc55b7e79cd3807d6b27f3647c9772249edc14bc4b26b`,
at `.artifacts/lkjscript-dev/check/1791676881020641040-443828-0/receipt.json`.
Builds, focused checks and both cost studies remain under
`.artifacts/data-sequences/`. Original failed and cancelled attempts remain distinct.
The earlier FIFO receipt `verification_0784e1cd3bbe8b0253cb062fdc1d29e412fd04e2532fea11bcdad74075beb183`
remains in the FIFO worktree under `.artifacts/native-fifo/resume-20261010/full-b9cacb40/`.
Its fixture corrections are included by normal merge; production limits are unchanged.

## Mainline and public binary

Last corroborated remote main:
`574a8b5783077cb0654ecd85ede94105cb6912a1`. PR #14 remains draft at remote head
`5c86cd33`; this combined increment has not yet reached main.

Development is **0.1.90**. Latest observed immutable public release is **v0.1.89**,
release **407859944**. The v0.1.90 tag is unoccupied. Earlier candidate
`38034392926/1` failed source verification; no finalized bytes from it were accepted
or promoted. No successor candidate has been dispatched for this increment.

Next: pass fresh full source acceptance of the corrected source, integrate normally
and independently verify remote ancestry. Then accept finalized static bytes and
promote those unchanged assets through the configured release workflow. Application
selection, deployment and operational data are unchanged.
