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
transaction digest. The certificate is the revision-independent digest of the exact validated
module-summary and reverse-dependency fact set. Its domain-separated digest is the revision ID. A
revision record binds that core to one receipt. HEAD binds the repository, revision, and record
digest and is the single accepted visibility point. Persisted summary/index bytes remain
disposable; the certificate authenticates their rebuild against canonical meaning rather than
promoting them to a second graph.

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

All normal program mutations lower through the exact transaction-v4 evaluator. CLI-v4 entry points
that can publish accepted authority are:

- `new`, which creates initial authority in a private sibling stage;
- `change --commit`;
- `draft publish`;
- `history merge --apply`; and
- `restore`, which verifies and recreates the exact backed-up authority.

`change --dry-run`, queries, inspection, checks, builds, runs, review, package staging, and
built-in export do not publish. Package staging only verifies and stores unreachable exact
dependency objects for a later dependency-binding change.

A change-v3 request carries an optional exact base, optional idempotency key, ordered
preconditions, ordered high-level changes, an explicit work budget, and bounded nonsemantic intent.
Current high-level forms add/replace/remove a dependency; create a module, record, variant, pure
function, component, test, or target; rename a module or declaration; and replace a function body.
The exact internal transaction supports the additional lower-level operations required by drafts,
merge, tests, and maintained reconstruction without exposing physical records.

Dry-run and commit share change normalization and lowering. Four precondition-free transaction
classes may prepare locally: eligible pure-function body replacements, independent empty-module
creation, module rename, and declaration rename. Body replacement resolves exact owners, validates
selected modules plus their recursive local import dependencies, and emits exact tombstone deltas
when a structurally different body removes nested identities. Independent creation validates the
new empty modules. Module and declaration rename use exact root lookup and validate owning modules
plus outgoing import dependencies without loading or rewriting importers or targets. Preconditions,
mixed operations, selection uncertainty, index failure, and every other operation fall back to
complete logical reconstruction, canonicalization, and validation. Building a missing disposable
index may itself require complete reconstruction.

Either path carries a prepared result bound to the exact base root, result root, root delta,
changed module set, summary delta, reverse-dependency index, semantic certificate, and validation
facts. Under the write lock publication rereads HEAD, rejects a stale base, reapplies the delta to
the accepted stored root, verifies the result and certificate bindings, writes immutable
module/map-page/root/receipt/revision/dependency objects durably, and replaces HEAD once. It does
not repeat the completed semantic validation.

Semantic-summary contract 2 persists content-addressed module summaries and a revision-bound
reverse-dependency index as disposable acceleration. The four local paths update these facts by
delta. Missing or malformed cache bytes rebuild from canonical modules; a rebuilt certificate
that differs from the accepted revision is corruption. The reverse-dependency frontier does not
yet select general validation. Packed reconstruction and the complete validator remain explicit
full-oracle and focused differential-test routes; deep doctor separately walks retained
object/history bindings and checks the current rebuilt certificate.

## Outcomes, ordering, and failure

Changes execute in request order after strict decoding and precondition evaluation. Canonical sets
and diagnostics use deterministic order. Closed transaction outcomes distinguish accepted change,
semantic no-change, exact replay, stale base, failed precondition, foreign identity, invalid graph,
and resource exhaustion. Malformed protocol, corrupt authority, cancellation, capability failure,
and infrastructure failure remain distinct diagnostics.

Validation, dry-run, query, stale input, rejection, no-change, and failed restore publish nothing.
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
