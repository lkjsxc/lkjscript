# Status

Current snapshot: 2026-10-04. This page owns availability and unfinished acceptance.
[Direction](direction.md) owns goals, [specifications](spec/) own semantics and
[roadmap](roadmap.md) orders future language work.

## Public binary

Immutable [v0.1.73](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.73) is
public/latest. Accepted source: `5d282e89849ba34eabc8d396c4a80041eaee2887`.
[Producer 37163764037/1](https://github.com/lkjsxc/lkjscript/actions/runs/37163764037)
completed `candidate_accepted` with joined cleanup.
[Promotion 37169036516/1](https://github.com/lkjsxc/lkjscript/actions/runs/37169036516)
completed `immutable_published_and_public_verified`. Release `402776903` is
immutable; annotated tag `0ea83404b6bb31a8c2246b0ab1d6bdb41f2b63fc` binds that exact
source. Promotion reused the original accepted archive, installer and checksum
without rebuilding.

Authenticated producer, selection, publication, anonymous public-acquisition and
terminal originals are indexed by
`.artifacts/20261003-owned-effects/release-0173/completed-publication.json` in
`/home/coder/workspace/lkjscript`. Final candidate proof covers 20 fresh source
gates, six target owners, two pinned userlands, installation/recovery and 35
required native public tests. Source, finalized candidate and public acquisition
remain separate proofs. Both anonymous exact/latest installation routes passed
30 lifecycle commands each with joined cleanup. Earlier immutable publication
and failures remain preserved; v0.1.72's original index is
`.artifacts/20261003-reusable-workers/release-0172/completed-publication.json`.

## Owned tasks with explicit authority

The [new semantic contract](spec/owned-effects.md) composes Owned generic tasks
and exact implementation witnesses with caller-supplied effects and requirements.
The [maintained native example](../examples/owned-effects/README.md) uses three
packages, distinct buffer/cell implementations, ordinary effectful callbacks and
explicit requirement forwarding. Methods remain closed and monomorphic.
[Release notes](releases/v0.1.73.md) describe the published boundary.

Fresh full source acceptance passed all 26 gates with stable inputs, zero reused
gates and no unrun gates. The original receipt is
`.artifacts/lkjscript-dev/check/1791070896199562685-742122-0/receipt.json`, digest
`verification_383ca574f1583b005381c991f28c1c41de88ba0e7f2458de556909de0732b486`.
The source/evidence index is `.artifacts/20261003-owned-effects/source-accepted.json`.
The workspace run passed 1,285 library, 202 public CLI and 253 verifier tests;
existing ignored tests retain their original status. Service, native package,
artifact, HTTP, tail-call and discovery owners passed independently.

Both copied-executable public cases passed. They check complete results and eight
capability calls after source removal; rejected proposals and the reviewed edit
preserve intended identities. Literal inputs and drafts remain under
`/home/coder/workspace/lkjscript-owned-effects-evidence-20261003/`, indexed by
`public-owned-effects-04.log` in the development evidence directory. Public native
checks passed 89 standard, 44 lkjournal, 79 guide and 62 policy tests. All four
accepted HEADs are unchanged; derived bundles, packs and discovery were refreshed
through their supported owners.

The first full run failed on stale generation assertions and the reviewed service
artifact pin. Its original stable-input receipt remains
`.artifacts/lkjscript-dev/check/1791067931985781805-632977-0/receipt.json`.
The corrected source received complete fresh acceptance above. Reporting descendants
receive their own checks and do not relabel source or finalized-byte proof.

## Compatibility and next work

Public v0.1.73 selects graph 23, semantic validator 28, authored request 27 for
nonempty implementation effect/requirement applications, compact change 31,
function-definition projection 13, compiler 23, bytecode 18 and artifact 30.
Supported predecessor semantic meaning remains readable; derived artifacts and
caches require rebuilding. Package interface 13 and type-object 10 retain their
layouts. Product-number components remain opaque identifiers. Installation never
migrates operational data, changes exact dependencies or replaces running services.

Parallel children retain their existing closed empty-effect application boundary.
The reusable auxiliary worker bound does not bound all root-plus-child CPU work
or prove fairness, preemption, speedup or hostile-code isolation. Channels, task
handles and asynchronous borrowing remain future work. The next semantic milestone
is reviewed concurrent candidate refresh with complete dependency and negative-lookup
footprints and preserved identity allocation.

Selected checkout: `/home/coder/workspace/lkjscript`, integrated on `main` and
independently confirmed through Git and GitHub's ref API. Preserve other worktrees,
the unrelated stash, original evidence and immutable publication history. No
application deployment changes here.
