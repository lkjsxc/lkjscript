# Packages, modules, components, and targets

Status: normative for meaning graph contract 1.

## Repositories and packages

One semantic repository currently owns one root package and its accepted revision DAG. Repository
ID, package ID, package name, revision ID, root digest, artifact digest, and filesystem location are
separate domains.

A package owns exact metadata, modules, dependencies, targets, and exported meaning. A module owns
one namespace, imports, exports, documentation, annotations, and declarations. Package/module
names are mutable locators. Module IDs survive rename. Declaration IDs survive rename and move.

Imports resolve only within the root package or an explicitly aliased dependency. Visibility and
exports are validated over exact stable owners. Mutable tags, ambient directories, current working
directory lookup, undeclared network state, and environment-dependent dependency resolution are
forbidden in accepted builds.

## Exact dependencies

A dependency binding contains alias, immutable package ID, exact semantic revision ID, and exact
graph-artifact digest. Before an add/replace transaction, `semantic dependency-stage` verifies and
stages the artifact closure as unreachable immutable objects. Publication makes the binding
authoritative atomically with the root change. Removing a dependency requires that semantic
validation find no remaining use.

Graph artifacts contain a sorted unique closure of packed package objects. Every object binds its
revision record, receipt, graph root, and module set. The artifact identity commits to the exact
closure and compiler-facing contract; it does not contain a deployment grant, secret, or host
path.

## Components

A component is a graph-owned declaration that groups typed ports and capability requirements. A
requirement binds a stable requirement ID, local alias, exact interface, and operation set. A port
binds a stable port ID, name, exact external entry type, and graph-owned function expression.
Components contain no deployment credentials or live adapters.

The same model covers command, HTTP, interactive, batch, worker, and test runner kinds. These are
target metadata, not separate language editions or application profiles. A target has stable ID,
name, exact component module/declaration/port identities, retained locator names for diagnostics,
and runner kind. Validation rejects stale locator names or incompatible port shape.

## Preparation and deployment

Artifact preparation resolves every target, component, port, function, type, test, and capability
requirement to compact compiler/runtime indexes while retaining semantic provenance. Those indexes
are derived and never enter stable graph identity.

A deployment descriptor is external operational authority. It selects one exact artifact and
target, binds requirement aliases to generic adapters and secret/configuration sources, sets
resource limits, and supplies runner topology. The artifact declares requirements; deployment
grants authority. Generic Rust adapters must not contain application routes, schemas,
authorization roles, SQL/table policy, object keys, or queue transitions.

## Maintained packages

`packages/standard` is a graph-authored exact dependency package. It declares reusable typed
interfaces and closed external functions for core values, HTTP, JSON, PostgreSQL, configuration,
secrets, clock, randomness, identifiers, password hashing, streams, objects, and queues.

`applications/lkjournal` is the maintained service package. Its routes, SQL, migrations,
authentication/authorization, JSON/HTML representation, object naming/publication, and queue/job
transitions are graph meaning. It has HTTP target `serve` and worker target `work`; both consume the
same exact standard artifact binding.
