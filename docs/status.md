# Status

Current snapshot: 2026-10-05. This page owns availability and unfinished acceptance.
[Direction](direction.md) owns goals, [specifications](spec/) own semantics and
[roadmap](roadmap.md) orders future language work.

## Public binary

Immutable [v0.1.76](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.76) is
public/latest. Accepted product source: `10c26f0b68d3e99b8bc1319ca83f7a8d3c90a71d`.
[Producer 37242435402/1](https://github.com/lkjsxc/lkjscript/actions/runs/37242435402)
completed `candidate_accepted`; [promotion 37249338644/1](https://github.com/lkjsxc/lkjscript/actions/runs/37249338644)
completed `immutable_published_and_public_verified` at 01:03:53 UTC. Release
`403287008` and annotated tag `06e0c2b07ae0512a25d1eb8bb2a4f9adefbc274e` retain
that exact source. Anonymous exact, release-ID and latest identity, all three
public asset sizes and digests, and the archived executable were independently
verified at 01:06:14 UTC. Promotion reused accepted assets without rebuilding the
product or replaying heavy application acceptance.

Original publication evidence is indexed by
`.artifacts/20261004-owned-sequences/release-0176/completed-publication.json` in
`/home/coder/workspace/lkjscript`. The [release notes](releases/v0.1.76.md) describe
runtime-sized owned sequences and their required derived-bundle rebuild. Full
source receipt
`.artifacts/lkjscript-dev/check/1791154187470041424-2850821-0/receipt.json`
retains 26 fresh passing gates, zero reused gates and stable inputs at the exact
source above. Later reporting commits do not change that tested product source.
No v0.1.76 publication gate remains.

## Parameterized owned contracts: acceptance in progress

The main checkout contains the v0.1.77 implementation of
[parameterized owned contracts](spec/owned-contract-parameters.md), based on
`bd8321cff99316bd5a286791b196a38894b7651e`. Fresh complete source acceptance and
mainline integration remain pending; this is not a public executable selection.

Ordered Owned arguments and structural method signatures compose reusable flat
and chunk32 worklists for independent cell and buffer carriers. The
[native witness](../examples/owned-worklists/README.md) includes generic transfer,
drain/reuse and complete provisional-graph validation/reachability. These are
designed language witnesses, not compiler self-hosting or application adoption.
The [release notes](releases/v0.1.77.md) describe the coordinated format cut and
required derived-bundle rebuild.

Development evidence is retained under `.artifacts/20261005-owned-contracts/`:
23 codec/interface tests and 23 focused admission/runtime/authoring tests pass;
strict workspace lint passes. Standard, lkjournal, native guide and policy checks
pass 281 native tests with equal bytecode/reference results. Their derived assets
and discovery were regenerated through the product. All maintained semantic HEADs
remain unchanged. Indexed cold reproduction in `cold-tracked-01/` reproduces all
four bundles and standard transport exactly, with all 191 packs unchanged. The
61 recorded historical fixture files remain byte-identical.

Fresh copied-product authoring in `public-authoring-smoke-01/` passes the
three-package example, 106 graph tests, both reachability targets and complete
513-item storage results. All 22 negative proposals reject semantically and
preserve HEAD. All three corrected public-harness cases pass in `public-focus-04/`,
including source-deleted execution, complete results and capacity boundaries;
their literal inputs and copied executables remain at the recorded temporary
roots. Initial failure logs remain in `public-focus-03/`; those earlier temporary
roots were automatically removed.

Matched seven-sample observations in `measure-storage-01/` and
`measure-reachability-01/` retain complete results, stage timings, modeled
allocation and coarse process RSS. Chunk32 executes more slowly and charges more
allocation than flat storage in every measured workload; no retained-owned-byte
or API-cost advantage is established. Full fresh 26-gate acceptance, normal
mainline delivery and a new source-specific release producer remain outstanding.
No v0.1.77 tag or release selection has been created.

Preserve other worktrees, unrelated stashes, original fixtures and failures, and
immutable publication history. No application deployment changes are part of
this work.
