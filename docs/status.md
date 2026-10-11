# Current status

Snapshot: 2026-10-11. Source acceptance and mainline delivery are complete for
**0.1.92**. Finalized executable acceptance and public availability remain separate
boundaries; the selected release candidate is running.

## Accepted mixed storage contracts

Owned contracts and generic implementation schemes now admit ordinary
transferable data parameters. Self remains exactly Owned; ordinary category,
transferability and capture safety are checked independently, including unused
parameters and nested phantom arguments. The native `IndexedData` library has
Flat and Chunk32 implementations; Chunk32 is a public nominal record with private
fields. The dependency planner selects integer and Boolean storage independently
in all four combinations. See the
[release notes](releases/v0.1.92.md),
[contract specification](spec/owned-contract-parameters.md) and
[native storage example](../examples/indexed-data-storage/README.md).

The exact accepted source is
**`a7a3d778fe34df5ed9a8f0fb5595fbf2f858c1a5`**, tree
`2ba8b337e5ba2bf9b8d333bcf2aa2310b390eb8d`. It was integrated by ordinary
fast-forward into remote main, with independent commit, tree and ancestry checks.
This status update is a reporting-only descendant; acceptance and the release
producer continue to bind the exact source above.

The selected worktree is
`/home/coder/workspace/lkjscript-mixed-storage-20261011`, branch
`work/mixed-storage-contracts-20261011`. Its complete fresh verification owner
passed **26/26 gates, zero reused**, with stable initial/final inputs. Workspace
tests passed **2,494**, failed **zero**, and ignored **29**. Source proof includes
the complete public suite, independent planner/checker coverage, release build
and lifecycle, distributed/runtime checks, maintained native asset regeneration,
and exact generated-asset comparisons. The original receipt is
`.artifacts/lkjscript-dev/check/1791710687727540908-1577338-0/receipt.json`, digest
`verification_cfea3bcf70b65f0d107df6e18bcabf5edb8e9e5c8e2e9af7535633bb5e172f86`.
Compact source and integration bindings are in
`.artifacts/mixed-storage/delivery/source-acceptance.json` and
`integration-receipt.json` in the same delivery directory.

All three focused copied-product storage tests also pass, including 104
differential invocations and 98 production invocations after source deletion.
Planner and independent checker suites pass 3/3 and 9/9 tests respectively; each
covers 55,620 complete plans across both prior component carriers and all four
storage combinations, attached and detached. The checker rejects exhaustive
false claims and targeted mutations, with joined cleanup and recovery. The full
fresh workspace run repeats both suites successfully.

Matched storage measurements pass all 480 serial executions across 20 workloads,
with 60 warmups and 420 measured samples. Complete results, joined cleanup and
asset identities hold. On this shared host, indexed Flat/direct invocation ratios
were 1.111–1.241 and Chunk32/Flat ratios were 1.364–3.392. Preparation and invocation
remain separate observations; no portable speedup or API-cost saving is claimed.
The native example records the measured table, executable/artifact identities and
original evidence at `.artifacts/mixed-storage/costs/measurement-first/`.

Original build, test, native-asset and copied-product evidence remains under
`.artifacts/mixed-storage/`, including failed attempts separately from corrected
runs. Full acceptance and referenced owner evidence, 4,147 retained files, are
archived with individual hashes alongside an exact accepted-source snapshot in
`.artifacts/mixed-storage/proofs/full-source-a7a3d778-v2/summary.json`.
Archive comparison and checksum verification passed; originals remain intact.
The separate native archives are indexed by
`.artifacts/mixed-storage/proofs/native-indexed-data-summary.json`;
`planner-second.tar` and `planner-verification-second.tar` remain in the same
proofs directory.

The intentional compatibility cut selects Graph 33, interface 21, compiler 33,
artifact 40, authored request 37 and semantic validator 38. Bytecode remains 27.
Supported older meaning retains its original owner generations and interface
projection; new mixed declarations and invocation expressions require their
applicable Graph 33 boundaries. Maintained derived artifacts are regenerated
through their native owners. Finalized-candidate contract and workload generation
9 require the complete new indexed-storage public witness family.

## Release handoff

Exactly one non-publishing v0.1.92 candidate was dispatched:
[38130920573/1](https://github.com/lkjsxc/lkjscript/actions/runs/38130920573).
Independent API reads authenticate the official repository and head repository,
workflow **343592338** (`.github/workflows/release.yml`), `workflow_dispatch`, main,
and exact source **`a7a3d778fe34df5ed9a8f0fb5595fbf2f858c1a5`**. At 10:12 UTC,
candidate admission was in progress, building immutable host tools. Finalized-byte
acceptance and publication are not yet established. Original preflight, dispatch
and API evidence, plus the compact run selection, are retained at
`.artifacts/mixed-storage/delivery/candidate-handoff.json`; the latest read-only
observation is in `delivery/reporting-preflight-20261011T101158Z/` under that same
mixed-storage evidence root.

The remaining external gate is generation-9 **`candidate_accepted`** from that
exact run and original attempt, with authenticated original assets, verifier and
essential acceptance handoff. At the next meaningful work boundary, inspect this
run and reuse it while healthy. If it passes, continue the configured exact-source
tag/control selection, unchanged-asset promotion and required public-verification
terminal under [the release procedure](release.md). If it fails, preserve and
diagnose its original terminal evidence before selecting any corrective action.
Do not duplicate the candidate or rebuild accepted assets during promotion.

The latest verified public release remains **v0.1.89**, immutable release
**407859944**. Its scoped control remains unchanged. No v0.1.92 tag, control change
or promotion has occurred.

The failed v0.1.91 producer
[38116583723/1](https://github.com/lkjsxc/lkjscript/actions/runs/38116583723) is
superseded by the accepted and integrated 0.1.92 source. It failed at the
1,800-second native-public execution deadline after 42 of 96 cases completed,
with no assertion failure observed; it never reached finalized-byte acceptance
or publication. Its original diagnostics remain in
`/home/coder/workspace/lkjscript-generalized-sequences-20261011/.artifacts/owned-nominals/delivery/failure-38116583723-1/`.
The successor grants only that serial execution aggregate 120 minutes; the
separate build allowance remains 30 minutes, with complete coverage and cleanup
still required. Do not retry or promote the failed predecessor. Its original
source acceptance remains bound to
[the previous status](https://github.com/lkjsxc/lkjscript/blob/0eaae23d2cd76f2b78b982170fd357fa08c8f8f5/docs/status.md).
Retain both worktrees and their ignored evidence through release closure.
