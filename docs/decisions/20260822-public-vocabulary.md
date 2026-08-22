# Public vocabulary and CLI v4 grammar

Date: 2026-08-22 UTC.

## Status

Accepted and implemented for the direct CLI v4 command surface. Remaining high-level refactor
forms are future work; this record does not claim that every desired authoring operation exists.

## Evidence and counts

The executable registry in `src/platform/cli.rs` is the exhaustive command owner. The inspected
graph-4 CLI v4 executable reported schema digest
`1980273fe10405fbf7aa7940c607af819c1261bd8b89019243326da31841df6c` and:

- 17 direct command groups: `capabilities`, `new`, `inspect`, `query`, `change`, `draft`,
  `history`, `package`, `check`, `build`, `run`, `serve`, `worker`, `review`, `backup`,
  `restore`, and `doctor`;
- 13 implemented high-level change forms, 16 advertised type forms, 24 owner kinds, and 14
  relation roles.

The predecessor CLI v2 registry exposed 40 flat entries below a mandatory `semantic` prefix.
Thus the registry-level command count changed from 40 flat entries to 17 compositional groups.
This count does not pretend that nested actions disappeared; it measures the discoverable command
vocabulary owned by each registry.

The selected glossary below has 15 core entries. It deliberately does not count identity-domain
qualifiers, language forms, or runner kinds as synonyms for those entries.

Black-box tests require `semantic`, `help`, `id-allocate`, `import`, `text-project`, `export-text`,
`export-bundle`, `hash`, and `deployment` to reject with `cli_usage`. There is no compatibility
routing.

## Selected vocabulary

| Term | Exact public meaning |
|---|---|
| meaning graph | The canonical typed program meaning committed by one accepted revision. |
| project | The filesystem container from which one repository is selected. It is a locator, not identity. |
| repository | The durable identity, accepted history, and publication domain for one root package. |
| package | A graph-owned unit containing modules, exact dependencies, targets, and a public surface. |
| module | A named graph namespace containing declarations and members. |
| declaration / member | Language constructs; use `owner` only as the query umbrella for selectable stable identities. |
| revision | One immutable accepted history node bound to an exact root and parents. |
| change | One public atomic semantic edit request. |
| transaction | The exact internal publication protocol and digest to which a change lowers. |
| draft | Explicit non-executable pending work bound to one repository and base revision. |
| artifact | Derived executable or package-closure bytes; never editable program authority. |
| review | A deterministic non-authoritative projection. |
| receipt | Compact durable evidence for an operation, with exact expansion. |
| deployment | External operational grants, resources, secrets, and runner policy. |
| capabilities | Offline discovery of the binary, grammar, schema digest, forms, and budgets. |

Stable identity, mutable name, namespace, revision, digest, filesystem path, request-local symbol,
compiler index, and runtime handle remain different domains. A name is not called an identity, and
a digest is not called provenance or authority.

## Grammar convergence

- Discovery is `capabilities`; schema-cache hits use `--known-schema`.
- Project orientation, exact owner and revision detail, target, artifact, and deployment detail are
  actions of `inspect`.
- Owner selection and relation traversal are actions of `query`; `relations`, `types`, and
  `capabilities` replace storage- or spelling-specific relation names.
- Dry-run and accepted publication are modes of one `change` lowering path. Request-local symbols
  replace separate stable-ID preallocation.
- Draft lifecycle actions are under `draft`; list/show/diff/merge are under `history`.
- Dependency staging and built-in inspection/export are under `package`.
- Graph-owned tests use `check`; deterministic projection uses `review`; recovery uses `backup`
  and `restore`.
- The two duplicate behavior pairs were removed: review projection has one name (`review`), and
  canonical recovery export has one name (`backup`).
- Fresh project creation is `new`; it is distinct from exact restore and dependency staging.

The success envelope uses five fields: `contract_version`, `ok`, `status`, `command`, and `result`.
A classified failure keeps the first three and uses `error`. Command-specific fields stay inside
the result rather than becoming synonyms in a second envelope.

## Boundaries and remaining work

Protocol identities and internal Rust symbols may retain `semantic` as a domain qualifier; they do
not authorize a public command namespace. CLI v4 currently emits the machine value
`typed_semantic_graph` in workspace orientation while public prose selects “meaning graph.” That
residual value is implemented reality, not a second approved term; changing it requires a direct
schema cutover and updated black-box fixtures.

Delete, move, rebind, signature, field/case, extract, inline, and typed conflict-resolution forms
remain unimplemented. When added, they must be actions or change forms within this grammar, not new
top-level synonyms. A complete nested expression schema is also not yet exposed by discovery.

## Reversal and compatibility policy

Add a top-level command only when it represents a distinct authority or execution domain and
cannot compose cleanly under an existing group. Rename or regroup only by direct schema cutover
with all maintained consumers migrated. Do not restore removed spellings, aliases, or the universal
namespace. Historical evidence may quote old commands but must label them historical.
