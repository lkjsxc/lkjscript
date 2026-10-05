# Status

Current snapshot: 2026-10-05. This page owns availability and unfinished acceptance.
[Direction](direction.md) owns goals, [specifications](spec/) own semantics and
[roadmap](roadmap.md) orders future language work.

## Public binary

Immutable [v0.1.76](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.76) is
public/latest at accepted product source `10c26f0b68d3e99b8bc1319ca83f7a8d3c90a71d`.
[Producer 37242435402/1](https://github.com/lkjsxc/lkjscript/actions/runs/37242435402)
completed `candidate_accepted`; [promotion 37249338644/1](https://github.com/lkjsxc/lkjscript/actions/runs/37249338644)
completed `immutable_published_and_public_verified` on 2026-10-05 at 01:03:53 UTC.
Release `403287008` and annotation `06e0c2b07ae0512a25d1eb8bb2a4f9adefbc274e`
retain that exact source. Anonymous exact, release-ID and latest identity, all
three asset sizes/digests, and the archived executable passed independent
readback at 01:06:14 UTC. Promotion reused accepted assets with no product
rebuild or heavy application replay. No v0.1.76 gate remains.

Original publication evidence is indexed by
`.artifacts/20261004-owned-sequences/release-0176/completed-publication.json` in
`/home/coder/workspace/lkjscript`. Full receipt
`.artifacts/lkjscript-dev/check/1791154187470041424-2850821-0/receipt.json` retains
26 fresh gates, zero reused gates and stable inputs at that product source.
The [release notes](releases/v0.1.76.md) describe owned sequences and their required
bundle rebuild; later reporting commits do not change the tested selection.

## Parameterized owned contracts accepted on main; candidate pending

Accepted product source `2f4936e831a2de1d96ec500e071ac84ad9e8bebf` implements
[parameterized owned contracts](spec/owned-contract-parameters.md) for v0.1.77.
Normal fast-forward integration on remote main was independently confirmed through
Git and the GitHub branch API. Full receipt
`.artifacts/lkjscript-dev/check/1791164478929468490-3238945-0/receipt.json`
passed all 26 fresh gates, with zero reuse and stable inputs; its digest is
`verification_fd0ac745c0cf3113f1f8967a752f310a0a7668f5f7d2a3ced0b16804fa30595b`.
Later reporting commits do not relabel that source. Receipt integrity and both
mainline observations are indexed by
`.artifacts/20261005-owned-contracts/accepted-source.json`.
Workspace logs record 1,431 passing library tests with eight ignored, 219 passing
public CLI tests with one ignored, and 253 developer-tool tests with 19 ignored.
All three new owned-contract public cases ran and passed.

The initial full attempt at `7697b2e5b52b368ce38dfa1757123a2c896a1931`
passed 25 fresh gates; its workspace gate failed on two stale golden assertions.
Original receipt `.artifacts/lkjscript-dev/check/1791163198766571170-3149869-0/receipt.json`
and failure logs remain intact. Test-only correction `2f4936e8` freezes the current
graph and validator contract expectations; acceptance above belongs to its fresh
repeat, not the failed predecessor. Ignored tests remain classified separately
in the original workspace logs and are never counted as passing.

Ordered Owned arguments and structural method signatures compose generic flat and
chunk32 worklists for independent cell and buffer carriers. The
[native witness](../examples/owned-worklists/README.md) exercises transfer,
drain/reuse and complete provisional-graph validation/reachability. Compiler
self-hosting and application adoption remain future work. The
[release notes](releases/v0.1.77.md) describe the format cut and required bundle rebuild.

Original evidence remains under `.artifacts/20261005-owned-contracts/`: 23 codec/
interface and 23 focused tests; 281 maintained native tests; exact cold reproduction
of four bundles and standard transport with 191 packs unchanged; and 61 unchanged
historical fixtures. Derived assets/discovery were regenerated through the product;
maintained semantic HEADs are unchanged. `public-authoring-smoke-01/` passes 106 graph
tests, both reachability targets, complete 513-item results and 22 semantic rejection
cases preserving HEAD. All three corrected public cases pass in `public-focus-04/`,
including source-deleted execution and capacity boundaries. Literal inputs and
copied binaries remain at recorded temporary roots. Failure logs remain in
`public-focus-03/`; its earlier temporary roots were automatically removed.

Seven-sample observations in `measure-storage-01/` and `measure-reachability-01/`
retain complete outputs, stage timings, modeled allocation and coarse RSS.
Chunk32 is slower and charges more allocation than flat storage in every measured
workload; no retained owned-byte or API-cost advantage is established.

[Producer 37253512076/1](https://github.com/lkjsxc/lkjscript/actions/runs/37253512076)
was dispatched once from main at the accepted product source at 2026-10-05 01:58:22 UTC.
At 2026-10-05 02:00:16 UTC, its state is in_progress and current gate is
`Fetch locked dependencies and build immutable host tools`. Original
source/dispatch/run/attempt/job/artifact observations
remain under `.artifacts/20261005-owned-contracts/release-0177/source-2f4936e8/`.

Final-archive candidate acceptance remains required: source gates, target owners,
pinned userlands, installation recovery, native public harness and original readers.
After `candidate_accepted`, authenticate the exact producer attempt/assets, create
an ordinary annotated v0.1.77 tag at the accepted product source, update/read back
scoped selection, then promote through `immutable_published_and_public_verified`.
No v0.1.77 tag or release selection exists; v0.1.76 remains public/latest.

Preserve other worktrees, unrelated stashes, original fixtures/failures and immutable
publication history. No application deployment changes are part of this work.
