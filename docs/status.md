# Status

Current snapshot: 2026-10-03. This page owns current availability, development scope
and unresolved acceptance. [Direction](direction.md) owns project goals,
[specifications](spec/) own semantic contracts, and [roadmap](roadmap.md) orders future
work. Prior prompts and historical delivery sequences are not required reading.

## Public binary: v0.1.64

Immutable [v0.1.64](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.64) remains
public/latest. Its exact accepted source is
`0048ae1ee2e4678b409c782e02044b038bf60052`; unchanged-asset publication and anonymous
installed verification completed. [Release notes](releases/v0.1.64.md) describe the
binary; the [publication record](campaigns/20261002-owned-choices.md#completed-v0164-publication)
retains exact producer, promotion and asset evidence.

Available foundations include owned buffers and scalar cells, first-order Owned
parameters and explicit implementation witnesses, owned products and recoverable
choices, immutable byte ranges, exact offline packages, effects and capability
resources. Pure helpers provide scoped synchronous loans. Command, HTTP, worker and
session programs run from standalone artifacts; the shared in-process host shares
immutable preparation while keeping instance state, grants and task lifecycles separate.

## Development v0.1.68: structured parallel owned tasks

Development adds `(parallel LEFT-CALL RIGHT-CALL)`: a task invokes two statically
named, monomorphic graph tasks with empty effect rows. Arguments are evaluated once
in parent order, independent owned inputs move to fresh child identities, and both
children join before ordinary `left` and `right` results return. Children can call
generic libraries and exact task-method implementations. Pure callers, borrowed
inputs, capability resources, secrets and dynamic child callables reject.

Bounded worker capacity uses immediate acquisition and caller fallback. Optional
instruction, modeled allocation and collection quotas remain invocation-wide;
failure requests sibling cancellation and joins started work. Sealed transfer
preserves nested owned allocations. Function extraction retains task kind separately
from effect rows, including task-valued invocations. See the [parallel contract](spec/structured-parallel.md)
and [native guide](guides/native-parallel.md) for the exact boundary.

This successor also includes [same-task owned calls](spec/owned-task-transfers.md),
[task methods](spec/owned-generics.md) and explicit runtime mailbox custody in the
[session writer](spec/structured-sessions.md#phases-and-atomic-transition).
Internal mailbox acceptance is distinct from transport delivery or processing completion.

The current audit also addresses canonical artifact correspondence, finite validation,
data codec/store limits, publication and installation durability, authored evaluation
order, and task cleanup. Findings and independent checks are retained under
`.artifacts/audit64-20261002/`; static findings are not execution proof. Development
checks isolate site/documentation and checker-only work while retaining full-source
acceptance for product changes. Campaigns remain historical originals, not required
task prompts.

**Accepted runtime repair is on main:** source
[`d60cf24d5a4f3a82d1725f607545b13d1593485b`](https://github.com/lkjsxc/lkjscript/commit/d60cf24d5a4f3a82d1725f607545b13d1593485b)
was integrated by [PR #5](https://github.com/lkjsxc/lkjscript/pull/5), merge
`3fef605c0a218262ef908d3baa8f713f8ed6dad3`, with the same source tree.
[Full run 37095802455](https://github.com/lkjsxc/lkjscript/actions/runs/37095802455),
attempt 1, passed all 26 gates freshly with stable inputs, no reused or unrun gates.
Receipt: `verification_c6c102511abd27dd2fbf37c3070f8cf2158eda639de2202e2eaaa9a57358a23d`.
The original is retained in that run's source-verification artifact; its controller,
source and tree identities were independently inspected before integration.

The repair retains the originating operational deadline instead of replacing it
with a sibling's generic cancellation. The foreground loop witness calibrates the
same immutable artifact before checking independently expected long-run output,
work beyond the former ten-million-instruction ceiling, bounded-mode equivalence,
fixed live-state maxima and complete cleanup. It no longer assumes nominal field
allocation order. [Focused run 37095977638](https://github.com/lkjsxc/lkjscript/actions/runs/37095977638)
also passed the 11 new policy tests and 1,041 standalone offline commands. Earlier
failed runs and the original accepted development source remain historical evidence;
they are not relabelled as proof of the repair.

**Mandatory native final-byte witnesses are also on main:**
[PR #6](https://github.com/lkjsxc/lkjscript/pull/6) integrated accepted source
`e059e806b60c652001ac93c6eab2f5119328218d` as
`a7c4222cc669088032689e6620a1082e91478597`. An independent fetch confirmed the
unchanged tested tree `d1b8d2eb658626acec01ebd93da2c1eb82787b6d`.
Fresh full acceptance passed 26/26 gates with stable inputs, no reused or unrun
gates and no failure. Workspace tests passed 1,665 cases with zero failures and
29 existing ignored cases; two nested child-test summaries are excluded. The
independent offline owner passed 1,041 commands and completed cleanup.
Receipt: `verification_94dd56045f9e1204947a12c6cdd8f182d5fb112640430bb3bb5092c2aaf0c81c`.

The new source-built public harness enumerated 195 tests and passed all 27 selected
ownership, buffer, range, resident, parallel and interactive-topology cases, with
zero failed or ignored selected cases, unchanged candidate/harness bytes and joined
cleanup. The owner rejects omitted, duplicate or ignored outcomes even with a zero
process exit. Ten independent regressions cover inventory, process, terminal and
real linked-worktree behavior. This copied development-host witness does not stand
in for final musl archive acceptance. No runtime source or product encoding was
changed by PR #6; publication authority and existing acceptance boundaries remain.

Full original: `.artifacts/lkjscript-dev/check/1791005253213055324-2567364-0/`.
Its exact copy is retained outside automatic rotation in
`.artifacts/20261003-structured-finalization/accepted-full-e059e806/`.
Native original: `/home/coder/.local/state/lkjscript-native-evidence-20261003-e059e806/`;
receipt SHA256 `2afc585eb6f54ec46161699a3fac9265ff3e28a06227ef7454722ffca99feb8b`.
The copied candidate and harness identities remain in that receipt and PR #6.
Earlier full-run originals are preserved under the current evidence root's
`retained-original-checks/`; the first failed linked-worktree invocation also remains.
Reporting-only descendants do not relabel the exact source accepted above.

## Current publication handoff

[v0.1.68](releases/v0.1.68.md) remains the selected successor. The corrected normal
candidate was dispatched exactly once from integrated main
`a7c4222cc669088032689e6620a1082e91478597` at 2026-10-03 05:47:42 UTC:
[producer 37100969378](https://github.com/lkjsxc/lkjscript/actions/runs/37100969378),
attempt 1, last observed in progress. This is a non-publishing candidate operation;
no v0.1.68 tag or release has been published. Public/latest remains immutable v0.1.64.

The new producer includes the integrated runtime repair and mandatory native public
harness under [acceptance contract 2](release.md#selected-structured-parallel-successor-v0168).
That owner replaces the manual final-byte supplement, not any source, target,
userland, installation or publication boundary. Resume this exact producer before
dispatching another. After inspecting its original accepted terminal and diagnostics,
use the existing exact-producer unchanged-asset promotion and anonymous acquisition
owners. Source delivery is complete; the external producer is release-only pending.

Earlier producer [37046478616](https://github.com/lkjsxc/lkjscript/actions/runs/37046478616),
attempt 1, accepted source `aa9883f0d073a0d7eaf1f44716ac7e2a5ef5b2fd` before the deadline
repair. It is superseded, not selected for promotion. Its original success, archive
and receipts remain unchanged; the new contract does not relabel that older evidence.

Integration checkout: `/home/coder/workspace/lkjscript-structured-handoff-20261002`,
branch `dev/structured-finalization-20261003`. Current logs, copied tools and dispatch
identity are under `.artifacts/20261003-structured-finalization/`. Preserve the older
`.artifacts/20261002-structured-parallel/` originals and other worktrees/stash. No
running application or deployment was changed.

## Compatibility and authority

Graph 21 retains supported historical meaning readers. Compiler 19, bytecode 15 and
artifact 26 require rebuilding predecessor derived artifacts from accepted meaning.
Authored request 25, compact discovery 29, function projection 11, CLI observations
35 and semantic validator 24 identify current development contracts. Existing
nonparallel authored intent retains its bytes. Discover actual binary support with
`lkjscript capabilities`; product version components are opaque identifiers.

Historical request generations 18–24 now select their original commitment identity.
Review tokens produced by the erroneous newer-codec fallback require fresh planning;
this correction does not claim to preserve those erroneous token bytes.

The typed meaning graph is the sole editable program authority. Native requests,
drafts, projections and compiled products do not become competing sources of truth.
Installation does not update exact dependencies, migrate operational data or replace
running services. A missing response does not prove rollback or safe replay.

## Current limits and unproved properties

- Parallel tasks currently have ordinary results. General channels, owned child
  results, detached tasks and cross-instance transfer remain future work.
- General traits, owned containers, partial moves, field borrows, mutable/escaping
  references, region policies and asynchronous borrowing remain incomplete.
- Shared hosting is not a hostile-code sandbox or dynamic supervisor. Worker counts
  and bounded overlap tests do not establish speedup, fairness or arbitrary scale.
- Dependencies are exact offline closures. General branch/merge history, concurrent
  semantic publication, graph/data garbage collection and compaction remain open.
- Native-code compilation, browser/Wasm execution and other binary targets are not
  established. Linux x86-64 musl is the supported binary target. Rust remains the
  current kernel/tooling implementation; complete self-hosting is a long-term goal.
- Durable transactions and backup/restore do not establish replication, consensus,
  encryption or automatic migration. Work quotas, deadlines and live limits have
  separate meanings; modeled allocation is not RSS or allocator overhead.

[Standard libraries](../packages/standard/README.md), [lkjournal](../applications/lkjournal/README.md)
and native web/editor templates remain maintained consumers. They exercise product
integration without setting language priorities. [Verification](spec/verification.md)
and [performance](performance.md) define evidence boundaries; historical details
remain in existing archives and Git rather than recurring current-work obligations.
