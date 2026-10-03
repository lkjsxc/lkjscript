# Status

Current snapshot: 2026-10-03. This page owns availability and unfinished acceptance.
[Direction](direction.md) owns goals, [specifications](spec/) own semantics and
[roadmap](roadmap.md) orders future language work.

## Public binary

Immutable [v0.1.72](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.72) is
public/latest. Accepted source: `6a14335e4d57015da0d4663a6ef3b1e49bb321fc`.
[Producer 37148975781/1](https://github.com/lkjsxc/lkjscript/actions/runs/37148975781)
completed `candidate_accepted` with joined cleanup.
[Promotion 37157337920/1](https://github.com/lkjsxc/lkjscript/actions/runs/37157337920)
completed `immutable_published_and_public_verified`. Release `402711413` is
immutable; annotated tag `d04c75a6c3f2451ecb1abf46ae69e137c2ea92a9` binds that exact
source. Promotion reused the original accepted archive, installer and checksum
without rebuilding the candidate or repeating its completed broad acceptance.

Authenticated original producer, selection, publication, public verification and
terminal evidence is under
`.artifacts/20261003-reusable-workers/release-0172/promotion-01/` in
`/home/coder/workspace/lkjscript`. The original terminal is `release-terminal.json`;
`producer-proof-originals.json` indexes the producer's exact receipt bindings.
Final candidate proof covers 20 fresh source gates, six target owners, two pinned
userlands, installation/recovery and 33 required native public tests. Source,
finalized candidate and anonymous public acquisition remain separate proofs.

The preceding fresh development-source acceptance passed all 26 gates at the
same source. Its original receipt remains
`.artifacts/lkjscript-dev/check/1791055313021873408-308116-0/receipt.json`, digest
`verification_9190b0b6ead630a89a885250bfc09a5008eed35a3b2a13f700130fc406345516`.
`.artifacts/20261003-reusable-workers/source-accepted.json` indexes that receipt,
retained originals and matched performance evidence. Reporting descendants do not
relabel exact-source acceptance. Failed earlier attempts remain preserved there.

Public v0.1.72 reuses lazily created auxiliary workers under an explicit joined
lifetime owner. The [three-package parallel workload](../examples/parallel-work/README.md)
uses both owned carriers, complete payloads, serial/parallel/nested execution and
an independent sibling service. [Release notes](releases/v0.1.72.md) and
[measurements](performance.md#reusable-structured-workers-2026-10-03) retain its
exact capabilities, slower cases and limits.

## Development v0.1.73: owned effect applications

Selected checkout: `/home/coder/workspace/lkjscript`, branch `main`, based on
`62f8e4f910596d6cee8aaf9727747c17c26b543c`. Implementation and maintained assets are
complete; fresh full source and finalized-byte acceptance remain pending.

The [new semantic contract](spec/owned-effects.md) composes Owned generic tasks
and exact implementation witnesses with caller-supplied effects and requirements.
The [maintained native example](../examples/owned-effects/README.md) uses three
packages, distinct buffer/cell implementations, ordinary effectful callbacks and
explicit requirement forwarding. Methods remain closed and monomorphic.

Development evidence belongs under `.artifacts/20261003-owned-effects/`. Workspace
Clippy, 10 owned-effect tests, 21 implementation tests, predecessor controls and
both copied-executable public tests passed. The public workload checks complete
results and eight capability calls after source removal; rejected proposals and
the reviewed edit preserve the intended identities. Literal inputs and drafts are
retained under `/home/coder/workspace/lkjscript-owned-effects-evidence-20261003/`,
indexed by `public-owned-effects-04.log`. Earlier failures remain preserved.

Public native checks passed 89 standard, 44 lkjournal, 79 guide and 62 policy tests;
all four accepted HEADs are unchanged. Their derived bundles and required packs
were rebuilt through public owners, and generated discovery verifies current.
These focused observations are not full source acceptance. Next: freeze inputs,
run all 26 fresh gates, integrate and independently verify remote main, then select
one finalized-byte candidate for publication.

## Compatibility and remaining limits

Development v0.1.73 selects graph 23, semantic validator 28, authored request 27
for nonempty implementation effect/requirement applications, compact change 31,
function-definition projection 13, compiler 23, bytecode 18 and artifact 30.
Supported predecessor semantic meaning remains readable; derived artifacts and
caches require rebuilding. Package interface 13 and type-object 10 retain their
layouts. Public v0.1.72 retains its own earlier contracts. Product-number components
remain opaque identifiers. Installation never migrates operational data, changes
exact dependencies or replaces running services.

Parallel children retain their existing closed empty-effect application boundary.
The auxiliary worker bound does not bound all root-plus-child CPU work or prove
fairness, preemption, speedup or hostile-code isolation. Channels, task handles and
asynchronous borrowing remain future work. After owned-effect acceptance, the
next semantic milestone is reviewed concurrent candidate refresh with complete
dependency and negative-lookup footprints and preserved identity allocation.

Preserve other worktrees, the unrelated stash, original evidence and immutable
publication history. No application deployment changes here.
