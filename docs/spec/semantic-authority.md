# Semantic authority

Status: normative. Accepted meaning is the typed meaning graph. Internal revision, receipt,
transaction, storage, and graph compatibility identities remain at their typed source owners and
are not product versions.

## Sole editable authority

An accepted lkjscript program is exactly one complete validated typed meaning graph revision selected by
the repository's atomic `HEAD`. The graph owns package metadata, typed semantic owners, interned
types, exact references, namespaces, relations, dependencies, components, ports, targets, tests,
documentation, annotations, and retirements.

Source text, compact authored records, request-local symbols, logical plans, package transports,
indexes, witnesses, compiler caches, artifacts, generated documentation, deployment descriptors,
names, runtime handles, logs, receipts outside accepted history, and host resources are not
separately editable program authorities.

Names are mutable locators. Typed stable identities express continuity. Logical semantic
references bind the appropriate exact package and owner identities; they do not acquire meaning
from names, module coordinates, physical map positions, or Rust representation. Content digests,
semantic revision IDs, dense runtime indexes, and filesystem paths occupy separate domains.

## Logical model and validation

A semantic snapshot contains one repository/package root and canonical typed maps for all owner,
type, dependency, namespace, relation, test, target, and retirement records. Every map key and
record validates its identity domain and owning relation. Canonical order is independent of hash
iteration, authored spelling, physical page layout, pack boundaries, and repository location.

The complete reconstruction oracle verifies:

- ownership, reachability, uniqueness, namespaces, visibility, and retirements;
- type structure, generic substitution, expressions, bindings, and function effects;
- exact local and dependency references and package interfaces;
- components, requirements, ports, targets, tests, and comparison policy; and
- forward/reverse relation witnesses and validation evidence against canonical meaning.

Missing required accepted meaning or inconsistent witness bindings are corruption. A disposable
cache miss is never allowed to select or repair semantics silently.

## Repository and accepted history

Immutable semantic objects and persistent-map pages are stored in content-addressed packs. The
catalog is a rebuildable physical locator. `HEAD` binds the repository, accepted revision record,
semantic state/root, witness/certificate, transaction, diff, and receipt. Physical packing and
catalog paths do not enter logical semantic state.

Accepted writes name an exact base, prepare and validate a complete result, write immutable data
durably, and expose one atomic `HEAD` visibility point. A revision is accepted only as a whole.
Stale, malformed, invalid, exhausted, cancelled, corrupt, interrupted, or failed work cannot
partially advance authority. After uncertain visibility, a caller reads `HEAD` and retained exact
acceptance evidence before retrying.

`GraphRepository::publish` is the sole normal existing-project writer. The released accepted-write
operations are:

- `new`, which constructs initial typed meaning authority in a private sibling and exposes it once; and
- `change apply`, which publishes one exactly reviewed prepared semantic change.

`change plan`, status, inspect, query, package inspection/export, check, build, run, capabilities,
and all their failure paths do not advance accepted authority.

## Reviewed authored changes

Every public mutation lowers to typed semantic intent before validation. Plan and apply share
strict decoding, normalization, deterministic allocation, exact-base reads, ownership/reference
analysis, impact and selected-test calculation, complete candidate validation, and logical-result
construction.

A reviewed token binds two separate commitments: normalized authored intent and every semantic,
validation, and test claim offered for review. Apply checks the intent commitment before project
access, reprepares against the exact base, checks the prepared commitment, and only then enters the
publication boundary. Optional logical-plan bytes are deterministic external evidence and are
never an apply input.

Request-local labels, operational budgets, witness maintenance, compiler scheduling, cache state,
storage packing, receipt paths, timing, and physical work observations do not enter logical plan
identity. They may affect admission or reporting but not accepted program meaning.

## Dependencies, compilation, and artifacts

An accepted dependency binding names exact package, semantic revision, logical package revision,
and public interface meaning. Package transport is strict immutable dependency transport, not an
authoring language. The current released resolver accepts only the exact built-in standard
dependency and rejects missing, foreign, stale, malformed, or additional dependency closure.

Compiler manifests, compiler units, and cache heads are derived from exact accepted authority. A
cache may be reused only when repository, accepted revision/state, compiler contract, options, unit
map, and object closure match. Missing cache state clean-builds. Invalid cache state is reported and
replaced by a clean build; it can never select semantics.

After an accepted `change apply`, an incremental cache update may use an exact base cache and the
in-memory prepared compiler impact. Publication is complete first. Incremental failure is reported
as derived-state status and cannot roll back, relabel, or invalidate the accepted semantic result.
No durable compiler-impact journal is authority.

An artifact bundle is immutable derived runtime input. Its manifest binds exact repository,
root package, semantic revision/state, dependency package revisions, compiler and bytecode
contracts, and object closure. Strict artifact validation precedes output publication or execution.
Equal authority, dependencies, compiler contracts, and options produce equal bytes.

## Execution and service separation

Normalized graph tests and pure command targets execute through both production bytecode and an
implementation-disjoint canonical reference interpreter. Both read the same exact accepted
revision and artifact closure. A disagreement is failure. Live external effects are never
duplicated to obtain a differential result.

`serve` and `worker` consume an explicitly selected immutable artifact bundle and external
deployment descriptors. Standalone preparation does not open editable graph authority or advance
`HEAD`. The bundle remains derived execution input; descriptors, secrets, grants, and host
resources remain external operational authority.

## Compatibility and security

Predecessor repository markers are rejected before mutation, cache work, or derived output. There is
no graph edition, migration command, compatibility flag, fallback reader, dual dispatch, or dual
write. Arbitrary predecessor conversion is not supported.

All paths, authored files, transports, artifacts, caches, continuations, deployment descriptors,
and runtime inputs are hostile bounded boundaries. The system does not claim hostile-code
sandboxing, multi-tenant isolation, encrypted graph storage, signed artifacts, distributed
consensus, or transport encryption.
