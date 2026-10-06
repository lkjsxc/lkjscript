# Status

Current snapshot: 2026-10-07 (Asia/Tokyo). This page owns availability and unfinished
acceptance. [Direction](direction.md) owns goals, [specifications](spec/) own semantics
and [roadmap](roadmap.md) orders future language work. External workflow states below
are observations, not a claim that independently running jobs cannot advance.

## Shared dependency interfaces accepted on main

The selected checkout is `/home/coder/workspace/lkjscript`, on `main`. Accepted
v0.1.85 source `d1461c863e1b0a507815632616223a27c84772a5`, tree
`17fe1b038c8f1a13c857bb591393bdc737274cd8`, was normally pushed and independently
confirmed through Git and the GitHub API. This page is a reporting descendant,
not a replacement for that tested source.

[Admission-local shared dependency interfaces](decisions/shared-dependency-interfaces.md)
materialize each needed exact public-interface record map once per source admission.
Importing snapshots and snapshot clones share those immutable records. The pool is
bound to one admitted interface inventory and releases its own references before
return. Snapshot-private type maps and application state remain separate.

A shared record map is not a validation certificate. Every load still independently
admits interfaces and canonical private bodies, compares interface/body agreement,
checks direct visibility and reconstructs the complete composed callable closure.
Equal names, Self types or contract shapes do not merge exact revisions or selected
implementations. Corrupted input rejects even after an earlier successful load.
The independent source oracle continues reconstructing its own maps.

Incoming edges, new map entries, record copies and remaining per-snapshot type
copies retain the existing aggregate budget. Exact-fit and one-below tests verify
reservation before growth; an incomplete admission does not publish readiness.
Copy-on-write fixtures cannot modify another snapshot, and the last retained
snapshot releases its shared records. No limit or authority was widened.

### Measured effect and composed public use

The matched eight-wrapper source-admission workload reduces actual interface record
copies from 774 to 86 (88.89%). Aggregate validation visits change from 327,446 to
326,016 (0.44%); the same 279 type copies and 208,237 validation-read bytes remain.
Both versions admit the exact serialized containers retained from the v0.1.84
predecessor, with unchanged output container bytes. Tiny interfaces add bookkeeping.
These are not RSS, total live heap, runtime-speed or API-cost measurements; the
retained deepest timing sample is slightly slower. [Measurements](performance.md#admission-local-shared-dependency-interface-records)
retain the limits and unfavorable observations.

The [four-package public workload](guides/native-shared-dependencies.md) exports a
recursive generic supplier before two independently authored private readers exist.
A consumer borrows one cell through both exact readers, then consumes the owner.
It preserves full I64-extrema results and canonical re-entry, refuses a malformed
call without changing accepted HEAD, and runs after deleting all four disposable
authoring projects and their transports. The predecessor also passes this functional
workload: the improvement is reduced duplication, not newly invented reader semantics.

This source includes v0.1.84's [whole prepared-witness sharing](decisions/shared-prepared-witness-nodes.md),
accepted at `ea6d5729431e426268a391b8e4a47a115cc5775b`. That earlier matched 24-layer
fixture reduced its calculated retained node/slot subset from 26,400 to 7,728 bytes.
Its distinct measurements and original acceptance remain under
`.artifacts/20261006-concrete-closure/source-acceptance.json` and
[performance](performance.md#whole-prepared-witness-node-sharing). Concrete DAGs and
compatible shared instruction bodies already existed; the newly maintained full
24-layer recursive consumer also passes on v0.1.83. Neither result is a claim that
all logical witness paths execute cheaply or that total process memory falls equally.

No meaning-graph, package-interface, type-object, bytecode or artifact encoding
changed. There is no application-data migration, global cache or language-level
reference-counting commitment. Shared type tables, cross-operation reuse and
remaining source-validation costs are separate follow-ups, not completed features.

### Original acceptance and exact local product

Original full acceptance is
`.artifacts/lkjscript-dev/check/1791308022373689585-3440901-0/receipt.json`:
26 fresh passing gates, no reused or unrun gates, and stable inputs. Its digest is
`verification_1a6e0fcf72a627fb73b04a79c4be7269072836747d3fce11b175bc2dc6445728`.
The workspace suite has 2,174 passing tests, zero failures and 29 pre-existing
ignored tests. This excludes two filtered subprocess replays. The public CLI suite
has 234 passing tests and one ignored test and is included in that total. All seven
focused dependency-interface tests pass, including independent reconstruction,
reclamation, corrupted-input and exact-budget checks.

The original receipt names actual parent
`bcc710b3f86d6adb23367c24ef8438609459000a` and stable worktree digest
`verification_48016fda5b27c87be36a72c98532d1c185782313f945dbb5411b9481520b821a`.
Before committing, all 1,779 frozen file SHA-256 values and all 1,786 input
kind/mode/length entries were independently rechecked. All 44 changed paths belonged
to that original input inventory. The committed tree equals the reviewed prospective
tree; the original receipt is not relabeled as a different source. Original logs,
failed development attempts, source mapping and remote observations are indexed by
`.artifacts/20261007-shared-interfaces/source-acceptance.json`.

The full command-lifecycle gate's retained executable has SHA-256
`ad622597a69d821bfc0f0bb0bc0274d9509cf9f6c1474c93c3025a469e6b0b7e`.
Its source-selected public harness has SHA-256
`75c334b4a507575bb6e1d456353deebc80bb6bdc8c742fd960db2116bf8db3b9`.
The new diamond case passes again against copies outside the checkout with an empty
credential environment. Product and harness are re-read unchanged. These bytes also
match the separately retained preflight copies. Originals remain under
`.artifacts/20261007-shared-interfaces/final-local/`. This is exact local-product
acceptance, not acceptance of a finalized release archive.

## Public binary

The independently re-read public/latest release is immutable
[v0.1.83](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.83), release
`404980756`, at source `65b3d00428d36827f9d65bf92f7900cf52719d21`.
[Producer 37452910212/1](https://github.com/lkjsxc/lkjscript/actions/runs/37452910212)
completed candidate acceptance. [Promotion 37502927035/1](https://github.com/lkjsxc/lkjscript/actions/runs/37502927035)
completed `immutable_published_and_public_verified`; its terminal records authorized
promotion, successful publication/public verification and selected latest.
Publication occurred at 2026-10-06T17:29:23Z (2026-10-07 02:29:23 JST).
Annotation `65d1c4e43776015eae510b2f895357040c28f246` retains that exact source.

The original terminal, authenticated workflow observations, anonymous exact/latest
and release-ID readbacks, unchanged asset lengths/hashes and completion index remain
under `.artifacts/20261007-shared-interfaces/release-0183/`. Its
`completed-publication.json` indexes those originals. This completed v0.1.83
publication does not validate successor source or distribution bytes.

## Exact v0.1.85 candidate remains pending

[Candidate 37521548961/1](https://github.com/lkjsxc/lkjscript/actions/runs/37521548961)
was dispatched exactly once at 2026-10-06T19:47:27Z from accepted source
`d1461c863e1b0a507815632616223a27c84772a5`. Repository, head repository, workflow,
event, branch and original attempt were independently re-read. It was in progress
at `Fetch locked dependencies and build immutable host tools`. No v0.1.85 tag
existed at dispatch; final-archive acceptance or immutable publication is not yet
established here. The exact resumption owner is
`.artifacts/20261007-shared-interfaces/release-0185/candidate-handoff.json`.

Authenticate this original producer's `candidate_accepted` terminal and retained
artifact identities before promotion. Reconcile the completed v0.1.83 publication
and current scoped tag selector; annotate only the exact accepted product source,
then use the maintained publisher on unchanged finalized assets. Require
`immutable_published_and_public_verified` and independent public readback. Do not
rebuild accepted assets, dispatch a duplicate producer or treat a green subjob as
publication. Missing, invalidated or expired originals require renewed proof.

The accepted v0.1.82 producer remains historical, not a selected publication target.
No v0.1.84 producer was created; its mainline changes are consolidated into v0.1.85.
The source and local acceptance are complete independently of this release-only
waiting. Existing stashes, unrelated worktrees, failed evidence, published immutable
history, application services and operational data remain preserved.
