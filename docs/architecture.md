# Current architecture

This document maps the implemented graph-to-runtime system. Normative behavior belongs to
`docs/spec/`; current limitations belong to `docs/status.md`.

```text
direct CLI v4 / strict change v3 / transaction v4
                         │
                         ▼
           typed stable-ID meaning graph v4
                         │
       ┌─────────────────┴─────────────────┐
       ▼                                   ▼
 body / create / either rename  all other changes:
 local preparation              complete preparation
       └─────────────────┬─────────────────┘
                         │  complete validator + packed oracle retained
                         │
                         ▼
 immutable modules + six persistent Merkle maps
                         │
       revision + receipt + one durable atomic HEAD
                         │
       ┌─────────────────┼──────────────────────┐
       ▼                 ▼                      ▼
 derived query     persisted derived      deterministic
 indexes           summaries/reverse      review/backup
       │                 │
       │          authenticated by the
       │          revision certificate
       └─────────────────┬──────────────────────┘
                         ▼
              graph-native package artifact
                         │
              ┌──────────┴──────────┐
              ▼                     ▼
     prepared bytecode      semantic reference oracle
              └────── differential tests ──────┘
                         │
              component/port runtime
                         │
              typed deployment grants
                         │
               generic bounded adapters
```

| Layer | Current owner | Owns | Excludes |
|---|---|---|---|
| Canonical authority | `meaning.rs`, `graph.rs`, `revision.rs`, `repository.rs` | graph contract 4, stable semantic owners, persistent root pages, immutable revisions, semantic certificate, exact publication | source coordinates as authority, index bytes, bytecode, host handles |
| Public development | `cli.rs`, `semantic_change.rs`, query/transaction/draft/diff/merge modules | direct CLI v4, concise change v3, exact internal transaction v4, bounded reads and drafts | predecessor command aliases, raw storage edits |
| Offline bootstrap | `bootstrap.rs` plus the embedded standard artifact | `new`, minimal/command recipes, built-in inspection/export, staged first publication | mutable template authority, network registry |
| Derived semantics | `semantic_summary.rs`, `semantic_query.rs` | persisted disposable module summaries, revision-bound reverse dependencies, invalidation-frontier oracle, revision-bound query indexes | a second accepted graph or independent writer |
| Review and recovery | `semantic_projection.rs`, repository backup/restore/retention preview | deterministic review, segmented exact backup, atomic restore, read-only cleanup inventory | writable text authority, canonical deletion |
| Compiler and artifact | `artifact.rs`, `execution/` | exact graph closure, explicit generic substitution, preparation, bytecode | credentials and deployment grants |
| Component model | graph-owned components, ports, requirements, targets | typed entries and required operation sets | sockets and application-specific native dispatch |
| Runtime | `runtime.rs`, `http.rs`, `worker.rs`, `stream.rs` | admission, task ownership, execution, cancellation, shutdown | routes, SQL, authorization, object keys, queue transitions |
| Capabilities | `execution/capability.rs` and generic adapters | requirement/grant equality, operation accounting, resource mechanics | domain permission policy |
| Deployment | `deployment.rs` and strict deployment JSON | concrete adapters, secret bindings, limits, listener topology | program meaning and artifact authority |
| Repository verification | `tools/check` | dependency DAG, bounded parallel gates, exact input fingerprints, fresh/reused evidence, bounded logs | accepted program authority and provider-cost inference |

## Authority, identity, and history

The sole accepted program authority is a graph-contract-4 revision. Its logical model contains
repository/package metadata, modules, exact dependencies, targets, tombstones, declarations,
types, expressions, relations, tests, components, and requirements. The physical
`StoredGraphRoot` is a bounded manifest containing six `MapRoot` values: modules by stable ID,
module names to IDs, dependencies by package ID, dependency aliases to package IDs, targets by
stable ID, and tombstones by typed identity. Those roots address canonical immutable
path-compressed Merkle radix pages. Module bodies remain independently content-addressed objects.

Repository, module, declaration, type-parameter, field, case, operation, parameter, binding,
expression, port, requirement, target, documentation, annotation, draft, and revision IDs use
distinct tagged domains. Names are mutable locators and indexed presentation. Canonical imports
bind exact package/module IDs, exports bind declaration IDs, typed expressions bind exact
package/module/declaration references, and targets bind exact module/component/port IDs. Module
and declaration rename are therefore local and do not rewrite importers or callers.

Accepted history is an immutable DAG. A revision binds exact parents, root digest, semantic
certificate, transaction digest, semantic diff, and receipt. The certificate authenticates the
exact graph-derived summary and reverse-dependency fact set without promoting disposable fact
files to program authority. Drafts are separate non-executable authority tied to one base. Holes
and conflicts cannot enter accepted HEAD. Review projections, embedded artifacts, index bytes,
compiler state, and deployment descriptors do not become accepted authority.

## Publication and physical locality

Preparation has local and complete paths. A precondition-free request may prepare locally when it
contains only eligible pure-function body replacements, only independent module creations, only
module renames, or only declaration renames. Body replacement resolves owners through the
revision-bound query index, validates selected modules plus recursively imported local
dependencies, and records removed nested identities as tombstone-map deltas. Independent creation
validates only the new empty modules. Module rename uses exact ID/name lookup and validates renamed
modules plus outgoing imports; importers and targets are not loaded or rewritten. Declaration
rename changes only owning modules and their summaries; exact-reference callers are not rewritten.
If eligibility cannot be proved, including mixed operations and every request with preconditions,
preparation reconstructs the logical graph, applies the transaction, canonicalizes relations, and
fully validates the complete candidate.

Either path produces one prepared validation bound to base root, result root, root delta, changed
modules, updated module summaries, reverse-dependency index, semantic certificate, and validation
facts. Eligible local preparation also carries a disposable exact-owner/name index delta bound to
the same base and result. It loads or rebuilds the base revision's certificate-matching semantic
index, applies exact summary replacements/removals, and rebinds it to the predicted revision. Under
the write lock publication rereads HEAD, rejects a changed base, replays the root delta, verifies
the prepared result and certificate bindings, and does not repeat semantic validation. It writes
immutable changed-module/page/root/receipt/revision objects and disposable summary/index bytes
before replacing HEAD once. Exact-index content objects precede their manifest; a cache-write
failure cannot change or prevent canonical publication. On Linux, new object data and directory
metadata are flushed with `syncfs` before the separately synchronized HEAD stage and rename. Other
targets use per-file synchronization.

The persistent root eliminates a monolithic module-reference payload as the physical accepted root:
equal maps have equal roots independent of insertion history, exact ID/name lookup traverses a
bounded path, and changed map paths are structurally shared. This is physical root locality, not a
claim that preparation is generally incremental. The four local classes avoid complete logical
reconstruction when their required disposable indexes are present. Missing semantic/query indexes
may rebuild broadly, and every fallback path still clones reconstructed logical vectors.

Root-delta mutation retains every generated path page in a private overlay, including a page whose
exact bytes already exist as an unreachable physical object. Final extraction walks only generated
pages reachable from changed map roots and skips unchanged map roots and accepted-base subtrees.
The publication lock, exact accepted base, and typed logical delta are the trust boundary: reused
subtree references originate in digest-checked accepted-base pages, while every generated page is
decoded, link-checked, and digest-checked before it is written. Ordinary local publication therefore
does not walk all persistent pages. External damage to an untouched accepted-base subtree can remain
latent until that subtree is read or deep doctor performs the exhaustive walk.

Deep doctor walks accepted history, verifies reachable module, map-page, root, receipt, and revision
bindings, reconstructs logical roots/module shape, and loads or rebuilds the current query and
semantic indexes. The rebuilt semantic certificate must equal the value in the current revision.
It does not currently rerun full cross-package semantic validation for every historical revision. Initial
publication runs complete direct-plus-packed validation, while restore verifies every entry and
runs deep structural/history doctor before visibility. Restore does not yet rerun the complete
cross-package semantic validator; copied-binary acceptance separately checks the restored fixture.
Focused differential tests retain the full oracle.

Backup contract 4 writes a manifest and bounded index segments, then copies canonical payload
objects one at a time. It includes HEAD's revision DAG, roots, pages, modules, receipts, exact
dependency artifacts, and live drafts; it excludes disposable indexes. Backup and restore retain
an O(object-count) sorted key set, so the payload path is segmented but not fully bounded-memory.
`doctor cleanup` uses the same HEAD-parent and live-draft-base reachability policy for a read-only
count-and-digest preview. It cannot delete and always reports `destructive_ready: false` until
revision pins, active-reader leases, and registered backup roots become explicit. Query indexes
are disposable and rebuildable.

## Summaries, queries, compiler, and runtime

Semantic summary contract 2 encodes per-module public signatures, implementations, effects, tests,
and typed dependency edges, bound to exact module content, package, and validator contract 2.
Summary objects are content-addressed below `indexes/summary-objects`; each accepted revision has a
revision-bound disposable `semantic-dependencies.lkix` generation. The reverse index and
invalidation-frontier implementation distinguish unchanged, private-implementation, and
public-signature changes. Local transaction paths delta-update the index and revisions authenticate
its fact set through `semantic_certificate`. This does not provide general frontier-driven
validation: the four admitted transaction classes are selected explicitly, while all other
changes use complete preparation. The complete validator remains the fallback and differential
oracle.

The query engine retains a revision-bound broad relation index and a local exact-index contract 3.
The exact index stores content-addressed owner/name shards independently of revision and binds 256
owner plus 256 name shard digest slots in a small revision/root manifest. Local changes rewrite
only touched buckets; full-candidate and initial publication seed from graph values already in
memory. Missing, stale, or corrupt query state rebuilds from canonical authority. Canonical
ordering and continuation meaning do not depend on physical page order. Broad relation-index delta
maintenance is not implemented.

Pure functions may declare explicit rank-1 type parameters with stable identities. Direct calls
and named pure function values require explicit type arguments, and `invoke` applies such a value.
Validation performs deterministic substitution; bytecode and the semantic reference interpreter
agree. There are no constraints, inference, generic task functions, lexical closures, or captured
environments.

One component/port model covers command, HTTP, interactive, batch, worker, and test runners.
Artifacts contain typed requirements, never grants or credentials. Deployment binds exact
requirements to generic adapters. The HTTP listener is plaintext and PostgreSQL uses `NoTls`.
Encrypted transport belongs at an external trusted boundary or a different adapter; lkjscript does
not plan TLS termination, PostgreSQL TLS, certificate management, ACME, or speculative TLS hooks.
That boundary does not provide hostile-code or multi-tenant isolation.

The source parser and source semantic builder remain Rust-test fixtures and independent oracle
material only. They cannot open, publish, build, or execute a maintained project and are not a
public authoring path.
