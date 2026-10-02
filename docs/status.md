# Status

Current snapshot: 2026-10-02. This page owns current availability, development scope
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
checks now isolate site/documentation and checker-only work while retaining full-source
acceptance for product changes. Campaigns remain historical originals, not required
task prompts.

**Accepted source is on main:**
[`aa9883f0d073a0d7eaf1f44716ac7e2a5ef5b2fd`](https://github.com/lkjsxc/lkjscript/commit/aa9883f0d073a0d7eaf1f44716ac7e2a5ef5b2fd).
Fresh full acceptance passed all 26 gates with zero reused or unrun gates and stable
inputs. Workspace tests passed 1,637 cases, with zero failures and 29 existing ignored
cases; two nested child-harness executions are separate. The source-matched copied
release host also passed the three-package parallel author/edit/detached-execution
witness with a cleared environment. This is development-host evidence, not finalized
distribution-byte acceptance.

Original full run: `.artifacts/lkjscript-dev/check/1790963982625792190-2932798-0/`.
Receipt: `verification_d2da25acbe5a3c72913d850274927b1bece97d0a17fe43e5b12bf383b07bcbd1`.
The copied host and harness identities, public witness, failed predecessor run and
subsequent fixes remain under `.artifacts/20261002-structured-parallel/`; start with
`full-02.jsonl`, `generator-host-05/identity.json` and `stabilization-index.json`.
Reporting-only descendants do not relabel this exact tested source.

Integration checkout: `/home/coder/workspace/lkjscript-structured-handoff-20261002`,
branch `dev/custody-mailbox-20261002`. Current integration logs live in
`.artifacts/20261002-structured-parallel/`; preserve this checkout and its ignored
evidence until delivery and release acceptance finish.

[v0.1.68](releases/v0.1.68.md) is the selected publication successor. Candidate
[run 37046478616](https://github.com/lkjsxc/lkjscript/actions/runs/37046478616), attempt 1,
was dispatched from the exact accepted source above at 2026-10-02 18:17:55 UTC;
last observed queued at 18:18 UTC. Mainline delivery is complete; release acceptance
is pending. Resume this producer before dispatching another. Once it reaches
`candidate_accepted`, run the exact extracted-executable supplement specified in the
[release procedure](release.md#selected-structured-parallel-successor-v0168), then
unchanged-asset promotion and anonymous verification. Public/latest stays v0.1.64
until those boundaries finish. No v0.1.68 tag or release has been published.

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
