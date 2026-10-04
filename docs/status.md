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

## Owned sequences awaiting source acceptance

The selected main worktree at `/home/coder/workspace/lkjscript`, based on
`238dc5f5f85a5340e086493c52bb57d79949aca5`, implements
[runtime-sized owned sequences](spec/owned-sequences.md) for development v0.1.76.
It adds generic construction, append, LIFO removal, lexical indexed reads and
structured transfer under the complete element contract. The
[three-package example](../examples/owned-sequences/README.md) owns literal native
inputs and independent full results. [Release notes](releases/v0.1.76.md) describe
the coordinated format cut and required derived-bundle rebuild.

Source acceptance and v0.1.76 publication remain pending. The current development
work has exercised independent kernel, compiler, reference, custody and transfer
checks. Maintained standard, lkjournal, guide and policy checks pass 281 native
tests collectively; their derived artifacts and discovery have been regenerated
through the product. Standard adds sequence wrappers and seven graph tests. The
guide changes one current phrase from a fixed structural-form count; application
and policy semantic HEADs remain unchanged.

Development evidence is retained under `.artifacts/20261004-owned-sequences/`.
Original failed builds and test logs remain intact. All three copied-product cases
pass in `public-focus-03/tests.log`, with outside-checkout literal inputs retained
under its recorded temporary root. They include the original 513-element reference
stack failure, repaired by preserving tail position through consuming matches and
unpacking. Indexed native cold reproduction passed in `cold-tracked-02/`: all four
bundles and standard transport match byte for byte, with all 187 packs and four
semantic HEADs unchanged. Initial full receipt
`.artifacts/lkjscript-dev/check/1791152416842311486-2739346-0/receipt.json`
passed 24 of 26 gates at `638b011f38d140831e587bcefe557db22569510c`, with stable
inputs. Its workspace and service gates exposed stale discovery assertions,
artifact magic and the reviewed service artifact digest. Those expectations are
corrected; dependency-complete `check full --fresh` must pass again before normal
mainline integration and selection of a v0.1.76 finalized-archive candidate. Neither
the failed receipt nor the predecessor receipt above proves the corrected source.

Preserve the other worktrees, unrelated stash, original fixtures and failures,
and immutable publication history. No application deployment changes are part
of this work.
