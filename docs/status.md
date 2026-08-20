# Implemented status

This file describes the current checkout. Normative contracts under `docs/spec/`, executable
validation, and the checked semantic projects outrank this summary.

## Semantic development and distribution

One discoverable semantic project owns one validated typed meaning graph, selected HEAD, immutable
full snapshots, canonical revision records, durable allocation/tombstones, semantic diff, and a
first-class target graph. The strict marker binds one workspace. Relative and absolute project
locators are deployment facts; symlinked/nonregular markers, unsafe traversal, nested ambiguity,
foreign binding, malformed objects, and corrupt continuity reject.

The public CLI implements initialization, compact orientation with known-digest reuse, status,
historical inspection, closed paginated semantic queries, generated function proposals, targeted
context, JSON/document validation and apply, log/show/diff/forward restoration, target
list/show/test/build/run, shallow/deep doctor, no-replace backup, and a correlated caller-owned
foreground session. Ordinary commands discover workspace/revision/schema facts. Raw engine RPC is a
separate conformance surface.

Every accepted mutation publishes exactly one immutable revision and record. Validate-only,
rejection, stale/foreign input, no-change, and read-only work publish nothing. Candidate validation,
affected targets, artifact preparation, and response bounds preflight before publication. A receipt
reports exact result revision/snapshot/record, accepted-change and diff digests, changed functions,
affected targets, returned bindings, and session-alias invalidation. No mutation is silently retried.

Durable target kinds are reusable release, application, and native product. Exact graph IDs own
target identity and edges; names are revision-bound lookup metadata. Build/test/run select one exact
revision and never publish development history. There is no semantic build script, arbitrary hook,
ambient file dependency, mutable coordinate, generated semantic source, or private builder.

Release format 2 is the reusable closure authority. Application contract 8 now uses internal format
9 under magic `LKJAPP\0\x08`; it directly rejects internal format 8. Interactive profile 3 binds
application-owned initialize/update/resume/render, styled frames, mouse/focus/open events, and typed
job outcomes. Headless replay 4 executes those same roles for at most 20,000 transitions.

## lkjedit ordinary editor

`applications/lkjedit/.lkjscript` is the sole maintained editor program/build authority. It is
workspace `6ee361b40e2ce5041d64321d79c3db0d`, revision 174, snapshot
`7e037b9e97e2f04cd2243899a30a3721f31faf30c55aa9d7d97f050d22004aa4`, record
`aa634314469c3712fd4fca1976d07ce20073c5162a4109abaec94c6dabfe107b`, with 10,926 graph nodes,
1,217 durable identities, 9,709 function-local references, three targets, 12 passing target cases,
and no blockers. Revision 172 is the user-visible public-CLI dogfood change; revisions 173 and 174
cut rendering over to the new bulk text/sequence and bounded cell-prefix operations. Each was
validated without publication and then applied exactly once.

The final target artifact is 471,096 bytes. Its file SHA-256 is
`95cb525cea6440164e9eac58383fc194d79fc2b6df9baeadfc75309083e8338a`, application digest
`109577058354605dbcb8c6ca45a17476a2a7a8cf3689d3499ecc611ff76e8ef5`, graph digest
`aebf4631e05840953a7d493999c610f18b189ad4bc31bd0eaf0fe3ab9608d2d3`, and root release
`13e49b20e4e2d5dbec7baf6a371d7ac9563b746547bb5af7b11d16c739c2b7ab`.

Application meaning owns the normalized split tree and integer weights; tile/tab/view/buffer/file
origin identities; heterogeneous tab lifecycle; Vim-like parser/modes/keymap; buffer content,
line-ending policy, register, dirty/conflict/reconciliation state, and undo/redo; independent view
cursor/selection/viewport; explorer and current/root search policy; mouse hit interpretation, tab
and splitter drag state; pending-job intent and outcome integration; close/exit policy; and logical
styled frames. Multiple views share one buffer without sharing view state.

The native product loads the embedded checked package, validates it, acquires and cleans the
terminal, decodes closed key/paste/resize/SGR-mouse/focus events, projects abstract styles and safe
Unicode cells, performs acknowledged row-differential output with full-frame fallback, grants one
selected root and optional semantic project, and runs one capacity-one worker. It does not interpret
Vim commands, content kinds, dirty policy, drop zones, search presentation, or semantic drafts.

Selected-filesystem 2 pins one Linux directory descriptor and confines UTF-8 components with
`openat2` beneath/no-magiclink/no-symlink/no-mount-crossing policy. It provides digest-bound sorted
directory pages, stable UTF-8 regular-file reads, bounded deterministic recursive literal search,
no-replace create, expected-base atomic replace, ordinary-mode preservation, explicit conflict,
known failure, unknown visibility, and independent reconciliation. New files use deterministic mode
0644. No possibly visible write is repeated.

Runtime text is an unobservable persistent UTF-8 piece treap with bounded 4 KiB target chunks,
structural sharing, byte/scalar/newline aggregates, and disposable extended-grapheme boundaries from
`unicode-segmentation` 1.13.3. Canonical flat UTF-8 materialization remains the independent oracle.
The language names byte, scalar, grapheme, line, terminal-width, splice, slice, and literal-search
operations separately. A bounded cell-prefix operation returns only complete grapheme boundaries
that fit the requested terminal-cell budget. Unicode normalization and locale behavior remain
absent.

The prior active editor/workbench product and all active `lkjstudio` paths, binary, artifact, native
entry point, instructions, target names, and compatibility aliases are deleted. Historical Git and
clearly dated evidence remain the archive.

## lkjwork dogfood

`applications/lkjwork/.lkjscript` remains workspace `6894b57786a7e1ef14370d2da3a3cf33`,
revision 9, snapshot `4ae531e41c93b69583ee14c44837ce1d5f6748e427d3a02f60c7164b2174eec1`,
record `fb28346946daef184262e06781ee42c537913d63ba313ef50834744458f30561`, with three targets,
eight cases, and no blockers. Its format-9 checked artifact is 170,545 bytes with file SHA-256
`a1473c270df99aa6ccdebba85a6c99e23f2f2da668b5a2955f28098c5561c57f`.

## Active identities and rejected predecessors

| boundary | active identity | directly rejected predecessor |
|---|---|---|
| raw workspace protocol / schema | 14 / `lkjscript-machine-schema-v14` | protocol/schema 13 and older |
| semantic project / marker | 1 / `LKJPROJ1` | every other version |
| project change / machine / session | 2 / 3 / 3 | machine/session 2 and older; change 1 |
| semantic query / change continuation | 1 / 1 | every other version |
| workbench / context / edit document | 2 / 2 / 2 | context/document 1 and `plan` |
| revision record | 1 / `LKJREC01` | every other version |
| workspace artifact / HEAD | 8 / `LKJTSM\0\x08` / `LKJHDA10` | format 7 / `LKJHEAD9` and older |
| target / reusable release | 1 / 2 (`LKJREL\0\x02`) | every other target/release version |
| application contract / format / interface | 8 / 9 (`LKJAPP\0\x08`) / 1 | internal format 8 and older contracts |
| interactive / headless / terminal | 3 / 4 / 4 | profile 2, replay 3, terminal 3 |
| project host / workbench host / selected filesystem | 2 / 2 / 2 | selected filesystem 1; other versions |
| durable instance / runtime session | 3 (`LKJINS\0\x03`) / 2 | instance 2; other session versions |

There are no editions, compatibility readers, dual success paths, migration modes, old launch
aliases, or builder fallbacks. Historical snapshots retain only the validator needed to inspect
their own history and cannot produce a predecessor current artifact.

## Verification, trust, and known limits

`tools/check` owns compact `quick`, `product`, and `full` profiles. It runs locked offline commands,
retains bounded complete logs under ignored `.artifacts/check/`, prints aggregate passing output,
and reports bounded failure excerpts plus exact log locators. Machine mode emits one uncontaminated
versioned JSON result. A skipped, unavailable, exhausted, or indeterminate gate is never a pass.

The verified deployment is Linux x86-64 under one trusted local operator and OS account. Trusted
code is first-party Rust plus selected terminal/filesystem dependencies. Semantic projects,
records, artifacts, proposals, terminal bytes, paste, dimensions, paths, files/metadata, search
results, origin/reconciliation tokens, host outcomes, and logs are hostile bounded input.

There is no network, multi-user authorization, encryption/signature/provenance, hostile-native-code
sandbox, broad ambient filesystem, secret store, shell/child-process editor interface, database,
daemon, general async runtime, automatic semantic merge/branch/rebase, GUI, binary editor, syntax
highlighting, clipboard, plugin system, file watch, persistent unsaved-editor recovery, or
cross-platform product claim. Provider token/cache/price telemetry is unavailable; no tokens or
money are inferred from byte measurements.
