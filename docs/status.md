# Status

Current snapshot: 2026-10-04. This page owns availability and unfinished acceptance.
[Direction](direction.md) owns goals, [specifications](spec/) own semantics and
[roadmap](roadmap.md) orders future language work.

## Public binary

Immutable [v0.1.75](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.75) is
public/latest. Accepted product source: `62537a7de98940b2174fe2be4bce61815181e74f`.
[Producer 37229047605/1](https://github.com/lkjsxc/lkjscript/actions/runs/37229047605)
completed `candidate_accepted`; [promotion 37235741568/1](https://github.com/lkjsxc/lkjscript/actions/runs/37235741568)
completed `immutable_published_and_public_verified` at 21:30 UTC. Release
`403216806` and annotated tag `57fff224c84abf13486b94f64df214ce277e8c73` retain
that exact source. Anonymous exact, release-ID and latest identity, all three
public asset sizes and digests, and the archived executable digest were
independently verified. Promotion reused the accepted assets without rebuilding
the product.

Original publication evidence is indexed by
`.artifacts/20261004-owned-borrows/release-0175/completed-publication.json` in
`/home/coder/workspace/lkjscript`. The [release notes](releases/v0.1.75.md) describe
scoped reads of owned children and the required derived-bundle rebuild. Its full
source receipt remains `.artifacts/lkjscript-dev/check/1791141767519073420-2430056-0/receipt.json`:
26 fresh passing gates, zero reused gates, at the exact source above. Later
reporting commits do not change the source proved by that receipt. Original
failures and copied-executable inputs remain at their recorded owners.

## Owned sequences accepted on main; publication pending

Accepted product source `10c26f0b68d3e99b8bc1319ca83f7a8d3c90a71d` implements
[runtime-sized owned sequences](spec/owned-sequences.md) for v0.1.76. It is
integrated on remote main through a normal fast-forward, independently confirmed
through Git and the GitHub branch API. Later status-only reporting commits do not
change the tested product source.

The capability adds generic construction, append, LIFO removal, lexical indexed
reads and structured transfer under the complete element contract. The
[three-package example](../examples/owned-sequences/README.md) owns literal native
inputs and independent full results. [Release notes](releases/v0.1.76.md) describe
the coordinated format cut and required derived-bundle rebuild.

Fresh full-source receipt
`.artifacts/lkjscript-dev/check/1791154187470041424-2850821-0/receipt.json`
passed all 26 gates, with zero reused gates and stable inputs at the exact source
above. Its digest is
`verification_4b0dad02aa0222653471ac547581c28a4a02c424f32eb6b0213b8e711b07e3c8`.
The workspace includes 1,408 passing library tests, 216 public CLI tests and 253
developer-tool tests, including all three new sequence public cases. Maintained
standard, lkjournal, guide and policy checks pass 281 native tests collectively.
Their derived artifacts and discovery were regenerated through the product.
Standard adds sequence wrappers and seven graph tests; application and policy
semantic HEADs remain unchanged.

Development evidence is retained under `.artifacts/20261004-owned-sequences/`.
The copied-product cases pass in `public-focus-03/tests.log`, with literal inputs
outside the checkout retained under its recorded temporary root. Original failures
remain intact, including the 513-element reference stack failure repaired by
preserving tail position through consuming matches and unpacking, and the initial
24-of-26 full receipt
`.artifacts/lkjscript-dev/check/1791152416842311486-2739346-0/receipt.json`.
Indexed native cold reproduction passed in `cold-tracked-02/`: all four bundles
and standard transport match byte for byte, with all 187 packs and four semantic
HEADs unchanged.

[Producer 37242435402/1](https://github.com/lkjsxc/lkjscript/actions/runs/37242435402)
was dispatched once from main at the accepted product source at 23:04:52 UTC on
2026-10-04. At 23:05 UTC it is building immutable host tools. Evidence is retained
under
`.artifacts/20261004-owned-sequences/release-0176/source-10c26f0b/`; source and
mainline evidence is indexed by the parent directory's `source-handoff.json`.
The remaining gate is final-archive candidate acceptance, including source gates,
target owners, pinned userlands, installation recovery, the native public harness
and original-reader admission. After `candidate_accepted`, authenticate the exact
producer run/attempt and unchanged assets, create the ordinary annotated v0.1.76
tag, update and read back the scoped release selection, then promote through
`immutable_published_and_public_verified`. No v0.1.76 tag or release selection has
been created; v0.1.75 remains public/latest.

Preserve the other worktrees, unrelated stash, original fixtures and failures,
and immutable publication history. No application deployment changes are part
of this work.
