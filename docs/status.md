# Status

Current snapshot: 2026-10-07 (Asia/Tokyo). This page owns availability and unfinished
acceptance. [Direction](direction.md), [specifications](spec/) and [roadmap](roadmap.md)
own contracts and future work. External states below are observations, not promises.

## Accepted v0.1.87: integrated on main

Selected checkout: `/home/coder/workspace/lkjscript`, branch `main`.
Accepted implementation source is `be923bdd843e9c6e460aa58c269e0ae1822ede84`, tree
`9072b61a615f1f493b1f4dcdc5bdd04e296fb60d`, authored as `lkjsxc`.
The ordinary push advanced remote main from `5aae7b02` to this exact source;
independent Git and GitHub API reads agreed. Its parent is accepted v0.1.86 source
`4d7557366c8d552054fbcf5070837038a9cef0c0`, so both custody advances are integrated.
A later status-only reporting child is not a substitute for the tested source.

[Owner-local worker dispatch](decisions/owner-local-worker-dispatch.md) separates
physical custody from receipt-joined availability. Stable local reuse and result
completion take only their current executor's state lock. New starts, cross-owner
handoff and closure retain shared coordination, exact join ownership and the
physical ceiling. Active jobs, unreceived results and failed cleanup never become
available merely because their weak custody entry exists.

Submission refusal retains one intact unexecuted payload. Unwinding cleanup returns
accounting without concealing failure or donating a failed-result worker. A stable
local dispatched pair removes two shared-catalogue acquisitions; this is not
lock-free scheduling, a measured general speedup, fairness, preemption or a total
root-plus-child CPU bound. CLI observation 43, shared-runtime observation 3 and all
semantic, package, bytecode, artifact and application-data encodings are unchanged.
No data migration, new grant, channel or active-task movement is introduced.

## Exact source and local-product acceptance

Fresh full run:
`.artifacts/lkjscript-dev/check/1791365007272928813-814215-0/receipt.json`.
All 26 gates passed fresh, with zero reuse or unrun gates and stable inputs on the
exact source above. Receipt digest:
`verification_2dfb4e65923fd0ae37b695279e2ab36a84654572981fc47166dc7f5bcb60e2fa`.
Receipt SHA-256:
`1a4840675275cd8c65cde4cdd8f692a3250c45b2aba175b3d71584287fdd83ae`.
Initial and final input digest:
`verification_78aec43ff02de117ec446403c46ff06f36a1bf51500c9ce43c958070534498bd`.

Workspace suites passed 2,197 tests, zero failures and 29 existing ignored cases.
The 234 public CLI successes and one ignored case are included; two filtered child
probes are recorded separately, not double-counted. Scheduler-focused coverage
passed 33 cases. Three selected race/locality cases were repeated 100 times each:
all 300 executions passed, including 102,400 concurrent result pairs, 6,400
handoff/shutdown iterations and 100 held-directory regressions. Repetitions are
supplemental evidence, not additional unique workspace tests or a fairness proof.

The exact full-run `release_command_lifecycle` executable independently passed all
67 selected native public cases through the maintained public-harness owner.
The source-selected harness ran outside the checkout with a closed environment,
checked source-deleted execution, and completed cleanup. Candidate and
harness bytes remained unchanged. This is local-product proof, not acceptance of
a finalized distribution archive. Its receipt is
`.artifacts/20261007-local-worker-dispatch/final-local/native-public/receipt.json`.
The copied executable SHA-256 is
`0e4fb7e5b123043675bdf05eb9e78adea244f66f2577a709ab88911a9d3124d5`.

Evidence index:
`.artifacts/20261007-local-worker-dispatch/source-acceptance.json`.
`preserved-full-run/` retains unchanged originals outside checker rotation;
sub-owner evidence remains at its original paths. `repeated-custody/`,
`matched-handoff/` and `final-local/` retain their distinct scopes and input hashes.
The actual predecessor regression and intermediate failed expectations remain
preserved. Reporting-only verification must not relabel the original full receipt.

## Distribution remains separate

The latest reread public release is immutable
[v0.1.83](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.83), release
`404980756`, accepted source `65b3d00428d36827f9d65bf92f7900cf52719d21`.
Its completed publication remains indexed by
`.artifacts/20261007-shared-interfaces/release-0183/completed-publication.json`.

Existing v0.1.85 producer
[37521548961/1](https://github.com/lkjsxc/lkjscript/actions/runs/37521548961) retains
its authenticated `candidate_accepted` terminal for source
`d1461c863e1b0a507815632616223a27c84772a5`. This continuation reread the exact
repository/workflow/source/run/attempt, mandatory successful steps and service
artifacts, and matched retained terminal/acceptance ZIPs against service digests.
There is no matching tag, published release or draft. The current repository
release-immutability setting could not be confirmed from the observed API fields;
existing v0.1.83 immutability is not proof of the setting for a new release.
No tag or scoped selector was changed, and no promotion was dispatched.
Do not rebuild or duplicate this accepted producer. Service asset retention begins
expiring at `2026-10-20T21:58:41Z`; missing trusted material requires renewed proof.

The v0.1.87 candidate request `lkjscript-continue-20261007-candidate-dispatch-c16`
was blocked by tool safety preflight before any command executed. This continuation
created no new producer and used no alternate dispatch route. Current originals
and the distinct remaining gates are recorded in
`.artifacts/20261007-local-worker-dispatch/continuation-boundaries.json`.
The next distribution step is an ordinary permitted candidate dispatch with fresh
source/version/run occupancy checks, followed by exact final-archive acceptance.
Serialize any later publication with the separately owned v0.1.85 handoff and
require the maintained immutable-publication/public-verification terminal.

Existing stashes, unrelated worktrees, failed originals, immutable history,
application services and operational data remain preserved. Mainline integration
and local-product acceptance do not claim production deployment or public binaries.
