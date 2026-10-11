# Current status

Snapshot: 2026-10-11. Accepted meaning, source verification, mainline delivery,
finalized executable acceptance and public availability are separate boundaries.

## Generalized owned sequences

Work is in `/home/coder/workspace/lkjscript-generalized-sequences-20261011`, branch
`work/generalized-sequences-20261011`. The implementation preserves the native
FIFO work and its corrections through `32594261` by normal merge. Ordinary
first-order sequence elements, immutable
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

Fresh full verification of source
`99232e109758f646c88d1f36f59d93bf1e1b8b38`, tree
`57eef72efdb6678d5e0f71a474ce7af1bc0f801d`, passed **26/26 gates**, with zero
reuse, zero unrun gates and stable inputs. Workspace results were **2,372 passed,
0 failed, 29 ignored**, plus two separately filtered passing child probes.
Acceptance includes all five generalized-sequence witnesses, all five FIFO
witnesses, all three planner tests, all nine independent verifier tests, maintained
artifacts, source lint, offline packages, HTTP, tail execution and services.

The passing receipt is
`verification_40eb2d40302de639cc773b1f85aa426bd51b37fc23d09f1b1e09bb1642b6c5de`,
at `.artifacts/lkjscript-dev/check/1791680514015518854-558889-0/receipt.json`.
Its immutable checker, source identity and independently checked result summary
are retained in `.artifacts/data-sequences/full-99232e10/`.

The preceding full run at `3b54f64a` passed 24/26 gates. Its stale artifact,
discovery and definition-projection expectations were corrected with exact
admission and historical negative controls retained. The original failed receipt
is
`verification_af35c17bcdd2fadf874dc55b7e79cd3807d6b27f3647c9772249edc14bc4b26b`,
at `.artifacts/lkjscript-dev/check/1791676881020641040-443828-0/receipt.json`.
Builds, focused checks and both cost studies remain under
`.artifacts/data-sequences/`. Original failed and cancelled attempts remain distinct.
The earlier FIFO receipt `verification_0784e1cd3bbe8b0253cb062fdc1d29e412fd04e2532fea11bcdad74075beb183`
remains in the FIFO worktree under `.artifacts/native-fifo/resume-20261010/full-b9cacb40/`.
Its fixture corrections are included by normal merge; production limits are unchanged.

## Mainline and public binary

Implementation reached remote main through the normal merge of
[PR #14](https://github.com/lkjsxc/lkjscript/pull/14), commit
`d8e62956791fd649e0a6bfd2722067375cacce83`. Its tree equals the accepted source
tree above, and exact accepted-source ancestry was independently verified.
The integration proof is `.artifacts/data-sequences/delivery/integration.json`.
This status update is a reporting-only descendant, separate from tested source
and candidate bytes.

Development is **0.1.90**. Latest observed immutable public release is **v0.1.89**,
release **407859944**. The v0.1.90 tag and release were unoccupied immediately
before the new candidate dispatch. Earlier candidate
`38034392926/1` failed source verification; no finalized bytes from it were accepted
or promoted.

Candidate [38103217658/1](https://github.com/lkjsxc/lkjscript/actions/runs/38103217658)
was dispatched once at **2026-10-11 01:51:09 UTC** from exact product/controller
source `d8e62956791fd649e0a6bfd2722067375cacce83`. GitHub independently identifies
repository and head repository `lkjsxc/lkjscript`, workflow **343592338** at
`.github/workflows/release.yml`, event `workflow_dispatch`, branch `main`, original
attempt **1**. The run is in progress; checkout succeeded and hosted acceptance
remains pending. Original dispatch, run, job and artifact responses are retained
under `.artifacts/data-sequences/delivery/candidate-dispatch/`.

Next: require that exact invocation's **`candidate_accepted`** terminal and required
assets, verifier and acceptance uploads. Hosted acceptance must independently pass
fresh release-source checks and generation-7 finalized-byte acceptance, including
six target owners, two pinned userlands, installed recovery and native public
witnesses. Local full acceptance does not substitute for these boundaries.

After acceptance, refresh occupancy and publication state, annotate `v0.1.90` on
the exact admitted product commit, compare and read back the existing scoped
tag-object selection, then promote that producer/attempt's unchanged assets through
the [release owner](release.md). Completion requires
**`immutable_published_and_public_verified`**. No v0.1.90 public acceptance or
publication is claimed yet. Retain both sequence/FIFO worktrees and their unique
ignored evidence through this handoff. Application selection, deployment and
operational data are unchanged.
