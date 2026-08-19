# Typed semantic project and program model

This specification owns accepted development meaning, project identity, durable/local identities,
transactions, automatic revision history, build targets, and canonical workspace persistence.

## Authority

One immutable `Snapshot` is the authoritative meaning of one accepted project revision. It contains
the exact workspace identity, revision, root, durable allocator frontier and tombstones, closed typed
program items, first-class build-target declarations, explicit order/references, completeness facts,
and canonical hash.

A project locator, path, JSON request, semantic document, context capsule, rendered view, revision
receipt, cache, Core IR, artifact, or Git revision does not replace the snapshot. A change proposal
normalizes through the single transaction owner and complete validator. A release or application
artifact may be immutable distribution authority in its own domain, never editable project meaning.

The physical bootstrap graph remains a `BTreeMap<NodeId, Node>` and revisions remain full canonical
snapshots. This is an implementation representation, not a requirement for a graph database.

## Project identity and discovery

A semantic project reuses exactly one workspace identity; there is no duplicate project identity.
The strict marker `.lkjscript/project` is `LKJPROJ1`, project contract 1, one workspace ID, and a
domain-separated checksum. It contains no project name, path, revision, target, credential, or Git
fact. `.lkjscript/repository` must contain exactly that workspace.

An explicit project path is resolved against the current working directory. Lexical parent
traversal, symlinked path components, a non-directory root, a nonregular marker, malformed marker,
foreign repository identity, or incomplete authority rejects. Without explicit selection, discovery
walks at most 64 ancestors and requires exactly one marker; nested markers are ambiguous. Absolute
path spelling is deployment state and never enters semantic identity, snapshots, records, releases,
or applications.

Initialization publishes a complete no-replace project directory with genesis revision zero and one
genesis revision record. Reinitialization rejects. Backup copies only the `.lkjscript` authority,
omits the process lock, bounds depth/files/bytes, verifies the copy independently, synchronizes, and
publishes a no-replace destination. A copied project retains identity and history.

## Closed meaning vocabulary

The current ownership hierarchy is:

```text
workspace root
  -> package
    -> module
      -> product declaration -> product field
      -> sum declaration -> sum variant
      -> sequence declaration
      -> function -> parameter / body
  -> build target

function body or structured operation
  -> region -> block -> block argument / operation / terminator
```

`src/schema.rs` owns every accepted kind, field, stable tag, type, and operation. Unknown kinds,
fields, tags, references, result indexes, owners, target variants, or control forms reject. Every
non-root item has one exact owner and is reachable from the root. Observable lists have explicit
canonical order.

## Identity domains

A `WorkspaceId` is 128 random bits rendered as 32 lowercase hexadecimal characters. A durable
`NodeId` is `WORKSPACE:SERIAL`, with a nonzero canonical decimal serial below `2^63`; the root uses
serial 1. Accepted creation alone advances the allocator. Durable identity belongs to packages,
modules, nominal declarations and members, functions and parameters, build targets, and explicit
typed holes. These classes have current continuity consumers. Deleted serials are tombstoned and
never reused.

Regions, blocks, block arguments, ordinary operations, and implied terminators use
revision-and-function-bound local IDs `WORKSPACE:lFUNCTION.ORDINAL`. They express body ownership,
order, dominance, binders, sharing, and diagnostics only. They cannot be nominal identities, call or
target roots, durable history continuity, or tombstones. A whole-body replacement canonically
rebuilds them.

`DraftSymbol` is proposal-local. A context alias is capsule/session-local and exact-revision bound.
Names, qualified names, paths, hashes, target indexes, artifact offsets, compiler IDs, session
handles, and runtime handles are not entity continuity. Friendly selectors resolve against one exact
revision; zero matches rejects, multiple matches rejects with bounded canonical candidates, and
exact IDs remain authoritative.

Releases form a separate immutable domain. Projection erases workspace/revision IDs, allocation,
tombstones, and local numbering and assigns exact release-local item IDs. Nominal equality across
release boundaries is `(ReleaseId, ReleaseItemId)` equality. Coordinate, user version, slot, export
name, path, or content similarity cannot substitute. The detailed contract is
[reusable-release.md](reusable-release.md).

## Function bodies and replacement

Function bodies are immutable typed regions. Validation checks scope, order, dominance, result
indexes, types, owner chains, structured regions, branch/match contracts, and terminators without
depending on rendered syntax.

`ReplaceFunctionBody` preserves the durable function/signature, refuses implicit removal of a
durable hole, deletes only old local items, and canonically rebuilds local ordinals. `Hole(type)` is
the sole durable body anchor. `RefineHole` preserves its ID/position/output/uses and accepts one
regionless same-result operation. Wrong type, terminator, second refinement, foreign reference, or
stale base rejects.

A declaration shape or incompatible function signature is replaced by create/rewrite/delete. The
caller must rewrite every exact use and target/case mapping atomically or across valid untargeted
intermediate revisions. Deletion preflight reports surviving uses. Names and structural similarity
never infer continuity.

## Change proposals and transactions

Project change contract 1 binds:

- exact workspace and base revision;
- `validate_only` or `commit` selected by the command;
- optional commit-only idempotency key;
- one bounded closed ordered edit list; and
- a bounded returned-symbol projection.

Creation operations use proposal-local symbols and are dependency resolved independently of request
order. Function bodies may be created/replaced wholesale, refined by hole, or changed through typed
local operations. Build-target definitions and focused export/query/case edits are typed operations,
not opaque text. A bundle can update multiple functions and targets atomically. When a new target
needs durable IDs allocated by an earlier declaration proposal, the supported reviewable workflow is
an untargeted valid declaration revision followed by the target revision; there is no incomplete
distribution or hidden draft authority.

Preparation expands high-level operations to private canonical edits, checks all input and response
bounds, assigns provisional durable IDs only after proposal validation, clones the base snapshot,
applies edits, validates the whole candidate and every target, derives semantic diff/impact, encodes
the workspace artifact, and preflights compact and pretty receipts. Rejection changes no revision,
identity frontier, tombstone, target, artifact, cache, or HEAD. Validate-only performs the same
semantic/artifact/response work and publishes nothing. Commit publishes exactly one revision.

Semantic no-change is a typed rejection and publishes nothing. Stale base rejects; there is no
automatic retry, rebase, merge, or first-match selector fallback. The receipt returns exact
base/result revision, snapshot and change digests, publication truth, durable created count,
requested bindings, completeness, semantic diff, target impact, and the published revision record.
Function-local churn is summarized as a body change; durable entities are listed exactly.

Idempotency fingerprints canonical protocol-v12 transaction bytes, including response projection.
An exact retained replay returns the same receipt after restart without another revision; changed
reuse rejects. Validate-only cannot carry a key. An output failure after publication does not undo
authority; status/log/idempotency recover the accepted result.

## Build targets

A build target is a durable root-owned node with presentation name and contract-1 definition.
Target names are lookup metadata. Dependency edges use exact target IDs, are bounded, and must be
acyclic.

A release target owns one exact package root, coordinate/user-version fields with current artifact
consumers, exports, exact release-target dependencies, imports, immutable invocation cases, and
policies. An application target owns an exact root release target, entry, typed/byte-stream/stateful
profile, all nominal interface mappings, host requirements (never grants), resource policy, and
immutable cases. A product target selects one exact application target for native distribution.

Every accepted snapshot prepares every target through the same release/application validators and
case runners. A target therefore cannot be silently incomplete. Build selects an exact revision,
lowers only the exact target closure, preflights the response, and publishes canonical no-overwrite
artifact bytes to an explicit deployment path. Build/test/run are derived operations and do not
publish development revisions. There are no hooks, scripts, mutable registries, `latest` lookup,
ambient files, grants, caches required for correctness, or target-owned output paths.

Target changes appear as durable target-definition changes in revision records and semantic diffs.
Impact follows exact declaration, call, case, and target edges; it is evidence for selective work,
not authority to skip validation.

## Automatic revision history

Revisions are contiguous from zero and immutable. Each revision has exactly one canonical record,
format 1 / `LKJREC01`, binding:

- workspace, revision, parent revision, parent snapshot, and parent record;
- result snapshot and accepted change-set digest;
- complete semantic diff digest/count;
- created, deleted, and modified durable identities;
- function-body and target-definition changes;
- exact affected targets; and
- genesis, accepted, or restoration publication outcome.

Optional ambient author, clock, provider transcript, hidden reasoning, or commit message is not
recorded. The accepted request normally records itself.

`log` pages compact deterministic summaries in descending revision order. `show` returns one full
record. `diff` recomputes exact endpoint semantic change with bounded offset/limit and direction; it
does not infer continuity for entities created then deleted between endpoints. Historical inspect
reads the exact retained snapshot.

`restore REVISION --validate` previews; without `--validate` it publishes a new revision. It never
moves HEAD backward. Current allocator/tombstones remain authoritative, later-only entities are
deleted and tombstoned, and a source identity absent from current meaning would be resurrected, so
automatic restoration rejects. Product instance state, host effects, artifacts, files, and Git are
outside restoration semantics.

## Canonical repository persistence

Each revision is one full canonical artifact:

- magic `LKJTSM\0\x08`;
- artifact version 8;
- semantic schema `lkjscript-tsm008`;
- fixed little-endian checked fields and lengths;
- canonical item/target order;
- domain-separated BLAKE3 snapshot hash; and
- strict trailing-byte rejection.

`LKJHDA10` binds workspace, exact HEAD revision/hash, exact current revision-record digest, and the
retained idempotency receipt under a domain-separated checksum. A mutation preflights immutable
snapshot, record, HEAD, and response; synchronizes immutable bytes before atomic HEAD replacement;
and acknowledges success only after the directory synchronization boundary. Failure before visible
HEAD leaves old authority. A visibility-capable failure is `commit_outcome_unknown`, stops unsafe
mutation, and requires exact reconciliation.

Ordinary open validates the kind, canonical name, presence, and size bound of every retained snapshot
path; decodes every compact record; validates record identity, ordering, parent record/snapshot links,
canonical change sets, exact HEAD, the current snapshot, and retained idempotency facts. It does not
decode unrelated historical snapshots. Selecting a historical revision decodes that exact snapshot
and binds it to the record's result digest. Deep doctor additionally decodes every snapshot, validates
every adjacent allocation/tombstone/identity transition, and recomputes every record's semantic diff
and entity/target facts. Missing, truncated, oversized, trailing, noncanonical, wrong-schema,
checksum-mismatched, foreign, or conflicting selected state rejects. Format 7 / `LKJTSM\0\x07` /
`lkjscript-tsm007`, `LKJHEAD9`, protocol 11, and all older direct predecessors reject; no migration
reader remains.

Full snapshots are selected over a change journal, content-addressed object graph, Merkle sharing,
and packed objects because they retain a simple independent reconstruction/diff/backup oracle and
the measured 100-change large-application corpus stays within practical repository bounds. Reopen,
recent log, arbitrary diff, deep doctor, and backup are measured separately. Change storage only if
retained bytes or service times cross the explicit roadmap gates; any replacement must preserve full
hostile decoding, crash consistency, tombstones, history, portable backup, and a simple oracle.

## Other authority domains

Release format 2 owns reusable immutable closure. Application format 5 owns one runnable release
graph, public types/entries/cases, stateful query/mutation mappings, resource policy, and host
requirements but no grant. Instance format 3 owns exact application-bound mutable continuity,
grants, state revisions, journal/checkpoints, commands/attempts/outcomes, and pure-query receipts.
Deployment owns paths, processes, users, and resources. Core IR, ownership plans, indexes, caches,
runtime tags, and execution values remain derived.
