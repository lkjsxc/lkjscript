# Status

Current snapshot: 2026-10-05. This page owns availability and unfinished acceptance.
[Direction](direction.md) owns goals, [specifications](spec/) own semantics and
[roadmap](roadmap.md) orders future language work.

## Public binary

Immutable [v0.1.77](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.77) is
public/latest at accepted product source `2f4936e831a2de1d96ec500e071ac84ad9e8bebf`.
[Producer 37253512076/1](https://github.com/lkjsxc/lkjscript/actions/runs/37253512076)
completed `candidate_accepted`; [promotion 37263457541/1](https://github.com/lkjsxc/lkjscript/actions/runs/37263457541)
completed `immutable_published_and_public_verified` at 04:34:49 UTC.
Release `403368349` and annotation `c3bf76e73e5dd9df1cbdf5f974bbd2c21a62cdeb`
retain that exact source. Anonymous exact, release-ID and latest metadata, all
three asset sizes/digests, and the archived executable passed independent
readback at 04:44:20 UTC. No v0.1.77 publication gate remains.

Original publication evidence is indexed by
`.artifacts/20261005-owned-contracts/release-0177/completed-publication.json` in
`/home/coder/workspace/lkjscript`. Source receipt
`.artifacts/lkjscript-dev/check/1791164478929468490-3238945-0/receipt.json`
retains 26 fresh passing gates, zero reuse and stable inputs at the accepted
product source. The [release notes](releases/v0.1.77.md) describe parameterized
owned contracts. Reporting descendants do not change that tested selection.

## Source-tied borrowed results awaiting acceptance

The v0.1.78 implementation adds [source-tied borrowed results](spec/owned-read-results.md)
through pure named functions and owned-contract methods. Explicit lexical
`borrow-call` scopes preserve exact input provenance and ancestor custody.
Independent source, memory, reference and artifact admission cover the new result
relationship; runtime packets retain cleanup ownership through failed handoff.

The [native selection witness](../examples/owned-read-results/README.md) uses three
packages to select and observe the first maximal element under four storage
implementations, then drain and reuse the original owner. This is a language
witness; production application adoption and compiler self-hosting remain future
work. The [release notes](releases/v0.1.78.md) describe the format cut and required
rebuild of experimental derived bundles.

Development evidence, copied executables, literal public inputs, original failures
and rebuilt native assets are retained under
`.artifacts/20261005-owned-read-results/`. Maintained semantic HEADs and historical
fixture bytes remain unchanged. The four maintained native projects pass 281
tests. A copy containing only tracked native inputs reproduces all four artifacts
and the standard transport exactly, with its 195-pack inventory unchanged;
`cold-tracked-02.log` retains that observation with the corrected runtime. Focused checks are development
evidence; complete fresh source acceptance and exact finalized-archive acceptance
remain required.

All four fresh public read-result cases pass against the copied executable;
`focus-07/public-read-results.log` retains the result. The three-package case
checks complete independent outputs across all four representations, then removes
the authoring projects and transports before detached execution.

Next: freeze source and generated discovery, run all fresh acceptance gates,
then deliver through normal
mainline integration. Dispatch one candidate from that exact accepted source and
require `candidate_accepted` before unchanged-asset promotion and anonymous public
verification. v0.1.78 is not yet a public release.

Preserve unrelated worktrees, stashes, original fixtures/failures and immutable
publication history. No application deployment changes are part of this work.
