# Current status

Snapshot: 2026-10-10. Product identifiers are opaque. Accepted meaning, complete
source verification, mainline delivery, finalized executable acceptance and public
availability remain separate boundaries.

## Current increment: generalized owned sequences

Work is in `/home/coder/workspace/lkjscript-generalized-sequences-20261011`, branch
`work/generalized-sequences-20261011`, based on `b9cacb40`. The combined increment
includes the pending native FIFO work, ordinary first-order sequence elements,
immutable indexed reads, custody-preserving replacement and dense working storage
in the bounded native dependency planner. The [sequence specification](spec/owned-sequences.md),
[data guide](guides/native-data-sequences.md) and [release notes](releases/v0.1.90.md)
own the semantics and intentional format cut.

The accepted standard revision is
`rev_55fb7bfafcef4ec97b158b23e5f8d49c244e8cf64d32786fa81448f136effc59`.
Public plan/apply, check, build and export produced 2,796 live owners, 290 compiler
units and **116 passing graph tests**, with agreement between the production and
reference evaluators. The maintained application, guide renderer and policy oracle
also rebuild through public check/build: **44**, **79** and **62** tests pass,
respectively, with their accepted HEADs and exact suppliers retained.

Fresh copied-product authoring accepts the generic data-sequence library before
its concrete consumers. Four focused public tests pass, including **3,110** finite
operation-word comparisons, immutable snapshots, shared reads, transferred owners,
displaced owned values, bounds, fuel refusal and recovery. The rebuilt public
rejection test passes all 18 cases without changing accepted revisions. All three
resident-policy cases pass with the expanded standard and joined cleanup.

Focused compiler/runtime testing exposed an independent source-checker omission:
new get/replace operations and ordinary push values must trigger ownership-flow
checking before source reuse. The correction retains the hostile tests and adds a
push-value/source-consumption adversary. Diagnostic and obsolete rejection fixtures
are also corrected. The updated workspace type-checks; **101 sequence tests** and
**19 release-inventory tests** pass. The complete planner matrix passes **18,540**
attached/detached comparisons plus refusal and recovery checks. The independent
verifier also passes its complete attached/detached plans, claims, exhaustive
candidate and mutation matrices, with refusal, recovery and cleanup intact. The
matched cost study exposed repeated generic type-table searches: large stage
cases were 16–18 times slower. Those unfavorable originals remain retained. A
bounded, program-bound lookup now reuses composite substitutions already proved
during preparation, while checking full bindings and current type shapes at use.
The identical 180-execution study now passes all result and cleanup checks on the
updated product. Large stage cases remain 2.5–2.7 times slower than the map/list
baseline; nontrivial complete plans remain 5–17% slower. Both original and updated
comparisons, exact identities and modeled metadata costs are retained in the
[planner owner](../examples/dependency-plan/README.md). This is a language capability
advance with a measured performance cost, not an immediate speedup claim.

A broader preflight exposed stale generation, golden-fixture and generated-document
expectations. Current controls are added without changing historical fixtures;
developer-tool inventories and the maintained service pin are updated too. The
rebuilt suites now pass **1,762 core tests** and **285 developer-tool tests**, with
zero failures and 8/19 intentional ignores. This includes eight new lookup tests
for scope, origin, type integrity, bounded work, reservation and cancellation.
Workspace lint passes with all targets and features enabled.

Evidence is retained under `.artifacts/data-sequences/` in the current worktree.
Failed and cancelled attempts remain distinct from successful checks. Maintained
artifacts and reference pages are regenerated through their product owners; no
privileged graph writer or external semantic generator is used.

## Predecessor verification and integration

The original FIFO full run on `b9cacb40c44a20d1d090bded9581d7e29293e71f`, tree
`27f455e1`, finished with **25/26 fresh gates**, zero reuse, zero unrun gates and
stable inputs. Workspace results were **2,314 passed, 4 failed, 29 ignored**, apart
from two separately filtered child probes. All five FIFO public witnesses passed.
The failures were two stale standard counts and two test-local allocation ceilings
below the expanded standard's complete type-metadata cost.

The retained receipt is
`verification_0784e1cd3bbe8b0253cb062fdc1d29e412fd04e2532fea11bcdad74075beb183`.
Originals remain in the FIFO worktree under
`.artifacts/native-fifo/resume-20261010/full-b9cacb40/`.
Corrections `a2757ebf` and `32594261` pass focused checks **2/2** and **3/3**.
Resident fixtures derive a finite allowance from a tiny successful workload plus
64 KiB; the heavy workload still refuses and the small request recovers under that
same allowance. Production limits are unchanged. These corrections will enter the
combined source acceptance; no duplicate intermediate full run is selected.

The last corroborated remote main is
`574a8b5783077cb0654ecd85ede94105cb6912a1`, the normal merge of the previously
accepted source-verification correction. PR #14 remains a draft with remote head
`5c86cd33`. The combined sequence increment has not yet been delivered to main.

## Public binary

Development remains **0.1.90**. The latest observed immutable public release is
**v0.1.89**, release **407859944**. The v0.1.90 tag is unoccupied. Earlier candidate
`38034392926/1` failed source verification; no finalized bytes from it were accepted
or promoted. No successor candidate has been dispatched in this increment.

Next: include the correction ancestry, freeze the complete source, complete one
fresh 26-gate source acceptance, integrate normally and verify remote ancestry.
Then accept finalized static bytes and promote
those unchanged assets through the configured release workflow. Application
selection, deployment, operational data and unrelated resources remain unchanged.
