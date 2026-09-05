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
public interface binding. The dependency's package transport carries the complete validated current
graph, including private implementations, and exact transitive selection required to install that binding. It is immutable transport,
not editable source or an alternate package repository.

Public package interfaces retain exact-interface capability-resource types and operation parameter
use modes. Resource interface references must resolve inside the exact dependency closure and
participate in interface identity. A missing, foreign, wrong-kind, or predecessor resource/use
shape rejects before a dependent graph can be accepted.

Dependency resolution unifies the entire exact closure by package identity before acceptance or
execution. Shared dependencies are checked and compiled once in deterministic dependency order.
Missing edges, cycles, self-dependencies, conflicting logical or semantic revisions, foreign
references, and interface/body disagreement reject. Equal display names do not unify identities.
Only direct dependencies expose their public interface to a package; transitive availability grants
no ambient visibility. There is no registry, mutable tag, ambient directory lookup, network
resolution, or implicit upgrade.

## Offline transport and readiness

The public transport is one strict versioned deterministic uncompressed container binding a root
transport, an ordered exact transitive selection, and unique ordered canonical objects. Its bytes
are independent of original pack partitioning and directory order. It contains exactly the current
owner, type, blob, dependency, retirement, semantic-root, logical-revision, transport, and public
interface closure. Historical bodies, operational data, host paths, grants, caches, and executable
artifacts are excluded. Ancestor identities do not require historical bodies. Bare interface packs,
predecessor selections, missing/extra/duplicate objects, malformed/noncanonical/trailing input,
forged bindings, and executable artifacts reject before readiness.

Admission reconstructs every graph and public interface and runs complete canonical semantic
validation, including private bodies. Transported witnesses do not prove validity. Externals obey
the existing exact intrinsic signature/effect contract and cannot install host implementations or
grant capabilities. Integrity proves content agreement, not publisher identity, original acceptance
provenance, or source confidentiality.

One admission allows at most 268,435,456 container bytes, 1,000,000 distinct objects, 10,000 packages
including the root, 100,000 dependency edges, 16,000,000 aggregate owner/type/expression/relation
validation visits, and 4,294,967,296 cumulative validation-read bytes. Existing narrower object,
depth, authoring, output, and execution limits also apply. Counts and lengths are charged with
checked arithmetic before allocation or work, without resetting aggregate budgets per package.
Traversal is bounded and iterative with visited sets. There is no compression or public override;
these are safety ceilings, not demonstrated capacities.

`PACKAGE-TRANSPORTS` owns operational readiness. Private staging validates the complete container,
durably installs immutable material, then publishes one atomic closure-ready selection binding the
validation contract and all exact inputs. Readers see a complete old or new selection. Failure
before visibility preserves previous readiness and semantic HEADs and removes owned stages;
interruption may leave unreachable immutable objects. Retry after lost acknowledgement identifies
the complete staged result. Identical restaging is idempotent. Another revision is only a candidate;
physical reselection for one logical revision cannot change meaning.

Reviewed add/replace binds old/new exact dependencies, transitive changes, interface effects,
affected owners/tests, and the plan commitment. Apply rechecks the base, immutable source
availability, validation identity, and logical closure under the publication lock before accepted
visibility. Stale/altered plans, incompatible replacements, missing source, cancellation, and
exhaustion publish nothing. Post-publication derived failure reports acceptance separately.

## Built-in standard material

`packages/standard` is the sole maintained owner of two generated assets:

- an exact package transport used to create and validate dependent typed meaning graph repositories;
  and
- an exact derived artifact bundle whose equivalence to compilation from that graph is verified.

The executable embeds both. Initialization strictly decodes each asset and verifies agreement on
package identity, semantic revision/state, logical package revision, public interface, compiler
contracts, artifact identity, and closure. Public inspect/export can observe or copy the exact
bytes but cannot replace them. Product verification regenerates the maintained files and compares
them byte for byte.

Public `add.dependency` accepts an exact fully validated staged package, semantic revision, and logical
package revision. The embedded standard is a convenient source supplier using the same admission
and compilation mechanisms as any other package. It performs
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

A target binds stable target identity and mutable target name to one exact component plus one
runner kind. A `command` or `interactive` target owns one exact component port. An `http` target
has no universal port and instead owns a nonempty finite set of stable route owners, each binding
one exact method and one typed exact-path or whole-segment pattern selector to a component-owned
HTTP port. A pattern's capture names and order index the backing function's trailing unrestricted
`Text` parameters. Public `create.target` accepts exactly
`command`, `http`, or `interactive`; `run` accepts only a pure command target, while `serve` accepts
exact HTTP or interactive topology. The full route contract is specified in
[Signature-indexed inbound HTTP route topology](http-route-topology.md).
Tests are graph-owned actual/expected expressions with exact comparison policy. Task functions may
name exact requirements; artifact linking retains their exact requirement owner closure without
treating a use by multiple task functions as duplicate semantic definition.

Validation rejects foreign domains, absent or retired owners, invalid type/effect shape,
requirement widening, unavailable operations, a port/function type disagreement, a target or route
whose port belongs to another component, a missing component requirement closure, incompatible
ports, malformed or exhausted selectors, duplicate match languages, incomparable overlap,
signature drift, and runner mismatch before publication or execution.
An interactive port must reconstruct the
canonical `(Option<State>, SessionEvent) -> SessionDecision<State>` relation with one closed
ordinary concrete `State`; graph validation, package construction, compilation, strict artifact
loading, and deployment preparation each reject relation or retained-state drift. Request-local forward references are resolved
within one normalized authored request and never weaken those checks.

## Compilation and artifact closure

Each selected semantic compiler owner lowers to a strict compiler-unit object. A compilation
manifest binds repository, exact revision/state, compiler and bytecode contracts, deterministic
optimization policy, and the complete unit map. Exact-current cache reuse requires every binding;
cache state is disposable and never enters semantic or package identity.

Admitted dependencies compile from read-only immutable package views through the ordinary compiler
and linker, without writable shadow projects. Missing or invalid derived caches rebuild; absent or
corrupt canonical source rejects with an exact restage diagnostic. No checkout, ambient directory,
embedded-revision substitution, or transported compiled-unit fallback is allowed. The canonical
reference interpreter independently evaluates dependency owners, without consulting production
bytecode, callable resolution, value layouts, or closure selection. Its disposable callable, nominal
layout, type, target, and test indexes come from the reconstructed canonical owner inventory; strict
boundary codecs share only neutral value-schema access. Blob values also come from canonical source.
Every package's graph tests execute once, with the compiler test inventory checked against the
canonical inventory; command target selection remains root-scoped.

The artifact bundle links the root compilation with graph-derived dependency artifacts. Its
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

Current `serve` and `worker` descriptors select the maintained artifact bundle. `serve` dispatches
only an exact HTTP or interactive target; one descriptor cannot select both. Standalone
preparation strictly loads the bundle without project discovery, resolves the exact root-package
target, component, runner, and complete requirement closure, then binds external grants. Live
effects execute once through the normalized production VM; reference execution remains limited to
pure or deterministic oracle work and never repeats external effects.

The HTTP and RFC 6455 listener boundary is plaintext. Encrypted transport requires an external trusted boundary or a
future explicitly selected adapter; no TLS or certificate machinery is implied by the component
model. The first-party data root is local trusted-host authority, not encrypted storage or a remote
database service.
