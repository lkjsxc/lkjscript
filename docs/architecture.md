# Current architecture

This document maps the implemented graph-to-runtime system. Normative behavior belongs to
`docs/spec/`; current limitations belong to `docs/status.md`.

```text
public semantic query / transaction / draft protocol
                         │
                         ▼
          typed stable-ID meaning graph candidate
                         │
              full validation + packed oracle
                         │
                         ▼
 immutable module tables + root + revision + receipt
                         │
                atomic durable HEAD
                         │
       ┌─────────────────┼──────────────────┐
       ▼                 ▼                  ▼
 derived query     deterministic       graph-native
 index shards      review/backup       package artifact
                                             │
                              ┌──────────────┴──────────────┐
                              ▼                             ▼
                     prepared bytecode          semantic reference oracle
                              │                             │
                              └──────── differential tests ┘
                                             │
                                  component/port runtime
                                             │
                                  typed deployment grants
                                             │
                                  generic bounded adapters
```

| Layer | Current owner | Owns | Excludes |
|---|---|---|---|
| Canonical authority | `meaning.rs`, `graph.rs`, `revision.rs`, `repository.rs` | stable semantic owners, immutable roots/revisions, exact publication | source coordinates, indexes, bytecode, host handles |
| Semantic development | `cli.rs`, `semantic_query.rs`, `semantic_transaction.rs`, `semantic_draft.rs`, `semantic_diff.rs`, `semantic_merge.rs` | bounded reads, high-level writes, drafts, diff/merge | raw packed-record editing |
| Derived review and recovery | `semantic_projection.rs`, repository backup/restore | deterministic review, exact canonical bundle | a writable text authority |
| Compiler and artifact | `artifact.rs`, `execution/` | exact graph closure, deterministic package objects, preparation, bytecode | credentials and deployment grants |
| Component model | graph-owned components, ports, requirements, targets | typed entries and required operation sets | sockets and application-specific native dispatch |
| Resident runtime | `runtime.rs`, `http.rs`, `worker.rs`, `stream.rs` | admission, task ownership, execution, cancellation, shutdown | routes, SQL, authorization, object keys, queue transitions |
| Capabilities | `execution/capability.rs` and generic adapters | requirement/grant equality, operation accounting, resource mechanics | domain permission policy |
| Deployment | `deployment.rs` and strict deployment JSON | concrete adapters, secret bindings, limits, listener topology | program meaning and artifact authority |

## Authority, identity, and history

The logical owner is a typed semantic graph. Its physical contract is canonical binary tables:
one packed immutable table per module, one packed root naming exact table digests, immutable revision
records and receipts, and one small HEAD visibility record. Modules are content-addressed and shared
across revisions. Query indexes, review files, artifacts, and bytecode are rebuildable.

Repository, module, declaration, field, case, operation, parameter, binding, expression, port,
requirement, target, documentation, annotation, draft, and revision IDs use distinct tagged domains.
Names are mutable indexed attributes. Rename and move preserve the selected stable owner; clone and
new creation allocate a new ID; deletion retains a typed tombstone and IDs are not reused. Content
digests, revision IDs, storage keys, dense compiler indexes, and runtime handles never substitute
for stable semantic identity.

Accepted history is an immutable DAG. A revision ID commits to repository identity, ordered exact
parents, root digest, transaction digest, and semantic diff digest. Drafts are separate
non-executable operational authority tied to one accepted base. Holes and conflicts cannot enter
accepted HEAD.

## Publication and storage

A writer locks the repository, rereads exact HEAD, validates the complete candidate and an
independent packed encode/decode reconstruction, writes unreachable immutable objects, then makes
one atomic HEAD replacement. On Linux, new object data and directory metadata are flushed together
with `syncfs` before the separately synchronized HEAD stage and directory rename. Other targets use
per-file synchronization. Readers therefore observe the old complete or new complete revision;
pre-HEAD leftovers are unreachable.

The broad query index contains ordered owner summaries and semantic relations. Separate 256-way
revision-bound name and owner shards let exact orientation/find/show read only relevant derived
parts; showing a body adds its owning canonical module table. Any missing, stale, or corrupt index
rebuilds from canonical objects. Deep doctor ignores derived indexes and reconstructs the retained
revision DAG.

Git transports canonical objects and revision records but does not define program identity or
merge meaning. Semantic diff and three-way merge operate on stable owners and exact bases before a
normal publication.

## Compiler and runtime

Build reads the exact graph root and dependency artifact closure directly. No maintained text is
generated or parsed. Preparation lowers typed expressions into compact bytecode while preserving
stable-owner provenance outside hot runtime handles. The production VM and reference interpreter
share graph types and values but not instruction dispatch. Every maintained package test compares
their values, failures, and instruction observations.

One component/port model covers command, HTTP, interactive, batch, worker, and test runner kinds.
Artifacts contain typed requirements, never grants or credentials. Deployment binds requirements
to generic adapters with exact operation sets, limits, sharing domains, and redacted secret
acquisition. `lkjournal` policy remains graph meaning; generic Rust contains no application route,
table, authorization, object-key, or queue-transition policy.

The source parser and source semantic builder remain compiled only for Rust test oracles and
fixture construction. They cannot open, publish, build, or execute a maintained project and are not
a user authoring path.
