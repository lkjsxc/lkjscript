# Status

Current snapshot: 2026-10-03. This page owns availability, selected source and
unfinished acceptance. [Direction](direction.md) owns goals, [specifications](spec/)
own semantics and [roadmap](roadmap.md) orders future work.

## Public binary

Immutable [v0.1.68](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.68) is
public/latest. Exact accepted source: `a7c4222cc669088032689e6620a1082e91478597`.
The [release notes](releases/v0.1.68.md) describe same-task owned calls and task
methods, runtime mailbox custody, and lexical parallel tasks consuming owned inputs
under fresh child identities. Both children join before ordinary results return.
Bounded auxiliary workers use immediate acquisition and caller fallback; quotas
remain invocation-wide. Pure helper loans, products, choices, exact offline packages,
effects and explicit capability resources remain separate contracts.

Producer [37100969378](https://github.com/lkjsxc/lkjscript/actions/runs/37100969378),
attempt 1, completed `candidate_accepted`: 20 fresh source gates, six target owners,
two pinned userlands, 27 native public cases and installed recovery passed with
joined cleanup. [Promotion 37106215025](https://github.com/lkjsxc/lkjscript/actions/runs/37106215025),
attempt 1, controller `96e7bab0c230cc8c4f4ee116ce50a4a423197b47`, completed the mandatory
`immutable_published_and_public_verified` terminal. GitHub release `402389568`
is immutable/latest; all three public asset lengths and digests match the unchanged
accepted assets. Annotated tag object `898b3de0e0ed20dd3f2b0461d63ffe863f8d248a`
binds the exact source. No publication gate remains for this selection.

Original authenticated receipts, assets and controls are retained under
`.artifacts/20261003-owned-parallel-results/release-0168/`. The
[prior mainline handoff](https://github.com/lkjsxc/lkjscript/blob/96e7bab0c230cc8c4f4ee116ce50a4a423197b47/docs/status.md)
retains the fresh 26-gate source result, PR #6 integration, runtime-repair evidence
and superseded producer 37046478616. The completed
[v0.1.64 publication record](campaigns/20261002-owned-choices.md#completed-v0164-publication)
and its immutable assets remain historical originals, not a current selection.

## Development v0.1.69: owned parallel results

The [parallel contract](spec/structured-parallel.md) now permits closed owned child
results. Two ordinary results form the existing record; if either result is owned,
`parallel` returns an OwnedProduct with exact `left` and `right` fields. The parent
uses existing complete decomposition, borrowing and consumption. A sealed result
custodian retains payloads through child-local cleanup and join; parent adoption
retags nested owners without copying their payload allocations. Result-storage
refusal, partial adoption, cancellation and sibling failure dispose of pending owners.

The independent symbolic memory oracle now substitutes imported Owned signatures
before flow checking. Separate exported/imported packages reproduce the predecessor's
rejection of valid calls. New tests cover exact parallel result types, complete closure
admission, allocation identity, joined cleanup and three-package public authorship.
The [release notes](releases/v0.1.69.md) describe the compatibility boundary.

Work is on branch `dev/owned-parallel-results-20261003`, checkout
`/home/coder/workspace/lkjscript-structured-handoff-20261002`, based on main
`96e7bab0c230cc8c4f4ee116ce50a4a423197b47`. Owned-result implementation and maintained artifact
regeneration are complete; this successor has no full-source acceptance or publication
claim yet. Current originals: `.artifacts/20261003-owned-parallel-results/`.
The predecessor oracle failed all four imported regressions at the expected valid-call
assertion. The successor passed six oracle tests, 40 parallel tests and 13 custody
tests (overlapping selectors), including inferred wrapper depth and rehashed artifacts.
All-feature workspace Clippy and generated-page verification passed.

Public check/build regenerated the four maintained artifacts without changing their
accepted HEADs or the exact standard transport. All 274 tests passed differentially.
An isolated copy of 176 tracked inputs plus the four newly generated maintained packs
also passed 274 tests with clean compilation: its 180 input files and 156 pack
path/content records remained unchanged. Original evidence is in `cold-inputs/`.
The separate-package public witness now passes all 16 complete payload, signed-cell
and choice executions, including source-deleted artifacts and the exact edited
canonical definition. Its exported buffer/cell APIs have distinct names. A repaired
literal-editor traversal preserves every body owner through implementation and method
calls; all 14 literal-edit tests pass, including metadata/arity substitution rejection.
The same unchanged public harness failed before this repair and passed with the
corrected host. Originals, 44 retained literal/artifact/input/result files and exact
executable identities are indexed under `public-witness-03/`; earlier failures stay
under `public-witness-01/` and `public-witness-02/`. The superseded verifier build
was cancelled before completion and is not an acceptance claim.
Next gates: fresh full-source acceptance from the fixed input set, mainline integration,
then exact finalized-candidate acceptance/publication.

Preserve the older `.artifacts/20261002-structured-parallel/` and
`.artifacts/20261003-structured-finalization/` evidence, other worktrees and stash.
No running application or deployment is changed by this work.

## Compatibility and remaining limits

Graph 21 and authored request 25 retain their representation. Compiler 20,
bytecode 16 and artifact 27 require rebuilding predecessor derived artifacts from
accepted meaning. Semantic validator 25 renews acceptance; compact discovery 29,
function projection 11 and CLI observations 35 retain their shape. Discover actual
binary support with `lkjscript capabilities`; product-version components are opaque.

The accepted typed meaning graph remains the sole editable program authority.
Installation does not update exact dependencies, migrate data or replace services.
A missing response does not prove rollback or safe replay.

- Direct parallel children remain monomorphic empty-row graph tasks. General
  channels, detached tasks, asynchronous borrowing and cross-instance transfer are open.
- General traits, owned containers, partial moves, field borrows, mutable/escaping
  references and region policies remain incomplete.
- Shared hosting is not a hostile-code sandbox or dynamic supervisor. Worker counts
  and overlap tests do not establish speedup, fairness or arbitrary scale.
- Concurrent semantic publication, branch/merge history and graph/data reclamation
  remain open. Dependencies are exact offline closures.
- Native-code compilation, browser/Wasm and complete self-hosting are future work.
  Rust remains the kernel/tooling implementation; Linux x86-64 musl is the binary target.
- Durable transactions do not establish replication, consensus, encryption or
  automatic migration. Modeled allocation is not RSS or allocator overhead.
