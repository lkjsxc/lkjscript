# Packages, components, targets, and artifacts

Status: normative for the typed meaning graph, package transport, compiler/bytecode behavior, and
artifact bundles. Internal compatibility identities remain at their typed source owners.

## Packages and exact dependencies

One typed meaning graph repository owns one root package. Repository ID, package ID, mutable package
name, semantic revision/state, logical package revision, package transport, public interface, artifact
manifest/bundle, compiler indexes, and filesystem location are distinct domains.

A package owns typed modules and other semantic owners, exact dependencies, components, ports,
targets, tests, documentation, and annotations. Stable identities survive supported rename and
move operations. Exact references bind package and typed owner identities, never ambient names or
physical object locations.

An accepted dependency records exact package ID, semantic revision, logical package revision, and
public interface binding. The dependency's package transport carries the complete validated public
meaning and exact transitive selection required to install that binding. It is immutable transport,
not editable source or an alternate package repository.

Public package interfaces retain exact-interface capability-resource types and operation parameter
use modes. Resource interface references must resolve inside the exact dependency closure and
participate in interface identity. A missing, foreign, wrong-kind, or predecessor resource/use
shape rejects before a dependent graph can be accepted.

Dependency resolution is closed and deterministic. The current released application lifecycle
accepts either no dependencies or the one exact built-in standard dependency. Missing, extra,
foreign, stale, truncated, duplicate, noncanonical, or interface-inconsistent closure rejects
before compilation or execution. There is no current general package registry, mutable tag,
ambient directory lookup, network resolution, or implicit upgrade.

## Built-in standard material

`packages/standard` is the sole maintained owner of two generated assets:

- an exact package transport used to create and validate dependent typed meaning graph repositories;
  and
- an exact artifact bundle used to link and execute the dependency.

The executable embeds both. Initialization strictly decodes each asset and verifies agreement on
package identity, semantic revision/state, logical package revision, public interface, compiler
contracts, artifact identity, and closure. Public inspect/export can observe or copy the exact
bytes but cannot replace them. Product verification regenerates the maintained files and compares
them byte for byte.

Public `add.dependency` accepts only this exact built-in package, semantic revision, and logical
package revision after its immutable transport has been staged through public export. It performs
no network, registry, ambient-directory, or unchecked-file lookup. The command, HTTP, and
relay-information recipes resolve their public declarations and interfaces from the same validated
inventory and install the same exact transport through authored change lowering. They retain no
name-only runtime resolution or recipe-only dependency path.

## Components, requirements, ports, and targets

A component is graph meaning that groups stable requirements and ports. Public
`create.component` creates one empty owner; separate `add.requirement` and function-backed
`add.port` records populate independently bounded children. A requirement binds one
stable ID, exact interface declaration, exact operation set, and resource limits. A port binds a
stable ID, function type, and exact function or expression implementation. Credentials, live
adapters, sockets, and deployment topology are excluded.

A target binds stable target identity and mutable target name to exact component and port
identities plus one runner kind. Public `create.target` accepts only `command` or `http`; the
released `run` command accepts only a pure command target.
Tests are graph-owned actual/expected expressions with exact comparison policy. Task functions may
name exact requirements; artifact linking retains their exact requirement owner closure without
treating a use by multiple task functions as duplicate semantic definition.

Validation rejects foreign domains, absent or retired owners, invalid type/effect shape,
requirement widening, unavailable operations, a port/function type disagreement, a target whose
port belongs to another component, a missing component requirement closure, incompatible ports,
and runner mismatch before publication or execution. Request-local forward references are resolved
within one normalized authored request and never weaken those checks.

## Compilation and artifact closure

Each selected semantic compiler owner lowers to a strict compiler-unit object. A compilation
manifest binds repository, exact revision/state, compiler and bytecode contracts, deterministic
optimization policy, and the complete unit map. Exact-current cache reuse requires every binding;
cache state is disposable and never enters semantic or package identity.

The artifact bundle links the root compilation with strictly loaded dependency artifacts. Its
manifest binds:

- root repository, package, accepted revision, and semantic state;
- every dependency package and logical revision;
- compiler, unit, optimization, and bytecode contracts;
- exact package interfaces and compiler-unit roots;
- exact runtime-owner metadata required by compiled relocations; and
- the complete immutable object closure and bundle checksum.

Artifact order and bytes are deterministic. The strict decoder checks bounds before allocation and
rejects unknown magic/version, noncanonical ordering, duplicates, foreign identity domains,
unresolved relocations, incorrect owner semantics, missing or extra objects, digest disagreement,
truncation, and trailing input.

Artifacts contain semantic requirements but never deployment grants, credentials, runtime handles,
host paths, or accepted repository visibility. An artifact is a derived executable input, not a
writer of typed meaning authority.

Compiler units preserve unrestricted/borrow/consume local-load decisions. Artifact metadata retains
the exact resource type, interface, operation parameter use, and requirement closure, but never a
live handle or private queue attempt tuple. Strict decoding rejects predecessor compiler,
bytecode, artifact, or package-interface forms before normalized execution.

## Preparation, execution, and deployment

`NormalizedProgram` maps exact artifact owners and compiler operands to compact process-local
indexes. These dense indexes are replaceable and do not become stable identities. Runtime resource
entries are additionally bound to one task scope, exact kind, interface, and acquiring requirement;
they cannot be reconstructed from ordinary values. Pure command and test execution must agree with
the canonical repository reference interpreter.

A deployment descriptor is external operational authority. It selects an exact artifact and target,
binds requirement aliases to generic adapters and secret/configuration sources, and sets resource
limits/topology. Application routes, data spaces/indexes/encodings, authorization, representations,
object keys, and queue transitions belong in graph meaning; Rust owns generic host mechanisms.

Current `serve` and `worker` descriptors select the maintained artifact bundle. Standalone
preparation strictly loads the bundle without project discovery, resolves the exact root-package
target, component, runner, and complete requirement closure, then binds external grants. Live
effects execute once through the normalized production VM; reference execution remains limited to
pure or deterministic oracle work and never repeats external effects.

The HTTP listener is plaintext. Encrypted transport requires an external trusted boundary or a
future explicitly selected adapter; no TLS or certificate machinery is implied by the component
model. The first-party data root is local trusted-host authority, not encrypted storage or a remote
database service.
