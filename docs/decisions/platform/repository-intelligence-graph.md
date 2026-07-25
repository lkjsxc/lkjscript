# Repository Intelligence Graph And Context

## Purpose

Define a deterministic, bounded repository graph and context-selection boundary
for humans and agents without making generated indexes semantic authority.

## Status

**Current.** The bounded graph and `structure graph`, `structure context`,
`structure impact`, and `structure tests` query interfaces are implemented in
`lkjscript-xtask`. They consume validated repository/source authorities.

## Authority And Identity

The graph is a derived index. Authored source, accepted decisions, validated
Semantic Source, compiler IR, manifests, and retained evidence remain their own
authorities. A graph answer identifies its source authority and revision; it
cannot manufacture type, effect, ownership, proof, implementation, or evidence
facts.

The graph identity is `lkjscript.repository-graph`, version `1`. A graph build
also records repository revision, topology-manifest identity, policy identity,
Semantic Source foundation/schema identity where present, compiler build
identity, and context-profile identity. Unknown names, versions, node kinds,
edge kinds, provenance values, and fields fail closed.

## Nodes

V1 has closed node kinds for repository, directory, file, document authority,
semantic capsule, heading, decision, source unit, declaration/entity, test,
command evidence, retained artifact, rule, diagnostic, task, and generated
artifact. Every node contains:

- a kind-prefixed stable identity and revision-scoped dense ID;
- exact provenance: `authored`, `generated`, `vendored`,
  `immutable-evidence`, or `build-artifact`;
- authority path and exact byte-span or semantic identity when applicable;
- content/fingerprint identity, status, and policy labels; and
- deterministic compact summary plus budget charge.

Stable identity uses length-framed version, kind, authority identity, and local
key bytes. Dense IDs are deterministic only within one exact graph revision.
Content digests are stale checks, not authorization or semantic identity.

## Edges

V1 has closed directed edge kinds: `contains`, `declares`, `links-to`,
`imports`, `depends-on`, `implements`, `verified-by`, `evidenced-by`,
`generated-from`, `supersedes`, `accepted-after`, `blocks`, `owns-task`, and
`relates`. Each edge records provenance, source authority, exact origin, and a
confidence class of `declared`, `compiler-derived`, or `heuristic`.

Only manifest, parser/compiler, test registry, and exact link extraction may
emit declared or compiler-derived edges. Heuristic edges are opt-in context
hints, never implementation or verification claims. Contradictory status or
provenance is a diagnostic rather than last-writer-wins data.

## Build And Query Boundary

The Current commands are:

```text
cargo run --locked -p lkjscript-xtask -- structure graph [--json|--dot]
cargo run --locked -p lkjscript-xtask -- structure context <target> [--profile weak|strong]
cargo run --locked -p lkjscript-xtask -- structure impact <target>
cargo run --locked -p lkjscript-xtask -- structure tests <target>
```

Canonical graph JSON is written only under `target/`. Requests and responses
use strict bounded envelopes and deterministic ordering. Queries select by
identity, kind, path, status, provenance, edge traversal, or exact compiler
fact. Full-text or heuristic retrieval is separately labeled and cannot replace
identity filters.

## Aggregate Budgets

A build precharges and checks input bytes, paths, directories, files, nodes,
edges, spans, strings, link targets, source entities, evidence references,
compiler-fact records, traversal work, sort work, output bytes, and peak retained
index bytes. A query separately charges request bytes, seeds, visited nodes,
visited edges, result nodes, result bytes, traversal depth, ranking work, and
wall/fuel policy. Checked arithmetic precedes allocation or indexing.

Exhaustion returns identity `lkjscript.repository-graph-diagnostic`, version `1`,
with category, limit, attempted charge, profile, and responsible node/edge when
available. Partial output never receives graph authority.

## Context Profiles

Profiles are versioned deterministic selectors, not prompt-size suggestions:

- `orientation`: authority indexes, status vocabulary, architecture, and read
  order;
- `change`: task scope, owning authorities, direct dependencies, applicable
  decisions, tests, and recent evidence;
- `diagnostic`: failing rule/diagnostic, producer, affected entities, and
  verifying tests;
- `review`: changed authorities, inbound/outbound semantic edges, status changes,
  policy coverage, and evidence; and
- `handoff`: task state, accepted next contract, blockers, touched authorities,
  commands, and untested gates.

Every response names the profile/version, exact seeds, inclusion reasons,
omissions caused by budgets, sort key, source revisions, and total charges.
Truncation is explicit and deterministic. Context may quote immutable evidence
but must not rewrite it.

## Provenance And Freshness

A generated node retains generator identity and all input identities. Vendored
nodes retain origin/version/license/integrity. Immutable evidence retains its
recorded commit/environment/command/result identity. Authored bytes are not
reclassified as generated to evade repository bounds. Any changed authority
makes dependent graph nodes stale until a complete rebuild.

## Acceptance Gates

V1 becomes Current only after deterministic rebuild, malformed-version,
provenance-conflict, stale-input, aggregate-boundary, traversal-cycle,
truncation, and authority-spoofing tests pass. Repository-local links and exact
compiler facts must agree with their authorities. Two clean builds from the same
inputs must be byte-identical, and generated files must remain under `target/`.

## Deferred And Rejected

Embeddings, remote graph services, cross-repository federation, probabilistic
ranking, and autonomous context mutation are **Deferred**. Treating graph data
as a type/proof/evidence authority, omitting provenance, unlimited traversal,
silent truncation, uploaded private context, and tracked generated graph output
are **Rejected**.
