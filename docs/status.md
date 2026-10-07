# Status

Current snapshot: 2026-10-07 (Asia/Tokyo). This page owns availability and unfinished
acceptance. [Direction](direction.md), [specifications](spec/) and [roadmap](roadmap.md)
own contracts and future work. External states below are observations, not promises.

## Selected v0.1.87: owner-local worker dispatch

Selected checkout: `/home/coder/workspace/lkjscript`, local branch `main`.
The successor builds on accepted local source
`4d7557366c8d552054fbcf5070837038a9cef0c0`, tree
`1f55c20bd3fc846b2b5be969f8e88e01340e093c`, without discarding the inherited report
or original verification. It selects
[owner-local worker dispatch](decisions/owner-local-worker-dispatch.md).
The exact successor commit, fresh full-source acceptance and normal remote
integration remain separate gates. No successor distribution candidate is selected.

### Capability and compatibility

Physical worker custody is represented by one weak owner/slot directory entry.
Receipt-joined availability is private to its current owner's locked state. A
local hit and its result completion no longer acquire the shared directory. A
miss drops its local lock before entering shared custody and rechecks closure and
local availability. New starts, cross-owner handoff and closure retain the shared
lock ordering and sole join owner. Active jobs and unreceived or failed results
cannot move merely because their custody is listed.

Submission refusal preserves an intact unexecuted payload. Parent unwind and
failed result disposal retain their existing joined-cleanup and error ownership.
The physical ceiling, closed-dispatch fallback, private cancellation and per-owner
accounting are unchanged. CLI observation 43, shared-runtime observation 3,
semantic graph, package, bytecode, artifact and application-data encodings are
unchanged. No data migration, new grant, channel, root CPU scheduling, fairness,
preemption or active-task movement is introduced. Removing shared-lock acquisitions
from a stable local pair is not a measured speedup or lock-free execution claim.

### Source acceptance handoff

Current originals belong to `.artifacts/20261007-local-worker-dispatch/`.
The predecessor's actual held-catalogue regression fails after releasing the
obstruction and joining its caller; the immutable source-selected test executable,
fixture and log remain preserved. Intermediate source/test outcomes are retained,
not converted into successful acceptance. The successor requires all new locality,
refusal, disposal, unreceived-result and concurrent-custody cases, the existing
scheduler/exact-program cases, and fresh dependency-complete full acceptance.

After generated inputs stabilize, make the ordinary source commit and run the
maintained fresh full checker on that exact source. Preserve its immutable receipt,
input identity, gate outcomes and executable owners. A later reporting-only
revision must not relabel that proof. Normal permitted mainline integration requires
fresh remote ancestry and independent readback; a local commit is not delivery.

### Retained predecessor

Inherited v0.1.86 source `4d755736` passed all 26 gates fresh, with stable inputs,
2,190 workspace successes, zero failures and 29 existing ignored cases. Its receipt
is `.artifacts/lkjscript-dev/check/1791358037632989726-527958-0/receipt.json`, digest
`verification_7a9c18578a018835a2c1396a39f4b4b89d1636cdae8ade0077e5aba400b7b9fb`.
Its preserved acceptance and race evidence remain indexed by
`.artifacts/20261007-worker-custody-integration/source-acceptance.json`.
Those results do not attest the v0.1.87 successor.

The previous ordinary push was blocked before execution. At this turn's initial
independent Git and API read, remote main still selected
`5aae7b02ebf3c781388ed9317d73d509c7ed6f99`; local source `4d755736` was not integrated.
The inherited uncommitted status report is preserved verbatim as
`.artifacts/20261007-local-worker-dispatch/inherited-status.md`. Previous tool
boundaries remain at their original evidence owners; no alternate write path,
force-push or protection change substitutes for normal permitted integration.

## Public binary and separately owned predecessor candidate

The public/latest release was reread as immutable
[v0.1.83](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.83), release
`404980756`. Its accepted source is `65b3d00428d36827f9d65bf92f7900cf52719d21`.
Producer `37452910212/1` and promotion `37502927035/1` previously completed
`immutable_published_and_public_verified`; original evidence remains indexed by
`.artifacts/20261007-shared-interfaces/release-0183/completed-publication.json`.

The previously authenticated v0.1.85 producer is
[37521548961/1](https://github.com/lkjsxc/lkjscript/actions/runs/37521548961), selecting
`d1461c863e1b0a507815632616223a27c84772a5`, tree
`17fe1b038c8f1a13c857bb591393bdc737274cd8`. Its retained terminal is
`candidate_accepted` under `lkjscript-final-candidate-acceptance-3`, not public
availability. The prior publication preflight was blocked before execution;
`.artifacts/20261007-worker-custody-integration/release-0185/` retains its authenticated
handoffs and exact boundary. Do not duplicate this producer or rebuild its assets.

Finish that existing candidate's ordinary permitted publication path independently.
Keep any later v0.1.87 candidate serialized with this handoff and require exact
final-archive acceptance before a distribution claim. This source-development turn
has not changed a workflow dispatch, tag, scoped publication selector or public asset.
Existing stashes, unrelated worktrees, failed originals, application services and
operational data remain preserved.
