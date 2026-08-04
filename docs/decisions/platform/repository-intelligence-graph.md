# Repository Intelligence Graph And Context

## Purpose
Define the Current complete deterministic repository graph and bounded query boundary
without making a generated index semantic authority.

## Status

**Current immediate correctness cut.** `structure graph` publishes a complete in-memory graph or typed exhaustion.
`structure explain`, `structure context`, `structure impact`, and `structure tests` use that complete graph and expose
structured bounded completion. Incremental rebuilds, sharding, and continuation cursors remain Deferred.

## Authority And Identity

The graph is a derived index. Authored source, accepted decisions, Semantic
Source, compiler facts, manifests, public facts, and evidence remain their own
authorities. A graph result cannot manufacture typing, ownership, proof,
implementation, status, or evidence.

The graph uses stable identity `lkjscript.repository-graph` and its exact full contract digest. Each successful build
records the base Git revision and a SHA-256 identity over complete nodes, edges, contract, revision, charged work, and
retained node/edge field bytes. Node revision IDs bind that identity.
Unextracted file bytes are not semantic input; unsupported extraction classes remain explicit.

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

## Bounds And Completion

Policy sets 32,768-node and 65,536-edge implementation safety maxima, plus separate graph work and retained-byte maxima.
Producers charge work and exact retained node/edge field bytes. Canonicalization sorts, deduplicates, and rejects
conflicting IDs, verifies every endpoint, and returns typed exhaustion with the dimension, used amount, attempted
amount,
limit, and no-publication guarantee. It never selects a successful prefix.

Query traversal is iterative and uses checked work and retained-byte charging. Results contain `complete` or `bounded`,
closed stop reasons, deterministic omitted frontier, ordering, selected limits, and `continuation_supported=false`.
Exact pretty-printed output is measured before publication; oversized output fails rather than publishing a partial
result. Unsupported extraction classes remain separate from bounded query completion.

These charges bound the Current in-memory implementation; they do not claim every future parser allocation or wall-clock
cost is represented. The affected taxonomy is available through `lkjscript-xtask limits --json`.

## Context Sections

Current context output uses a deterministic section order covering goal,
revision/profile, capsule card, interfaces, status, exclusions, evidence,
projections, rules, implementations, source facts, dependencies, dependents,
tests, decisions/status, provenance, and omissions. A section may be empty.
The checker does not infer evidence freshness or nearest next work.

Impact follows a focused relation set for facts so an affected fact reaches its registered projections and dependent
facts without inheriting unrelated capsule closure. Tests queries follow retained test relations. Any budget stop is a
bounded result with an exact frontier; it cannot masquerade as complete impact.

## Verification

Focused tests require deterministic repeated complete builds, stable IDs, graph identity changes across revision inputs,
evidence-bearing extracted edges, endpoint closure, typed exact-limit exhaustion, public-fact projection impact, fixed
context section order, structured query completion, and serialized output bounds. `structure check` enforces repository
topology; it does not prove every graph edge semantically.

## Deferred And Rejected

Rich orientation/change/diagnostic/review/handoff profiles, complete type-use and
macro-expanded Rust edges, complete architecture projection, classified example
edges, evidence-freshness ranking, embeddings, remote graph services,
cross-repository federation, and probabilistic ranking are **Deferred**.

Treating graph data as type, proof, status, implementation, or evidence authority; unlimited traversal; successful
global truncation; hidden query omissions; uploaded private context; and tracked generated graph output are
**Rejected**.
