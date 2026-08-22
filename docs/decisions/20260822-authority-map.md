# Durable authority and sole-writer map

Date: 2026-08-22 UTC.

## Status

Accepted inventory of the CLI v4 and current repository implementation. It distinguishes accepted
program authority from draft, operational, derived, evidence, bootstrap, recovery, and deployment
state. Persisted semantic summaries and their reverse index are described as derived; compiler
caches, packs, retention pins, compaction, and garbage collection are not described as
implemented.

## Durable project inventory

All live repository-managed paths are below `.lkjscript/meaning`.

| Path or value | Classification | Meaning and loss behavior |
|---|---|---|
| `HEAD` | operational authority | The single visible binding to repository ID, revision, and revision-record digest. Losing or corrupting it prevents selection; replacing it is the publication visibility point. |
| `revisions/*/*.lkjv` | accepted history authority | Immutable revision cores bind repository, parents, canonical root, semantic certificate, semantic diff, and transaction. Reachable records are required to reconstruct accepted history. The certificate authenticates rebuildable semantic facts; it is not another graph. |
| `objects/roots/*/*.lkjr` | canonical accepted meaning | Immutable root manifests bind package metadata and canonical collection roots. |
| `objects/map-pages/*/*.lkjp` | canonical accepted meaning | Reachable immutable persistent-map pages hold root collections. Unreachable pages are physical garbage, not accepted meaning. |
| `objects/modules/*/*.lkjm` | canonical accepted meaning | Reachable immutable typed module tables own declarations, members, expressions, relations, and retained graph documentation. |
| `artifacts/*/*.lkja` | exact dependency authority | Immutable package-object closure required by accepted dependency bindings. Staging alone introduces no graph binding or new reachability and cannot change HEAD. |
| `receipts/*/*.lkjt` | integrity-bound evidence | Accepted revision records bind receipts proving transaction, diff, affected owners, and validation. Receipts explain acceptance but are not a second program graph. |
| `drafts/*.lkjd` | non-executable draft authority | Pending operations, base, generation, holes, conflicts, and intent. Draft mutation cannot change accepted HEAD. |
| `indexes/summary-objects/*/*.lkis` | derived cache | Content-addressed semantic-summary contract-2 facts. Loss rebuilds from canonical modules. |
| `indexes/*/*/semantic-dependencies.lkix` | derived cache | Revision-bound reverse dependencies whose certificate must match the accepted revision. Missing/malformed bytes rebuild; a rebuilt certificate mismatch is canonical corruption. |
| other `indexes/**` | derived cache | Revision-bound query manifests and owner/name/relation shards. Loss or corruption rebuilds from canonical authority. |
| `LOCK` | operational coordination | Empty local lock file serializes publication, draft mutation, backup, and related repository operations. It owns no meaning. |
| directories and private stage names | operational state | Layout and interrupted unreachable stages. Opening may reconstruct missing draft/index directories and `LOCK` only after canonical validation. |

Reachability, not mere file presence, determines authority. Canonical corruption blocks writes;
derived-index loss changes performance only. Current backup includes reachable canonical objects,
bound receipts, exact dependency objects, and drafts, but excludes indexes and the lock.

## Durable state outside the live repository

| Item | Classification |
|---|---|
| `packages/standard/.lkjscript/meaning` | Maintained accepted standard-package authority. |
| Embedded standard artifact and `applications/lkjournal/dependencies/standard.lkja` | Integrity-checked derived replicas of maintained standard authority for offline bootstrap and exact dependency transport. |
| Built `.lkja` application artifacts | Derived executable/package closure; requirements but no grants or secrets. |
| Review JSON | Derived non-authoritative evidence with no apply path. |
| `.lkjb` backup directory | Segmented external recovery authority for an exact repository; it becomes live only after verified restore. |
| Deployment descriptor | External operational authority binding artifact, target, grants, adapters, limits, and secret names. |
| Environment secrets | External secret authority; never graph, artifact, receipt, review, or log content. |
| PostgreSQL, object-store, and durable-queue data | External application/runtime authority governed by graph-authored policy and deployment grants. |
| Rust source, `Cargo.lock`, toolchain, and executable | Bootstrap/host implementation authority, not lkjscript application meaning. |
| `AGENTS.md`, specifications, decisions, prompts, tests, benchmarks, checker receipts, and logs | Policy or evidence according to their named role; none can replace accepted graph bytes. |

An in-memory graph snapshot, query index, prepared program, bytecode, VM frame, resident task,
capability adapter, connection, stream, lease, or session is never durable program authority.

## Writer boundaries

`replace_head` in `src/platform/repository.rs` is the only implementation point that switches the
visible revision in an existing repository. It is private to the repository module. New immutable
data is written and synchronized first; HEAD is then replaced by one rename under repository
serialization. Zero-base initialization and restore instead construct a complete private store and
make that store visible by a directory rename.

Visible-authority flows are:

1. `new` builds an initial graph in a private project stage and calls
   `SemanticRepository::initialize`; the command template additionally uses the normal `change`
   path inside that stage. The project directory becomes visible by one outer rename.
2. `change --commit` lowers one change contract to the exact transaction protocol, then calls
   `SemanticRepository::publish` after validation and exact-base checking.
3. `draft publish` constructs that same transaction request and calls the same transaction and
   publication path; draft create/append/rebase/drop affect draft authority only.
4. `history merge --apply` builds a typed merge proposal and calls `publish_merge`, which shares the
   repository lock, validation, immutable writes, and HEAD switch. A conflict publishes nothing.
5. `restore` verifies a complete backup in a private store, deep-checks it, and makes that store
   visible without synthesizing a new graph revision.

`package stage` may add exact dependency objects but does not add a graph binding or change HEAD.
`inspect`, `query`, `check`, `build`, `run`, `review`, `backup`, and ordinary `doctor` do not change
accepted meaning; index rebuild is derived state. `serve` and `worker` change external runtime
systems only through declared capabilities.

`doctor cleanup` is also read-only. Retention contract 1 inventories HEAD's parent DAG plus live
draft bases, candidate unreachable canonical files, derived bytes, and unknown entries, then binds
that observation with a plan digest. It creates no durable retention authority and always reports
`destructive_ready: false` because revision pins, active-reader leases, and registered backup roots
are not represented.

The sole ordinary authoring protocol is `change`. Revision-advancing authored updates from
`change`, draft publication, and typed merge use the one repository publication kernel. Zero-base
initialization and exact restore are separate operations because their invariants differ, not
alternate graph editors.

## Current boundary gaps and selected cleanup

The ordinary-authoring one-writer invariant is implemented at the CLI and storage visibility
boundaries, but not yet at the Rust crate surface. Public modules currently expose
`InitialPublication`, `PublicationProposal`, `SemanticRepository::{initialize,
initialize_from_artifact, publish, publish_merge, backup_to, restore_backup_from,
retention_preview}`, `execute_change`,
`execute_transaction`, `SemanticDraftStore` mutation, and `merge_revisions`. They still pass through
canonical validation and the repository publication kernel, but a Rust caller can bypass the CLI
v4 grammar. No predecessor migration executable remains.

CLI v4 has no artifact-import or predecessor-migration command, and maintained first use uses
`new`. The remaining selected cleanup is to remove `initialize_from_artifact` and its workspace
wrapper and narrow the low-level publication API once test/oracle consumers have an explicitly
test-only construction route. Until that cutover, the precise claim is one ordinary CLI writer and
one in-place revision-publication kernel, not one callable Rust writer.

## Recovery and security invariants

- Project identity, package identity, revision identity, graph digest, artifact digest, storage
  key, and filesystem location remain separate domains.
- `new`, commit, merge, and restore expose the old complete state or the new complete state.
- A stale base, invalid graph, failed precondition, conflict, no-change, exhaustion, or failed
  validation never advances HEAD.
- Backup and restore preserve exact repository and revision authority; cloning is a separate future
  operation.
- Deployment grants and secrets never enter accepted artifacts or graph authority.
- Deep doctor is the exhaustive reconstruction path; caches cannot certify missing canonical data.
- Current semantic summaries and reverse indexes, and future compiled units, sessions, packs, and
  GC catalogs, remain derived or operational unless a later explicit decision changes their role.
