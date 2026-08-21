# Package, component, and artifact contract 1

This specification owns reusable package composition and runnable application description. Package
metadata, semantic validation, artifact encoding/decoding, and preparation are executable oracles.
It does not own deployment grants, credentials, process placement, or domain data.

## Packages and modules

`lkjscript.package.json` is strict JSON contract 1 with one package identity/name, 1 through 4,096
explicit modules, at most 1,024 exact dependencies, and at most 1,024 targets. Names are canonical
bounded ASCII tokens; module locators are relative slash paths beneath `src/`, with no empty, `.`,
`..`, absolute, backslash, or NUL component. Duplicate module names/paths, dependency aliases or
identities, and target names reject.

Module imports may form cycles because modules have no ordered side-effecting initialization.
Constants are pure and validated as declarations. Package dependency cycles reject. A dependency
binds alias, exact package identity, exact semantic revision digest, and exact artifact digest. Its
`artifact` field is a project-relative locator excluded from semantic dependency bytes. No build
consults a registry, network, `latest`, mutable tag, or ambient search path.

Visibility is export-based. References use local declaration, imported alias plus declaration, or
dependency import alias plus module/declaration. Re-export and feature selection are not present in
contract 1.

## Components and targets

A component owns an exact declaration identity, requirement set, and typed entry ports. A
requirement names one interface owner, a subset of its operations, and maximum semantic resource
limits. A port is a direct function reference with an exact function type. A target names a
component, port, and runner kind: command, HTTP, interactive, batch, worker, or test.

Runner kinds are adaptations of the same prepared port, not profiles with separate semantics. A
component has no implicit mutable state, transport context, or lifecycle callback. Per-call values
are explicit parameters; shared/durable state is reached only through declared capabilities.
Startup validates the complete component/grant relation. Runtime readiness and shutdown are runner
mechanics and cannot inspect private application state to make business decisions.

## Artifact

A `.lkja` artifact is canonical strict JSON contract 1 containing the root package and its complete
transitive exact package closure, module source, exact dependency edges, targets, and a domain
digest. It contains requirements but no grant, secret, environment name, listener, database URL,
object root, live handle, or development workspace path.

Artifact digest is BLAKE3 derive-key domain `lkjscript.component-artifact.v1` over canonical core
bytes; per-package artifact digest uses `lkjscript.package-artifact.v1`. Maximum artifact size is
128 MiB and package closure 1,024. Decode re-parses and semantically validates source independently,
checks every package/revision/artifact edge, rejects cycles/duplicates/missing closure, and verifies
the root and digest before preparation.

Build writes a deterministic artifact to a required-absent output. It does not publish source
history. Repeated builds from the same exact accepted closure must be byte-equal. Artifact
preparation creates disposable bytecode and indexes; cache corruption or loss must fall back to
re-preparation from the artifact.

Application contract 8/internal format 9 and typed, bytes-stream, stateful, and interactive profiles
are direct predecessors. Contract 1 has no decoder, alias, wrapper, or current execution route for
them.
