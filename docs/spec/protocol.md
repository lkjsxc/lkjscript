# Project CLI, protocol, context, and document contracts

This specification owns public project commands, strict machine framing, the retained low-level
engine protocol, context capsules, editable proposals, sessions, limits, rejection, and exit
behavior.

## Logical owners

`Project` owns discovery, exact authority selection, project reads, change publication/history,
target derivation, doctor, and backup. It delegates candidate construction to `Transaction`, durable
publication to the single-workspace persistence owner, context/documents to the workbench owner, and
target lowering to release/application owners. Command parsing and JSON rendering do not define
semantics.

`Engine` remains the lower-level typed boundary for create/open, transaction, query batch, run, and
schema description. Its raw RPC is retained as an independent conformance/embedding transport and
for implementation differential tests. It requires an explicit repository state directory and is
not the normal project authoring interface. Release/application/instance/runtime distribution
commands consume their own immutable or mutable authority domains; they do not modify projects.

One open engine holds one exclusive `lkjscript.engine.lock`. A competitor receives
`authority_busy`; there is no hidden queue. No mutation is silently retried. A
`commit_outcome_unknown` stops unsafe further mutation in that engine/session.

## Public project command surface

Project commands are:

```text
init [PROJECT]
orient [--project PROJECT] [--known-digest DIGEST]
status [--project PROJECT]
inspect SELECTOR [--project PROJECT] [--at REVISION] [--summary]
query PROJECTION [--root SELECTOR ...] [--at REVISION] [--limit COUNT]
  [--continuation TOKEN] [--known-digest DIGEST] [--project PROJECT]
proposal FUNCTION [--at REVISION] [--project PROJECT]
context --purpose PURPOSE [--target SELECTOR ...] [--at REVISION]
change validate|apply [--project PROJECT] [--document] [--context FILE]
log [--project PROJECT] [--before REVISION] [--limit COUNT]
show REVISION [--project PROJECT]
diff --from REVISION --to REVISION [--offset N] [--limit N]
restore REVISION [--validate] [--project PROJECT]
target list|show|build|test|run ...
doctor [--project PROJECT] [--deep]
backup DESTINATION [--project PROJECT]
session [--project PROJECT]
```

An explicit `--project` overrides ambient discovery and no facts are read from an ambient project.
Otherwise ordinary commands discover exactly one marker above the current directory. Reads default
to exact HEAD but accept `--at` where defined. Mutations carry exact workspace/base facts in their
input and reject stale or foreign requests. Build output and backup destination paths may be
relative; they resolve against the command working directory, reject lexical parent traversal and
symlink/nonregular parents, and remain deployment facts.

The one-shot project machine envelope is contract version 2:

```json
{"version":2,"result":{"kind":"status","data":{"contract_version":1}}}
```

Each command writes exactly one JSON value plus newline to stdout. `--pretty` selects the equivalent
indented, deterministic, terminal-safe human-readable JSON projection. Progress never contaminates
stdout. Semantic/project input errors exit 2, transport/authority failures 3, output failures 4,
artifact publication/validation failures 5, and resource exhaustion 8. Errors remain typed inside
the same project envelope; stderr contains one bounded diagnostic. Broken output after publication
does not roll back authority.

Project response serialization is bounded by the 32 MiB machine policy. Any mutation or target build
whose compact or pretty receipt cannot fit rejects before semantic or artifact publication. Query
responses are bounded at construction or paginated.

## Project foreground session

`lkjscript session` holds one selected project/engine in the caller-owned foreground process and
accepts one project-session-v2 JSON request per line:

```json
{"version":2,"request_id":1,"request":{"kind":"status"}}
```

Responses carry the same unique nonzero request ID. The closed request variants cover orient,
status, inspect, semantic query, function proposal, context, JSON/document validate/apply, log,
show, diff, restore, target operations, doctor, backup, and shutdown. At most 65,536 request IDs and
one 8 MiB request line are admitted.
Duplicate fields, unknown fields/variants, wrong versions, duplicate/zero IDs, invalid UTF-8,
trailing values, and oversized lines reject.

A malformed complete line produces one bounded uncorrelated error and the next line is processed.
EOF closes the session. Shutdown returns current status and flushes before exit. A commit/output
publication-unknown condition is fatal. Session-local context aliases are bound to the exact project
and revision and reject after same-session or external HEAD advance. Restart discards aliases and
recovers all authority from durable project state. Session reuse is disposable acceleration, not a
daemon, scheduler, queue, lock file, or semantic identity.

## Raw engine protocol and machine schema

Raw protocol and JSON envelope version is 13:

```json
{"version":13,"request_id":1,"request":{"kind":"create_workspace"}}
```

Responses carry the same nonzero ID. Unknown/duplicate fields or variants, invalid IDs, wrong
version, invalid UTF-8, trailing JSON, excessive input/output, and a second one-shot request reject.
Input is limited to 8 MiB and output to 32 MiB. A semantic failure is a successfully delivered typed
`error` response; inability to open authority or an indeterminate commit is transport/fatal.

`src/contract.rs` is the one executable schema owner. `src/machine.rs` owns strict JSON and
fingerprinting only; `src/machine_contract.rs` owns shared descriptors. The active schema identity is
`lkjscript-machine-schema-v13`. Schema requests return a compact manifest, at most 16 named roots
with deterministic dependency closure, the explicit full contract, or an exact `unchanged` result
for a known digest. Unknown, duplicate, empty, or excessive roots reject. Ordinary project work gets
the active schema digest from orientation and does not need a global schema dump.

The raw `--state DIR rpc|session` grammar is intentionally distinct from project commands. Its
session accepts one protocol-v13 envelope per line and shares exact one-shot semantics. It has no
project locator, target build configuration, or automatic friendly selector resolution. The former
`agent` adapter was removed rather than retained as an alias.

## Orientation

Project orientation binds contract, workspace, revision, snapshot, revision-record digest, active
machine-schema digest, bounded target summaries, command roots, and explicit omissions into one
domain-separated digest. It omits full graph, bodies, history, schema, target definitions, and
artifacts. Supplying the exact digest returns `unchanged` only for the same project/revision/content;
a foreign or stale digest returns changed facts. Status independently reports current exact
authority, graph summary, target count, and health without building or testing.

## Closed semantic queries

Semantic-query contract 1 is a derived observation over one immutable project snapshot. Its closed
projections are `summary`, `exact`, `children`, `function`, `owner_chain`, `dependencies`,
`incoming_uses`, `callers`, `callees`, `targets`, and `blockers`. A query binds workspace, exact
revision and snapshot, projection, zero to eight exact roots, deterministic input/root order, a page
limit in 1 through 256, and an optional continuation. `targets` and `blockers` accept no roots; all
other projections require roots, and `function` requires exactly one function.

A changed page reports its exact plan digest, result digest, items, total/returned/work counts,
truncation, and optional continuation. Supplying that page's result digest to the same exact request
returns `unchanged`. The opaque continuation canonically binds project, revision, snapshot,
projection, roots, limit, plan digest, and next offset and is protected by a domain-separated BLAKE3
digest. Any changed field, malformed token, foreign identity, stale revision, excessive work, or
response over 4 MiB rejects. Query construction performs at most 4,096 charged semantic work items
and retains no index or cache. Reads publish nothing.

`proposal FUNCTION` renders one complete function-scope semantic document directly from the same
snapshot. It returns exact workspace, revision, snapshot, function, qualified name, document version
and digest, plus the document. The proposal contains exact durable references and base-local draft
symbols; it does not require a context capsule. It is untrusted text and acquires no authority by
being generated.

## Context capsules

Context/workbench version 2 is a disposable exact observation. Its digest binds workspace,
revision, schema, purpose, target set, optional comparison revision, requested bounds, all included
facts, aliases, and omissions. Purposes are `orient`, `create`, `repair`, `refactor`, `debug`,
`extend`, `delete`, and `review`; targeted purposes require exact targets.

A capsule contains at most eight targets, at most 256 expanded nodes, bounded query pages, legal
edit/expression codes, typed observations, completeness blockers, and explicit truncation. Encoded
size is at most 4 MiB. Aliases are `@n1`, `@n2`, … and valid only with the exact capsule. Decode
revalidates version, digest, schema, domains, ordering, limits, and every target. A known context
digest returns `unchanged` only when all these facts agree.

Project `context` can select current or historical revision and returns a project envelope around
the capsule. `change --document --context FILE` accepts either raw capsule JSON or the exact saved
project context envelope and extracts only a validated changed capsule. A stale/foreign/malformed or
unchanged envelope cannot authorize aliases.

## Editable semantic documents

Editable semantic document version 2 is one proposal surface. Version 1 and the old `plan` root
reject:

```text
document {
  version 2
  schema "DIGEST"
  packet "DIGEST"
  workspace "WORKSPACE"
  base_revision REVISION
  scope (workspace)
  edits [ ... ]
  return_symbols [ ... ]
}
```

`packet` is required only when aliases are used. A generated function proposal sets it to `null`
and spells every durable identity exactly. Commit may carry one idempotency key; validation may not.
Function scope accepts one complete body replacement for the exact function. Workspace scope
accepts the closed transaction vocabulary, including build-target edits.

The grammar uses `{ field value ... }`, `[ value ... ]`, tagged `(kind payload)` / `(kind)`, JSON
strings, booleans, null, canonical integers, bare identifiers, and aliases. Commas, comments,
semicolons, equals, duplicate/unknown fields, parser recovery, multiple roots, and trailing input
reject. Limits are 8 MiB, 32 nesting frames, 65,536 parsed items, and 512 diagnostic bytes, checked
before corresponding work. Deterministic byte/line/column diagnostics name the exact owner and legal
alternatives where available.

Parsing discards syntax and produces typed transaction operations. The document declares every
edit; context is read-only; omission never deletes. Stale base/schema/packet, foreign alias, local
escape, wrong scope, implicit durable-hole deletion, invalid target reference, and response overflow
reject before publication. Formatting or selector spelling that normalizes to current meaning is
semantic no-change.

## History and review projections

`log` returns descending compact revision summaries and an exact `next_before` continuation.
`show` expands one canonical record. `diff` binds both endpoint snapshots, direction, exact digest,
total count, offset/limit, changes, and continuation offset. `inspect` returns selector spelling,
resolved durable ID, qualified name, typed facts/summary, and exact selected revision. Friendly
ambiguity is `invalid_query` with bounded canonical candidates; no first-match fallback exists.

Project apply returns the full semantic diff and revision record needed for immediate review. A
published change additionally returns project-change-continuation version 1: exact new revision and
snapshot, revision-record digest, accepted-change and semantic-diff digests, requested created
bindings, changed functions, affected targets, explicit invalidation of session-local aliases, and
a digest over those facts. The continuation is fixed-shape, at most 64 KiB, and response-preflighted
before publication. Validate-only returns no continuation. It is an observation, never authority or
durable local identity. Exact selectors in it can seed the next query or target action without a
global context refresh.

## Distribution and runtime command families

`release validate|inspect|test` consumes immutable release-format-2 artifacts. `app
validate|inspect|test|run|stream` consumes immutable application-format-8 artifacts. Release and
application construction occurs only through project targets; removed command-local `build`
predecessors reject. Instance contract 3 and runtime contract 2 are specified separately. These
records remain absent from the global workspace catalogue where they have an independent typed
owner.

## Version rejection

Protocol 12 and older, machine schema v12 and older, project/marker versions other than 1, project
change version 1, project machine/session version 1, editable document version 1, revision-record
versions other than 1, context packet 1, workspace format 7,
`lkjscript-tsm007`, `LKJHEAD9`, release format 1, application format 7, instance format 2, runtime
session versions other than 2, the `agent` command, command-local release/application build commands,
and the `plan` document root reject. There is no alias, fallback, edition, migration mode, or daemon
transport.
