# Protocol, context, and editable-document contracts

This specification owns logical requests/responses, strict JSON, process adapters, agent context
packets, editable documents, caching, limits, rejection, and exit behavior.

## Logical engine boundary

`Engine` owns workspace create/open, transaction preparation and publication, query, compilation,
run, and exact reusable-release preparation. Workspace RPC accepts closed typed `Request` values
and returns closed typed `Response` values. Release preparation uses one exact immutable workspace
revision plus explicit dependency bytes through its own typed method. Application composition uses
only explicit immutable release bytes; instance operation uses a separate exact local store.
Semantics do not depend on JSON or terminal rendering.

The logical request families are workspace creation, transaction application, query batch, run,
schema description, and adapter shutdown. Semantic failures are `Response::Error`; inability to open
the authority or a publication outcome that may be unknown is a transport/fatal engine failure.

One engine holds an exclusive `lkjscript.engine.lock` for the state directory. A competing direct
command or session rejects with `authority_busy`. The engine never silently retries a mutation.
`commit_outcome_unknown` permanently stops that engine object.

## Process topologies

The primary CLI opens `Engine` directly for one workspace command and exits.
`lkjscript --state DIR session` holds one engine and accepts one compact protocol-v10 JSON request
per line; each line has an independent publication boundary. EOF closes the session. A successful
`shutdown` response is flushed before exit.

Application and instance one-shot commands use the topology-neutral runtime kernel. The separate
`lkjscript runtime session --store DIR` command retains that same kernel and store lock behind exact
line-delimited runtime protocol version 1; its contract is specified in
[runtime-kernel.md](runtime-kernel.md). There is one installed binary and no daemon, socket client,
or background service. Disconnect or stdout failure does not roll back a published workspace
revision, instance revision, or host outcome. Exact idempotency receipts are the retry route; no
mutation or possibly visible host action is retried implicitly.

## Strict JSON projection

Protocol and JSON envelope version is 10. A request envelope is exactly:

```json
{"version":10,"request_id":1,"request":{"kind":"create_workspace"}}
```

Response envelopes carry the same nonzero request ID. Unknown fields and variants, duplicate fields,
invalid canonical IDs, wrong version, trailing JSON, excessive input, invalid UTF-8, and a second
request reject. JSON input is limited to 8 MiB and output to 32 MiB.

The protocol uses stable typed error codes and structured targets. Process exit distinguishes CLI
usage/JSON/document error, authority/transport failure, and output failure. A semantic rejection is
a successfully delivered logical `error` response rather than a transport failure.

## Executable machine contract

`src/contract.rs` owns the closed executable contract description. `src/machine.rs` owns only strict
wire encoding/decoding and re-exports the public contract entry points. `src/machine_contract.rs`
contains the shared descriptor value model. Agreement tests compare every advertised request,
response, record, variant, scalar domain, error, operation, query, and limit with strict serde and
executable samples.

The active identity is `lkjscript-machine-schema-v10`. Its canonical BLAKE3 digest is embedded in the
agent binary and printed by `agent orient`. Diagnostic clients may request:

- a compact manifest;
- at most 16 named roots and their deterministic dependency closure;
- the explicit full contract;
- `unchanged` for an exact known schema digest.

Unknown, duplicate, empty, or excessive roots reject. Normal agent work does not require schema
discovery.

Reusable-release build, validate, inspect, and test are command-local projections of the separate
[release contract](reusable-release.md), not additions to workspace RPC. Release build JSON uses
contract version 1 and names exact workspace/revision authority; all dependency artifact paths are
explicit command inputs. Its strict Rust records, canonical codec, and command-local help own the
fields.

Application build, validate, inspect, test, typed run, and stream are command-local projections of
the separate [application contract](application.md). Application JSON uses contract version 4 and
build accepts only explicit release files. Durable-instance commands use command-local contract
version 2 specified by [instance.md](instance.md). Runtime orientation, inspection, and session use
command-local contract version 1. Release, application, instance, and runtime records are
deliberately absent from the global workspace catalogue, avoiding duplicate schema owners and a
mandatory global dump. Top-level parsing and operation errors return the applicable contract
version.

## Context packets

Context packet version 2 is a disposable exact observation. Its digest binds:

- workspace and revision;
- machine-schema digest;
- purpose and target set;
- optional comparison revision;
- requested bounds;
- every included fact and explicit omission.

Purposes are `orient`, `create`, `repair`, `refactor`, `debug`, `extend`, `delete`, and `review`.
Targeted purposes require targets. A packet contains at most eight targets, at most 256 expanded
nodes, bounded query pages, exact aliases, legal edit/expression codes, typed observations,
completeness blockers, and explicit truncation flags. Total encoded size is at most 4 MiB.

Aliases use `@n1`, `@n2`, and so on. They are valid only with the exact supplied packet and never
persist. Packet reads revalidate version, digest, schema, identity domains, canonical ordering,
limits, and all alias targets.

`agent context --known-digest DIGEST` rebuilds the requested capsule and returns exactly
`{"version":2,"digest":"...","unchanged":true}` when the digest matches. A stale, corrupt,
foreign, cross-purpose, or differently bounded digest cannot produce unchanged. This saves output
bytes but is not authority or a semantic cache.

## Editable semantic documents

Editable semantic document version 1 is the preferred proposal surface. Its root is `document`; the
old `plan` root is invalid. Required fields are:

```text
document {
  version 1
  schema DIGEST
  workspace WORKSPACE
  base_revision REVISION
  scope (workspace) | (function NODE)
  edits [ ... ]
  return_symbols [ ... ]
}
```

`packet DIGEST` is required when aliases or packet-bound scope are used. Commit documents may carry
one idempotency key; validate-only documents may not. Function scope accepts exactly one
`replace_function_body` targeting the durable function included in the packet. Workspace scope
accepts the closed transaction vocabulary.

The grammar uses `{ field value ... }`, `[ value ... ]`, tagged `(kind payload)` or `(kind)`, JSON
strings, booleans, null, canonical integers, bare identifier strings, and packet aliases. Commas,
semicolons, equals signs, comments, duplicate fields, unknown fields, multiple roots, and trailing
input reject.

The parser is limited to 8 MiB, 32 nesting frames, 65,536 parsed items, and 512-byte diagnostics. It
tracks deterministic byte/line/column locations, uses explicit parser frames, and checks size before
unbounded allocation. Parsing produces a closed typed proposal; formatting and syntax are discarded.

The document declares all editable content. Packet context is read-only. Omission never implies
deletion. Stale base, stale schema, packet mismatch, foreign alias, local-reference escape, wrong
scope, and implicit deletion of a durable hole anchor reject before publication.

`agent document --packet FILE` renders one complete function target. Render-parse without edits is a
semantic no-op. The renderer refuses a whole-body replacement when the body contains a durable hole
anchor, because omission cannot erase continuity.

Run uses a separate `run { ... }` document naming exact workspace, revision, entry, arguments, and
policy. It never implies current HEAD.

## Review and output policy

`agent view` renders a deterministic read-only semantic review. `agent diff` renders the exact
change set carried by a review packet. Durable and function-local identities are labelled
separately; full IDs are optional. Output is terminal-safe and bounded to 4 MiB.

Successful apply returns the compact transaction receipt. It does not yet return a context delta;
the exact known-digest mechanism avoids unchanged serialization. Apply-and-refresh remains a future
gate if measured request savings justify response preflight and idempotency complexity.

## Version rejection

Protocol/JSON 9 and older, machine schema v9 and older, context packet 1, release command contract
0, application command/artifact version 3 and older, instance command/artifact version 1 and older,
runtime session versions other than 1, release artifact formats other than 1, and the `plan` edit
root reject. No alias, fallback, compatibility reader, daemon transport, or migration mode remains.
