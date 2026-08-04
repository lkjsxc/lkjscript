# Repository Intelligence Graph And Context

## Purpose
Define the Current deterministic, bounded repository graph and query boundary
without making a generated index semantic authority.

## Status

**Current.** `structure graph`, `structure explain`, `structure context`,
`structure impact`, and `structure tests` are implemented in `lkjscript-xtask`.
The richer selectors, profiles, diagnostics, and derived documentation described
under Deferred are not Current.

## Authority And Identity

The graph is a derived index. Authored source, accepted decisions, Semantic
Source, compiler facts, manifests, public facts, and evidence remain their own
authorities. A graph result cannot manufacture typing, ownership, proof,
implementation, status, or evidence.

The graph uses stable identity `lkjscript.repository-graph` and its exact full
contract digest. Each build records the base Git revision and a SHA-256 identity
over its canonical retained nodes, edges, contract, revision, and budget state.
Node revision IDs bind that graph identity, so a dirty or staged graph does not
claim to be the unchanged base commit.

## Retained Records

Each retained node contains:

- a stable kind-prefixed ID and graph-scoped revision ID;
- kind and compact label;
- provenance and authority;
- an optional span; and
- declared, derived, or inferred confidence.

Current producers add bounded repository, directory, file, capsule, Cargo,
Rust, Markdown-link, lkjscript declaration, command, rule, public-fact, status,
interface, exclusion, and evidence records where the corresponding authority is
available. Unsupported extraction classes are listed in graph output.

Each edge contains source ID, destination ID, kind, evidence identity, and
confidence. Producers emit containment, dependency, declaration, import, link,
command, authority, implementation, evidence, projection, status, exclusion,
and public-fact dependency/invalidation relations. The graph preserves producer
confidence; inferred edges do not become declared facts.

## Commands And Output

```text
cargo run --locked -p lkjscript-xtask -- structure graph [--json|--dot]
cargo run --locked -p lkjscript-xtask -- structure explain <rule-path-or-fact>
cargo run --locked -p lkjscript-xtask -- structure context <target> [--profile weak|strong]
cargo run --locked -p lkjscript-xtask -- structure impact <target>
cargo run --locked -p lkjscript-xtask -- structure tests <target>
```

`structure graph` writes canonical JSON and DOT files below
`target/lkjscript/structure/` and optionally prints one representation. Query
commands print deterministic JSON to standard output. Context supports only the
`weak` and `strong` profiles. Targets are exact graph IDs, fact IDs, paths, or
bounded structural matches implemented by the query traversal.

Production graph and query commands validate the public-fact registry once and
pass that validated value into graph construction. Malformed facts therefore
cannot produce a graph with silently omitted fact edges.

## Bounds And Truncation

Policy sets graph node, edge, work, and charged-byte limits and separate query
work and output-byte limits. Graph producers charge bounded work and retained
field bytes. Canonicalization sorts and deduplicates records, truncates at node
and edge limits, and drops edges whose endpoints were not retained.

Query traversal is iterative, uses checked work and byte charging, and retains a
conservative fraction of the serialized output budget. The command then measures
the exact pretty-printed JSON before publication. Oversized output fails rather
than crossing the configured limit. Truncation and unsupported classes remain
explicit in successful output.

These charges bound the Current implementation; they are not a claim that every
future parser allocation or wall-clock cost is represented. Typed exhaustion
diagnostics with attempted charges and responsible edges remain Deferred.

## Context Sections

Current context output uses a deterministic section order covering goal,
revision/profile, capsule card, interfaces, status, exclusions, evidence,
projections, rules, implementations, source facts, dependencies, dependents,
tests, decisions/status, provenance, and omissions. A section may be empty.
The checker does not infer evidence freshness or nearest next work.

Impact follows a focused relation set for facts so an affected fact reaches its
registered projections and dependent facts without inheriting unrelated capsule
closure. Tests queries follow retained test relations. A globally truncated
graph remains explicitly truncated even when a focused retained route succeeds.

## Verification

Focused tests require deterministic repeated builds, stable IDs, graph identity
changes across revision inputs, evidence-bearing extracted edges, public-fact
projection impact, fixed context section order, retained-edge truncation, and
serialized query bounds. `structure check` enforces repository topology; it does
not prove every graph edge semantically.

## Deferred And Rejected

Rich orientation/change/diagnostic/review/handoff profiles, complete type-use and
macro-expanded Rust edges, complete architecture projection, classified example
edges, evidence-freshness ranking, embeddings, remote graph services,
cross-repository federation, and probabilistic ranking are **Deferred**.

Treating graph data as type, proof, status, implementation, or evidence
authority; unlimited traversal; silent truncation; uploaded private context; and
tracked generated graph output are **Rejected**.
