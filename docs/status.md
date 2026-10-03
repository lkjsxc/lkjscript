# Status

Current snapshot: 2026-10-03. This page owns availability and unfinished acceptance.
[Direction](direction.md) owns goals, [specifications](spec/) own semantics and
[roadmap](roadmap.md) orders future language work.

## Public binary

Immutable [v0.1.71](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.71) is
public/latest. Accepted source: `200f1512b15e3b5b15147c70332c9566ef06d1f7`.
[Producer 37130051190/1](https://github.com/lkjsxc/lkjscript/actions/runs/37130051190)
completed `candidate_accepted` with joined cleanup;
[promotion 37142421921/1](https://github.com/lkjsxc/lkjscript/actions/runs/37142421921)
completed `immutable_published_and_public_verified` at 18:06:34 UTC.
Release `402629877` is immutable; annotated tag
`90461b95a76e1a2d226653972bdbc885a723754f` binds that exact source.
The unchanged archive, checksum and installer passed independent anonymous public
acquisition, provenance checks and installed lifecycles. The accepted producer was
reused without rebuilding or replaying its completed broad acceptance.

Original producer, selection, publication, public-verification and terminal evidence
is under `.artifacts/20261003-transfer-contracts/release-0171/promotion-01/` in
`/home/coder/workspace/lkjscript-transfer-contracts-20261003`.
The parent release directory's `completed-publication.json` indexes the authenticated
service archives and original proof bindings.
The prior source acceptance remains the fresh 26-gate receipt
`.artifacts/lkjscript-dev/check/1791036974135001923-3943037-0/receipt.json`, digest
`verification_513b173a892e32c366a4b40809e670511a118cbb0f7bd44cacdabc28cb934038`,
in that checkout. Its complete retained copy and original failed attempts remain
under `.artifacts/20261003-transfer-contracts/`. Reporting descendants do not
relabel this exact-source acceptance.

Public v0.1.71 supports explicit transferable contracts for generic libraries that
form their own parallel groups, exact implementation forwarding, mixed owned/data
results and source-free offline composition. See its [notes](releases/v0.1.71.md)
and the [three-package guide](guides/native-transferable-parallel.md).

## v0.1.72 source acceptance and release candidate

Selected checkout: `/home/coder/workspace/lkjscript`, branch `main`.
Accepted source `6a14335e4d57015da0d4663a6ef3b1e49bb321fc` was fast-forward
integrated into remote main and independently verified through Git and GitHub.
Later reporting descendants do not relabel this exact-source acceptance.

The implementation replaces per-child OS thread creation with explicitly owned,
lazily reused auxiliary workers. Nonblocking reservation and caller fallback
preserve nested progress. Jobs retain shared prepared code and sealed custody;
invocation origins, control, quotas and authority remain private. Shutdown closes
new dispatch, drains invocations and joins the worker owner.

The [maintained native workload](../examples/parallel-work/README.md) uses three
ordinary packages and exact ByteBuffer/OwnedI64Cell implementations. Serial,
parallel and nested routes return complete payloads and weighted reductions from
the same methods. The fixed-workload HTTP route runs beside an independent service.
See the [parallel contract](spec/structured-parallel.md),
[shared-runtime contract](spec/shared-runtime.md) and [notes](releases/v0.1.72.md).

Fresh `check full --fresh --jobs 1 --machine` passed **26 of 26 gates**, all fresh,
none reused, with stable source inputs. Original receipt:
`.artifacts/lkjscript-dev/check/1791055313021873408-308116-0/receipt.json`, digest
`verification_9190b0b6ead630a89a885250bfc09a5008eed35a3b2a13f700130fc406345516`.
This includes workspace Clippy/tests, maintained packages and exact artifacts,
generated discovery, copied-executable applications and joined service acceptance.
Workspace results include 1,266 library passes with eight existing ignored entries,
200 public CLI passes with one existing ignored scale fixture, and 253 checker
library passes with 19 existing ignored entries. Both new workload cases passed.
Their command results remain exact after source deletion; each repeated HTTP host
completed six dispatches using four workers and joined every worker.

`.artifacts/20261003-reusable-workers/source-accepted.json` indexes the original
receipt, its complete retained copy, executable identities and performance evidence.
The accepted host executable is byte-identical to the corrected measured copy.
This is source/development-host acceptance, not finalized musl distribution proof.
The earlier full run at `db542d5a` remains failed: 25 of 26 gates passed before a
checker fixture's raw PATH control encountered Linux `ETXTBSY`. The repair uses
the existing joined-child writer pattern without changing production executable
selection or weakening the security assertions. Original failures, including the
separate helper run without Rust on PATH, remain retained under the same evidence root.

[Matched measurements](performance.md#reusable-structured-workers-2026-10-03)
retain all samples and tradeoffs. The shared host completed 14 child dispatches
using four auxiliary workers and joined every worker. Its warm CPU HTTP median was
488.737 ms versus 508.447 ms before; nested command timings and shared idle RSS
increased. The corrected comparison pairs the same post-lifecycle GNU host build
stage. Initial observations from differing dependency features remain retained and
superseded for comparison. These finite observations establish neither general
speedup nor fairness.
[Candidate 37148975781](https://github.com/lkjsxc/lkjscript/actions/runs/37148975781)
was dispatched once at 19:44:19 UTC with event source
`6a14335e4d57015da0d4663a6ef3b1e49bb321fc` and is in progress. Its original
service metadata and handoff belong under `release-0172/candidate-01/` in the
evidence root. The remaining release gate is fresh hosted source/finalized-byte
acceptance, including the required native public workload cases. After authenticated
`candidate_accepted`, refresh publication occupancy, select the exact accepted tag
object and promote the unchanged assets through the existing publication/public
verification terminal. No v0.1.72 assets have been accepted or published; its tag and
publication selection remain untouched. Public/latest remains v0.1.71.

## Compatibility and remaining limits

CLI observation 36 intentionally replaces `parallel_workers_spawned` with
`parallel_worker_dispatches` and adds `parallel_inline_fallbacks`. Shared-runtime
observation 2 and foreground `executor-observation` distinguish physical starts,
active/completed dispatches, fallback and joined worker lifetime. Consumers must
use executable discovery for these fields.

Graph 22, semantic validator 27, authored request 26 for transfer-bearing requests,
package interface 13, compiler 22, bytecode 17 and artifact 29 are unchanged by
worker reuse. Existing supported accepted meaning and artifacts remain readable.
Product-number components remain opaque identifiers. Installation does not migrate
operational data, update exact dependencies or replace running services.

Children still require direct named tasks with closed empty effect rows. Auxiliary
worker bounds do not bound all root-plus-child CPU work or prove fairness,
preemption, speedup or hostile-code isolation. Blocking-capable roots retain their
existing executor. Channels, task handles and asynchronous borrowing remain future
work. The next semantic priority is composing Owned implementation applications
with caller-supplied effects and requirements, followed by reviewed concurrent
candidate refresh with complete dependency and negative-lookup footprints.

Preserve other worktrees, the unrelated stash, original evidence and immutable
publication history. No application deployment changes here.
