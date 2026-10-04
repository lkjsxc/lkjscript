# Status

Current snapshot: 2026-10-04. This page owns availability and unfinished acceptance.
[Direction](direction.md) owns goals, [specifications](spec/) own semantics and
[roadmap](roadmap.md) orders future language work.

## Public binary

Immutable [v0.1.74](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.74) is
public/latest. Accepted product source: `9f8610e896bb8f96345054ef17d47501cda2333d`.
[Producer 37215197861/1](https://github.com/lkjsxc/lkjscript/actions/runs/37215197861)
completed `candidate_accepted`; [promotion 37221494622/1](https://github.com/lkjsxc/lkjscript/actions/runs/37221494622)
completed `immutable_published_and_public_verified` at 17:49 UTC. Release
`403135466` and annotated tag `36e42f70d5fc7543acbfaf3fa7259dd039275909` retain
that exact source. Anonymous exact/latest identity and all three public asset
sizes and digests were independently verified. Promotion reused the accepted
assets without rebuilding the product.

Original publication evidence is indexed by
`.artifacts/20261004-refresh/release-0174/completed-publication.json` in
`/home/coder/workspace/lkjscript`. The [release notes](releases/v0.1.74.md) describe
reviewed concurrent candidate refresh and its compatibility cut. Its full source
receipt remains `.artifacts/lkjscript-dev/check/1791128757456244504-1878981-0/receipt.json`:
26 fresh passing gates, zero reused gates, at the exact source above. Later
reporting commits do not change the source proved by that receipt. Original
failures and copied-executable inputs remain at their recorded owners.

## Scoped owned reads accepted on main

Accepted product source `62537a7de98940b2174fe2be4bce61815181e74f` adds
[lexical read access to owned children](spec/owned-borrows.md): generic
product-field views and exhaustive borrowed choice inspection, with explicit
ancestor custody and all-exit cleanup. The [maintained example](../examples/owned-borrows/README.md)
owns the native cross-package witness. The normal mainline push was independently
verified through Git and the GitHub branch and comparison APIs. This status-only
reporting descendant does not change the source selected for product acceptance.

Fresh full acceptance passed all 26 gates, with zero reused gates, stable inputs
and no unrun gates. The exact-source receipt is
`.artifacts/lkjscript-dev/check/1791141767519073420-2430056-0/receipt.json`
in `/home/coder/workspace/lkjscript`. It includes 1,347 library tests, 213 public
CLI tests and 253 developer-tool tests passing, alongside the other suites and
copied-release workflows; intentional ignored cases remain recorded in the logs.
All three new public borrowing tests passed in that run.

Development evidence remains under `.artifacts/20261004-owned-borrows/`.
`public-focus-02/tests.log` and `public-focus-02/external-tmpdir.txt` retain the
earlier copied-product results and outside-checkout literal inputs. Maintained
bundles, standard transport and discovery were regenerated through the product;
all four accepted program HEADs are unchanged. Original failed receipts remain
intact. The [release notes](releases/v0.1.75.md) describe the intentional contract
changes and required rebuilding of prior derived bundles.

## Pending final-archive acceptance

[Candidate producer 37229047605/1](https://github.com/lkjsxc/lkjscript/actions/runs/37229047605)
was dispatched on 2026-10-04 at 19:38 UTC from exactly
`62537a7de98940b2174fe2be4bce61815181e74f`. Final-archive acceptance and
unchanged-asset publication remain separate from completed source acceptance and
mainline integration. The successor is not yet publicly released; no successor
tag or publication-control change has been made. Public/latest remains v0.1.74.

Dispatch observations, source acceptance summary and exact release notes are
retained in `.artifacts/20261004-owned-borrows/release-0175/source-62537a7d/`.
Next: inspect that producer's terminal result and essential uploads. After
`candidate_accepted`, refresh release occupancy and mainline reachability, then
select the ordinary annotated v0.1.75 tag at the accepted source and promote
producer `37229047605`, attempt `1`, following the [release procedure](release.md).
Require `immutable_published_and_public_verified` before claiming public closure.
Reuse this healthy producer; do not dispatch a duplicate while it is pending.

Preserve the other worktrees, unrelated stash, original failures and immutable
publication history. No application deployment changes are part of this work.
