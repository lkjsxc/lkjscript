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

## Scoped owned reads in development

The selected checkout is `/home/coder/workspace/lkjscript`, branch `main`, based on
`25a38f9a6ac9bb00fded3bc32f9566b3bd90dfd6`. The selected implementation
adds [lexical read access to owned children](spec/owned-borrows.md): generic
product-field views and exhaustive borrowed choice inspection, with explicit
ancestor custody and all-exit cleanup. The [maintained example](../examples/owned-borrows/README.md)
owns the native cross-package witness. This successor is not yet source-accepted,
integrated or publicly released.

Development output is retained under `.artifacts/20261004-owned-borrows/`.
The three copied-product feature tests passed with the development executable;
`public-focus-02/tests.log` and its `external-tmpdir.txt` retain the results and
outside-checkout literal inputs. Maintained bundles, standard transport and
discovery were regenerated through the product; all four accepted program HEADs
are unchanged. Workspace linting and all 14 corrected library regressions passed.
Original failed runs remain retained.

The first fresh full run on `0e654f72845d7d264fd84d56ead9a68a3a562d29`
recorded 23 passing gates and three failures from stale verification expectations,
with stable inputs and no unrun gates. Its original receipt is
`.artifacts/lkjscript-dev/check/1791138981050628568-2247770-0/receipt.json`.
The selected successor corrects public discovery, definition-projection and
maintained service-artifact expectations; complete acceptance must be renewed.

Next: run dependency-complete fresh source acceptance on the stabilized
implementation. Integrate the accepted source normally and
select a new release candidate through the [release procedure](release.md).

Preserve the other worktrees, unrelated stash, original failures and immutable
publication history. No application deployment changes are part of this work.
