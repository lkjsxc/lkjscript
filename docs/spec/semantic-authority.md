# Meaning-graph authority

Status: normative. Logical contract: `lkjscript-meaning-graph-4`. Physical root contract:
`lkjscript-persistent-root-2`. Public change contract: version 3. Internal transaction contract:
version 4.

## Owned authority

An accepted lkjscript program is exactly one validated typed meaning-graph revision. The graph owns
repository and package metadata, modules and namespaces, declarations, explicit types and
expressions, imports/exports, components, ports, capability requirements, tests, retained semantic
relations, documentation, annotations, targets, exact dependencies, and continuity tombstones.

Names are mutable locators and presentation. Stable semantic IDs express continuity. Content
digests identify exact bytes in one domain. Revision IDs identify accepted history nodes. Physical
page coordinates, compiler indexes, runtime handles, rendered coordinates, summaries, caches,
deployment grants, secrets, and live resources do not become program authority. Reachable map-page
bytes are the canonical physical encoding committed by the accepted root, but their paths are not
semantic owner identity.

Maintained lkjscript source files and package descriptors are forbidden. A review file is a
non-authoritative projection and has no apply path. The embedded standard artifact is exact
derived bootstrap data, not mutable authority. A backup transports accepted authority but is not a
writer until verified restore publishes it.

## Logical and physical models

The logical `GraphRoot` contains one repository ID, one package ID and name, and deterministic
sets of module references, exact dependency bindings, targets, and tombstones. Each module
reference binds a module ID and current name to an immutable module object. Module objects own
declarations, stable member identities, typed operation trees, documentation, annotations, and
sorted semantic relations.

The accepted revision does not encode those root sets as one flat physical vector. Its
`StoredGraphRoot` is a bounded manifest with exact metadata and six persistent map roots:

- module ID to module object reference;
- module name to module ID;
- package ID to exact dependency binding;
- dependency alias to package ID;
- target ID to target binding; and
- typed tombstone identity to tombstone.

The maps use canonical immutable path-compressed Merkle radix pages. Equal logical maps produce
equal page/root digests independent of insertion history. Physical pages are integrity objects, not
semantic owners. Full logical reconstruction remains the independent representation used by deep
doctor and complete validation.

Validation and canonical relations bind stable owner identities. Imports store exact package and
module IDs, targets store exact module/component/port IDs, and types and value references store
exact package/module/declaration IDs. Exports are declaration-ID sets, and constant references are
distinct from lexical variables. Module and declaration rename therefore update their owning
module plus persistent name/summary paths; importer objects and targets remain unchanged.

## Identity domains

Stable IDs have closed textual prefixes and packed tags. Foreign domains reject even when display
bytes coincide. Creation never silently reuses an ID; deletion records a typed tombstone when
continuity requires it. Clone creates new identities. Exact restore preserves historical identity.

| Domain | Text prefix | Continuity consumer |
|---|---|---|
| repository | `repo_` | accepted store and exact restore |
| module | `mod_` | rename and namespace continuity |
| declaration | `decl_` | rename, move, references, diff/merge |
| type parameter | `typeparam_` | generic substitution and rename |
| field | `field_` | record evolution |
| variant case | `case_` | variant evolution |
| interface operation | `op_` | capability operation evolution |
| value parameter | `param_` | signature member continuity |
| binding | `bind_` | selected body rewrites |
| expression site | `expr_` | exact expression selection |
| requirement | `req_` | component capability continuity |
| component port | `port_` | target binding |
| target | `target_` | deployment selection |
| documentation | `doc_` | retained documentation continuity |
| annotation | `annotation_` | retained annotation continuity |
| draft | `draft_` | non-executable work authority |
| conflict | `conflict_` | one typed merge-conflict report |

Revision IDs use the content-derived `rev_` domain. Package IDs, object digests, map-page digests,
compiler indexes, runtime handles, and temporary local symbols remain separate domains.

Production project identities use fresh allocation. A change-v3 request may define a typed local
symbol beginning with `$` and refer to it later in the same ordered request. Stable allocation is
deterministically bound to repository, exact base, normalized request, domain, and request order.
The public change result returns the complete local-symbol map. Duplicate, forward, ambiguous, or
foreign-domain use rejects; exact replay preserves allocation.

## Revisions, drafts, and accepted history

A revision core binds its current contract versions, repository ID, zero to two exact parent
revision/record pairs, persistent root digest, semantic certificate, semantic diff digest, and
transaction digest. The certificate is the revision-independent digest of the exact persistent
summary-binding, test-owner, and typed reverse-edge map roots. Its domain-separated digest is the
revision ID. A revision record binds that core to one receipt. HEAD binds the repository, revision,
and record digest and is the single accepted visibility point. Persisted summary objects, fact
pages, and manifests remain disposable; the certificate authenticates their rebuild against
canonical meaning rather than promoting them to a second graph.

Accepted revisions are complete: they contain no holes or conflicts, and every reference resolves
within the exact dependency closure. Names, scopes, types, generic substitution, effects,
capabilities, components, targets, tests, identities, and canonical relations validate.

History is an immutable DAG. Ordinary accepted publication has one parent, merge publication has
two unique canonically ordered parents, and bootstrap has none. Bounded intent belongs to the
receipt and is nonsemantic.

A draft is separate packed non-executable authority. It binds one repository, exact base,
generation, ordered transactions and preconditions, typed holes, closed conflicts, and bounded
intent. Draft mutation cannot alter HEAD. A draft with holes or conflicts cannot build, run, serve,
start a worker, or publish. Rebase is explicit; drop cannot affect accepted authority.

## Sole normal writer and publication

Normalized program mutations lower one typed `AuthoredChangeSet` through deterministic identity
allocation, primitive edits, canonical deltas, derived witness deltas, impact selection, incremental
validation, and required full-oracle policy. `GraphRepository::publish` is the sole visibility
boundary for an existing normalized repository. The current normalized CLI entry points that can
publish are:

- `new DEST --template minimal`, which creates initial authority in a private sibling stage; and
- `change apply ... --plan TOKEN`, which publishes one exactly reviewed prepared change.

`change plan`, including optional external logical-plan output, status, inspection, normalized
query, and capability discovery do not publish.
Draft, history merge, restore, package, check, build, run, review, service, worker, and doctor still
target predecessor authority and cannot mutate a normalized repository; their normalized cutover
remains required.

A compact-record request carries an exact base, optional idempotency key, ordered high-level
changes, and bounded nonsemantic intent. Direct `rename.owner` flags carry the equivalent exact
base, exact typed owner, name, and optional controls. Both adapters construct one transport-neutral
`AuthoredChangeSet` plus publication options before reviewed-plan comparison or repository access.
The public vocabulary and current subset are executable-derived from `capabilities --section
change`, `type`, and `expression`. The typed engine retains additional private operations and
explicit multidimensional work budgets, but those are not public grammar until their complete
public workflows exist.

Plan and apply use the same parser, typed lowering, impact analysis, validation, predicted revision,
semantic diff, and allocation path. The canonical `plan_` token combines a request commitment that
can be checked before repository discovery with a prepared-plan commitment over the complete
logical review projection. Apply reparses, checks the request component, reprepares the request,
and checks the prepared component before publication. Function-body replacement updates the
function and retires its complete prior expression/binding ownership closure; the logical plan
lists those exact owner and relation removals and never leaves live unowned nodes. A stale,
mismatched, exhausted, corrupt, or invalid request publishes nothing.

The optional logical-plan file is deterministic derived evidence, not repository state or a second
program authority. Its commitment covers exact semantic effects and exported validation/test
scope. Request-local labels, witness maintenance, summary refresh, compiler scheduling, staged
storage, physical roots/pages/packs, receipt work, timing, and filesystem paths remain outside
review identity. Apply never trusts or imports file bytes: it recomputes the plan from accepted
authority and the authored request. A complete external file may survive an interruption after its
atomic rename without implying that HEAD advanced.

Prepared publication binds the repository, package, exact base revision, canonical semantic root,
dependency bindings, validation contract and evidence, semantic diff, transaction, and publication
receipt in separate typed digest domains. Under the publication lock, the repository rechecks the
base, makes immutable semantic and evidence objects durable, publishes their bindings, and advances
HEAD once. HEAD locates both accepted meaning and the exact durable acceptance binding. Rebuilding
derived witness maps, summaries, indexes, packs, or validation evidence does not silently change
semantic meaning identity.

Normalized public query opens one immutable current `RepositoryView` and retains its repository,
package, and accepted-revision binding for the operation. Owner enumeration reads canonical owner
bindings and owner objects. Exact name lookup uses the committed namespace witness only as a
bounded locator, then requires the selected live canonical owner to reproduce the namespace key.
Relation lookup reads the committed forward or reverse relation witness and strictly revalidates
each selected key and empty value. These authenticated witnesses are required accepted read
evidence, not independently editable meaning. Missing or inconsistent required evidence is
corruption and is never silently rebuilt by query.

Query results, query digests, and stateless logical continuations are transient projections. They
cannot advance HEAD, create a draft, write an index, persist a cursor, or authorize a foreign
repository read. Full canonical reconstruction and canonical relation extraction remain
implementation-disjoint verification oracles rather than ordinary query dependencies.

## Outcomes, ordering, and failure

Changes execute in request order after strict decoding and precondition evaluation. Canonical sets
and diagnostics use deterministic order. Compact results distinguish prepared, accepted, already
accepted, stale base, source or semantic invalidity, resource exhaustion, corruption, and
infrastructure failure. Stale-base failures use process exit 7; other classes retain the executable
exit mapping. Diagnostics carry a stable code and available physical record location.

Validation, planning, query, stale input, rejection, no-change, and failed restore publish nothing.
An uncertain visibility failure requires reading current HEAD and retained receipts before retrying;
blind replay is forbidden.

## Bounds and security assumptions

Request, object, decoder, expression-depth, transaction-work, affected-owner, and finite-output
bounds are checked before untrusted growth. Persistent map keys are at most 256 bytes, values at
most 48 KiB, target leaf pages 16 KiB, and hostile page inputs 64 KiB; larger semantic data must
remain in independently addressed objects. These are storage/decoder limits, not public counts of
modules in a package.

Current logical module and transaction containers still have implementation bounds documented in
`docs/status.md`. Growing public results paginate or write to an explicit output. Raising a bound
does not substitute for incremental algorithms.

The local operator, executable, host OS, and filesystem durability behavior are trusted. The model
does not claim hostile-code isolation, encrypted storage, authenticated artifact provenance,
distributed consensus, or multi-tenant publication.

## Compatibility and non-goals

There is no compatibility edition, alias, fallback reader, writable text syntax, storage-record
authoring API, ambient dependency resolution, or network registry in the authority contract. A
future graph-contract change requires direct reconstruction, complete maintained-consumer cutover,
predecessor rejection, and deletion of the former current reader. Git history may retain historical
bytes; current execution does not interpret them.
