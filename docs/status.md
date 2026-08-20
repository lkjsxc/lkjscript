# Implemented status

This file describes the current checkout. Normative contracts under `docs/spec/` and executable
validation outrank this summary. Product-specific `lkjwork` and `lkjstudio` behavior is owned by
their application READMEs and checked semantic projects.

## Semantic development repository and CLI

One discoverable semantic project owns one validated typed graph, first-class build-target graph,
selected HEAD, full immutable revision snapshots, and canonical compact revision records. The strict
`.lkjscript/project` marker binds one workspace. Explicit relative/absolute selection overrides
ambient discovery; parent traversal, symlinked or nonregular markers, nested ambiguity, foreign
workspace bindings, malformed state, and corrupt history reject.

The public CLI implements init, orient/known-digest, status, historical inspect, closed paginated
semantic queries, packet-free function proposal rendering, targeted context, JSON and document
validate/apply, log, record show, endpoint diff, forward restoration, target list/show/test/build/run,
shallow/deep doctor, no-replace backup, and a correlated foreground session. Public machine results
use project envelope 2. Ordinary work discovers workspace/revision facts; callers do not supply
internal repository paths.

Every accepted mutation publishes exactly one immutable revision and one record. Validate-only,
malformed, foreign, stale, excessive, invalid, rejected, and semantic-no-change requests publish
nothing. The owner preflights candidate validation, every target, artifact preparation, and both
response projections before publication. A commit-only idempotency key fingerprints response
projection and replays one exact retained receipt after restart. No stale mutation is retried.

Published receipts include change-continuation 1: new revision/snapshot/record, accepted-change and
semantic-diff digests, requested created bindings, changed functions, affected targets, and explicit
local-alias invalidation. It is bounded to 64 KiB and is an observation, not durable local identity.

Semantic-query 1 has closed `summary`, `exact`, `children`, `function`, `owner_chain`,
`dependencies`, `incoming_uses`, `callers`, `callees`, `targets`, and `blockers` projections. Queries
bind one exact revision, deterministic roots/order, work/page/output bounds, result digest, and an
opaque plan-bound continuation. There is no general query language, index, cache, or concurrent
snapshot-reader path. All project operations conservatively hold one engine lock.

## Build and distribution

Durable target kinds are reusable release, application, and native product. Target identity and
edges use durable graph IDs; names are revision-bound lookup metadata. Every accepted candidate
eagerly validates every target through the full oracle. `target build` deterministically lowers one
exact revision and publishes one synchronized no-overwrite artifact. Build/test/run never publish a
development revision. There is no arbitrary hook, build script, ambient file input, mutable
coordinate, derived cache, or hidden generated binding.

Release format 2 and application format 8 are immutable distribution authorities. Application 8
supports typed, byte-stream, stateful, and interactive profiles. Interface self-description remains
contract 1. Interactive profile 2 binds exact application-owned initialize/update/resume/render
roles, closed event/action/outcome values, and full frames. Headless replay 3 exercises the same roles
for up to 10,000 deterministic events.

## lkjstudio semantic workbench

`applications/lkjstudio/.lkjscript` is the sole maintained program/build authority. It is workspace
`6ee361b40e2ce5041d64321d79c3db0d`, revision 48, snapshot
`12898095ee151d9d0c6f46fdbd17838ed88febd17533c6c6badb731b1f4cf83e`, record
`79837c3208dadc34e941192c54c1fcb2252260fe9b3d657055e09ee9ed1a3961`, with 3,301 graph nodes,
359 durable identities, 2,942 function-local references, three targets, 29 passing target cases,
and no blockers. Its 49 accepted revisions (including genesis) were created through public project
commands. Revision 46 is the retained self-change: host actions moved to Alt so Ctrl-N/W/Y/Z remain
semantic editor commands. Revision 47 replaced application-level scalar-copy loops with checked
sequence slice/concatenate, and revision 48 raised only the interactive target's deterministic fuel
policy to the already bounded 100,000,000 maximum required by the 65,536-scalar paste workflow.

The checked 161,562-byte `lkjstudio.lkja` has file SHA-256
`d7c89b503a8b3ca882495919105812ebf38735dd3e019daf2127f6b6cdd9e039`, application digest
`74597b6fb8fbc38dd2d6191f979fe8e380af32de6bb5a085812dac436fe4b683`, graph digest
`39e1d780bd08cd76562fe37ab308cb47696c7b67851da6d1aca8ef67cba7fe85`, and root release
`9ba6554e46709b5148236c55f37ec410b3d3ede6449e749b84429d71bb3675cc`.

Application meaning owns buffer allocation/nonreuse, active/order state, scalar-indexed content,
cursor/selection, insert/delete/replace/movement, line movement, select-all, 32-entry bounded
undo/redo, exact literal search, resize, exit, keymap action selection, one-pending-action state,
outcome transitions, status, and frame content. Restart loses unsaved foreground state. Editor undo
is neither semantic history nor filesystem rollback.

The explorer actions cover orientation, children, exact function, callers/callees, targets,
blockers, history, record, and diff. Proposal actions render a selected function document, validate
without publication, apply once, and preserve a stale draft on conflict. Target actions list, test,
build, and run. The native adapter invokes the same `Project` owner and has no private-store mutation
path.

Terminal contract 3 decodes a closed key/paste/resize vocabulary, performs bounded Unicode-width
full-frame rendering, and owns raw/alternate-screen/bracketed-paste/cursor lifecycle. Cleanup is
idempotent and tested across staged acquisition/output failures, unwind, normal exit, signal, EOF,
and disconnected pseudo-terminal input. The application cannot emit ANSI.

Selected-filesystem 1 pins one Linux directory descriptor and confines UTF-8 relative components
with `openat2` beneath/no-magiclink/no-symlink/no-mount-crossing policy. It provides deterministic
digest-bound directory pages, stable regular-file read, no-replace create, expected-content atomic
replace, explicit conflict/known-failure/unknown-visibility outcomes, and independent reconciliation.
No possibly visible write is repeated.

## lkjwork dogfood

`applications/lkjwork/.lkjscript` is workspace `6894b57786a7e1ef14370d2da3a3cf33`, revision 9,
snapshot `4ae531e41c93b69583ee14c44837ce1d5f6748e427d3a02f60c7164b2174eec1`, record
`fb28346946daef184262e06781ee42c537913d63ba313ef50834744458f30561`, with 3,353 graph nodes,
547 durable identities, three targets, eight passing cases, and no blockers. Revisions 8 and 9 are
the new cross-cutting dogfood feature: application-owned summary reports blocked nonarchived planned
tasks; target cases and native rendering consume it.

The checked 170,545-byte artifact has SHA-256
`dbd9e2e3ec63e6291978ba0409428d75a657decc701bc9959b23ce893b8d26eb` and application digest
`53c52e0a513c1b03b3c57f131aa372d64aea6cc04f07f0857fe24cd0fe45f985`. The prior `why` feature,
complete work-ledger lifecycle, pure queries, durable instance journal/checkpoints, immutable blob
grant, backup/restore, doctor, strict JSON, and foreground session remain active.

## Active identities and rejected predecessors

| boundary | active identity | rejected predecessor |
|---|---|---|
| raw workspace protocol / schema | 13 / `lkjscript-machine-schema-v13` | protocol 12, schema v12, and older |
| semantic project / marker | 1 / `LKJPROJ1` | every other version |
| project change / machine / session | 2 / 2 / 2 | version 1 |
| semantic query / change continuation | 1 / 1 | every other version |
| workbench / context / edit document | 2 / 2 / 2 | context/document 1 and `plan` |
| revision record | 1 / `LKJREC01` | every other version |
| workspace artifact / HEAD | 8 / `LKJTSM\0\x08` / `LKJHDA10` | format 7 / `LKJHEAD9` and older |
| target / release | 1 / 2 (`LKJREL\0\x02`) | every other target kind/version; release 1 |
| application / interface | 8 (`LKJAPP\0\x08`) / 1 | application 7 and older |
| interactive / headless / terminal | 2 / 3 / 3 | every other current-execution version |
| project host / workbench host / selected filesystem | 2 / 2 / 1 | every other version |
| instance / runtime session | 3 (`LKJINS\0\x03`) / 2 | instance 2; other runtime-session versions |

There are no editions, compatibility artifact readers, dual success paths, migration modes, old
command aliases, or builder fallbacks. Immutable historical project snapshots retain their exact
validation route; that history reconstruction cannot produce or execute a predecessor artifact.

## Trust, limits, and explicit absences

The verified deployment is Linux x86-64 under one trusted local operator and OS account. Native
code, rustix/crossterm/signal adaptation, and selected host adapters are trusted. Projects, records,
artifacts, JSON, proposals, terminal bytes, paste, dimensions, selected roots, directory entries,
file content/metadata, origin/reconciliation tokens, and outcomes are hostile bounded input.

There is no network, cloud sync, multi-user authorization, encryption/signature/provenance,
hostile-native-code sandbox, broad filesystem grant, secret store, child-process application
interface, wall-clock language semantic, database, persistent project/text index, build cache,
daemon, background worker, general async runtime, automatic merge/branch/rebase, GUI, binary editor,
grapheme-correct editing, syntax highlighting, clipboard, mouse workflow, or cross-platform product
claim. Provider token/cache/price telemetry is unavailable; bytes are not converted into token or
monetary claims.
