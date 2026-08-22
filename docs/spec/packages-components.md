# Packages, modules, components, and targets

Status: normative for meaning graph contract 4, package artifact contract 3, and executable artifact
contract 4.

## Repositories, packages, and modules

One repository owns one root package and its accepted revision DAG. Repository ID, package ID,
package name, revision ID, persistent root digest, artifact digest, and filesystem location are
separate domains.

A package owns exact metadata, modules, dependencies, targets, and exported meaning. A module owns
one namespace, imports, exports, documentation, annotations, declarations, identities, and
relations. Package/module names are mutable locators. Module IDs survive rename; declaration IDs
survive rename and move.

Semantic relations bind exact stable package/module/declaration/member identities after validation.
Canonical imports bind exact package and module IDs while retaining only a module-local alias.
Canonical targets bind exact module, component declaration, and port IDs. Module rename therefore
updates the module object and name map without rewriting importers or targets. Exports bind
declaration IDs and expression references bind exact package/module/declaration IDs. Declaration
rename therefore updates only its owning module and derived name/summary entries; callers retain
the exact binding.

Imports resolve only within the root package or an explicitly aliased exact dependency. Visibility
and exports validate over stable owners. Mutable tags, ambient directories, current working
directory lookup, undeclared network state, credentials, and latest-version resolution are
forbidden in accepted builds.

## Exact dependencies and built-ins

A dependency binding contains alias, immutable package ID, exact semantic revision ID, and exact
graph-artifact digest. Before add or replace, `package stage PATH` verifies the package artifact
closure and stores its immutable objects as unreachable operational data. A committed change-v3
`add_dependency` or `replace_dependency` makes the exact binding authoritative with the root.
`remove_dependency` rejects while accepted meaning still uses it.

Graph artifacts contain a sorted unique closure of packed package objects. Every package object
binds its revision record, receipt, canonical logical graph root, and module set; artifact transport
is deliberately independent of repository page coordinates and packing. Artifact identity commits
to exact closure and compiler-facing contracts. Artifacts contain requirements, never deployment
grants, secrets, live resources, or host paths.

The executable carries one exact standard artifact for offline bootstrap. It is derived from the
maintained `packages/standard` authority and cannot be replaced by an ambient file.
`package builtin inspect` exposes its contract/package/revision/artifact/digest/size;
`package builtin export --output PATH` exports identical checked bytes. A command-template
project binds that exact package through the same dependency contract used by any project.

## Components, requirements, and ports

A component is a graph-owned declaration grouping typed ports and capability requirements. A
requirement binds a stable requirement ID, local alias, exact interface, and operation set. A port
binds stable port ID, name, exact external parameter/result shape, and one graph-owned named
function expression. Components contain no deployment credentials or live adapters.

The same component/port model covers command, HTTP, interactive, batch, worker, test, and related
runner kinds where their entry contracts align. Runner kind is target metadata, not a language
edition. A target has stable ID and name, exact module/component/port identities, and one runner
kind. Current names are derived for inspection. Validation rejects foreign identities, stale
caller-supplied expected-name preconditions, requirement mismatch, or incompatible port shape.

Pure generic functions are instantiated explicitly before use in a port or another expression.
Named function values carry stable declaration provenance and no closure environment. Task ports
may bind named task functions only through component validation with their declared requirements;
generic task functions and captured closures are not allowed.

## Preparation, execution, and deployment

Artifact preparation resolves every target, component, port, function, explicit type application,
test, and capability requirement to compact runtime indexes while retaining semantic provenance.
Prepared indexes and specialization choices are derived and never enter stable graph identity.
Bytecode and semantic reference execution must agree.

A deployment descriptor is external operational authority. It selects an exact artifact and target,
binds requirement aliases to generic adapters and secret/configuration sources, sets resource
limits, and supplies runner topology. The artifact declares requirements; deployment grants
authority. A descriptor cannot change accepted program meaning.

Generic Rust adapters must not contain application routes, schemas, authorization roles, SQL/table
policy, object keys, retry/domain transitions, or rendering policy. Every live resource is owned,
bounded, cancellable, and closed under the runtime contract.

The HTTP adapter listens in plaintext. PostgreSQL connects with `NoTls`. lkjscript does not plan
HTTP TLS termination, PostgreSQL TLS, certificate parsing/management/rotation, ACME, or speculative
TLS capability hooks. Encrypted transport requires an appropriate external trusted boundary or a
different adapter outside current scope. That boundary does not provide hostile-code or
multi-tenant isolation.

## Maintained packages

`packages/standard` is graph-authored exact dependency meaning. It declares reusable typed
interfaces and closed external functions for core values, HTTP, JSON, PostgreSQL, configuration,
secrets, clock, randomness, identifiers, password hashing, streams, objects, and queues. Its
generic pure identity function is a maintained consumer of explicit rank-1 type parameters.

`applications/lkjournal` is the maintained service package. It binds the exact standard revision
and package artifact. Routes, SQL, migrations, authentication/authorization, JSON/HTML
representation, object naming/publication, and queue/job transitions remain graph meaning. Its
`serve` target selects `service.Web/request`; its `work` target selects
`worker.Worker/run`.

The command bootstrap is an additional ordinary consumer: it calls the exact built-in standard
identity function with explicit `Text` and owns a graph test, component, port, and command target.
No private Rust application builder or external template artifact is required at runtime.

## Ordering, failure, persistence, and non-goals

Modules, dependencies, exports, targets, requirements, ports, and artifact objects use canonical
deterministic order. Unknown/foreign IDs, dependency mismatch, unexported use, stale locator,
capability widening, invalid runner shape, corrupt artifact, and resource exhaustion reject before
accepted publication or runtime admission.

Accepted package/dependency/component/target meaning persists in the graph. Package staging,
embedded bytes, prepared indexes, deployments, grants, secrets, and live resources do not.

There is no online registry, mutable-tag resolution, implicit dependency upgrade, application-
specific native policy, hidden source macro, TLS subsystem, or certificate capability in this
contract.
