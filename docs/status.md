# Status

Current snapshot: 2026-10-07 (Asia/Tokyo). This page owns availability and unfinished
acceptance. [Direction](direction.md), [specifications](spec/) and [roadmap](roadmap.md)
own long-term goals, contracts and future work. External job states are observations.

## Selected v0.1.86: idle worker custody

The selected checkout is `/home/coder/workspace/lkjscript`, on `main`. This change
selects [demand-driven idle worker custody](decisions/idle-worker-custody.md) for
committed-source acceptance and normal integration. The previously tested worktree
is preserved; its receipt is not relabelled as proof for this commit or report.
Exact committed-source acceptance and finalized distribution remain separate gates.

Independent open executors move an idle worker's mailbox, join handle and unreleased
physical reservation together. Local workers remain the first choice. Active work
and unreceived results never move. Failed submission preserves one intact unexecuted
payload; a late failure stays with the current join owner. Closing dispatch
serializes with handoff, so former owners neither cancel nor wait for transferred
work. The bounded weak catalogue is not a permanent program or executor root.

A receipt disposal guard returns active accounting even if destruction of an
unreturned host result unwinds. It does not donate that worker, erase the original
unwind or report successful cleanup. Joined shutdown retains the failure while
still reclaiming the physical worker. Process abort is not recovered.

CLI observation 43 and shared-runtime observation 3 add received/handed-off worker
counts. Before counter saturation, starts plus received equals joins plus handed
off plus remaining. No meaning-graph, package, bytecode, artifact or application-data
encoding changes. There is no data migration, new channel, active-task migration,
CPU-fairness guarantee or total root-plus-child CPU bound.

### Preserved development proof and current-source handoff

Original uncommitted-worktree run
`.artifacts/lkjscript-dev/check/1791325186113628793-3962765-0/receipt.json`
passed all 26 gates fresh with stable inputs. Its receipt digest is
`verification_64f4b33bef8203a504562ba6c516dc75d8649ed41780bb17f8f34e32f7b9651a`.
The tested input digest was
`verification_0875c141cd54b901fc15a545c1309845f84d6b250fbab85e82f986e9a3a1960c`,
rooted at `5aae7b02ebf3c781388ed9317d73d509c7ed6f99` before integration.

That run passed 2,190 workspace tests with zero failures and 29 existing ignored
cases, including 234 public CLI cases. Two filtered child probes are recorded
separately rather than double-counted. The four maintained native projects passed
96 + 44 + 79 + 62 checks with equal production/reference results. Eight regenerated
public documents verified. Focused scheduler and exact-program coverage passed
26 and two cases respectively. A controlled one-worker fixture completed 1,000
alternating dispatches with 999 handoffs; it proves reuse, not throughput or fairness.

The original evidence index is
`.artifacts/20261007-worker-handoff-completion/source-acceptance.json`.
Its `preserved-full-run/` retains the original run outside automatic rotation.
Initial prototypes, the actually failing disposal regression and prior blocked
integration remain preserved, including the original status report. The old tool
preflight failure was not bypassed or converted into a successful operation.

Current integration evidence belongs to
`.artifacts/20261007-worker-custody-integration/`. Before editing this report, all
33 non-status implementation files matched the preserved tested SHA-256 inventory;
the index was empty, no verification/build job was active, and remote main was
independently reread at the base above. Current status bytes and any new committed
source require their own appropriate acceptance. Run the maintained fresh full
checker on stable committed input, then normally push and independently verify
remote ancestry. Never claim a clean-tree `changed` check covers the implementation.

## Previously accepted v0.1.85 and public distribution

Accepted v0.1.85 source is `d1461c863e1b0a507815632616223a27c84772a5`, tree
`17fe1b038c8f1a13c857bb591393bdc737274cd8`. It includes whole prepared-witness sharing
and admission-local shared dependency interfaces; [performance](performance.md)
retains their distinct results. Original source acceptance is indexed by
`.artifacts/20261007-shared-interfaces/source-acceptance.json`; its copied local
product and public harness remain under that directory's `final-local/`.

The last independently established public/latest release is immutable
[v0.1.83](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.83), release
`404980756`, source `65b3d00428d36827f9d65bf92f7900cf52719d21`.
Producer `37452910212/1` and promotion `37502927035/1` completed
`immutable_published_and_public_verified`; originals are indexed by
`.artifacts/20261007-shared-interfaces/release-0183/completed-publication.json`.

## Exact v0.1.85 release-only handoff

[Producer 37521548961/1](https://github.com/lkjsxc/lkjscript/actions/runs/37521548961)
completed from accepted source `d1461c863e1b0a507815632616223a27c84772a5`.
Its original dispatch owner is
`.artifacts/20261007-shared-interfaces/release-0185/candidate-handoff.json`.
Retained workflow/source and artifact observations are under
`.artifacts/20261007-worker-handoff-completion/release-0185/` and its parent.

The retained original terminal and acceptance receipt now read
`candidate_accepted`, contract `lkjscript-final-candidate-acceptance-3`, complete
cleanup and no failure. They select the same exact producer/source/tag. This read
alone is not promotion authority or public availability. Authenticate their service
provenance, refresh occupancy and the existing scoped selector, then promote the
unchanged finalized assets through the maintained controller and publisher. Require
`immutable_published_and_public_verified` and independent public readback. Do not
duplicate this producer or rebuild its accepted bytes.

No v0.1.86 candidate has been dispatched. Serialize later publication with this
still-owned v0.1.85 handoff. Source integration need not wait for public distribution.
Existing stashes, unrelated worktrees, failed originals, immutable publication,
application services and operational data remain preserved.
