# Product release and contract version authority

Date: 2026-08-29 UTC.

## Status

Accepted and implemented for the static 0.1.8 Linux distribution cutover.
Amended by the owner's 2026-10-01 decision: all three product-version components
are opaque identifiers, with no Semantic Versioning roles. The single-owner
and independently owned contract decisions below remain in force.

## Decision

The version of the root `lkjscript` Cargo package is the human-facing product release snapshot. It
owns the matching annotated `vA.B.C` tag. All components have the same identifier-only
role; no component denotes compatibility, change size or stability. That version identifies one distributed
product selection; it is not a language edition or a compatibility version for the meaning graph,
CLI, executable registry, project, artifact, deployment, runtime, standard package, repository, or
contributor tooling.

Each public or stored contract advances at its existing executable owner only when that contract's
representation or behavior changes. Registry content may change its digest without changing the
registry encoding contract. Semantic and package revisions, artifact identities, target triples,
asset and file digests, and commit SHAs remain exact identities in separate domains. The unpublished
`lkjscript-dev` crate does not inherit the root product version merely because it shares the Cargo
workspace.

The operational release manifest binds the product version, exact source, target policy, candidate
bytes, executable CLI contract, registry digest, and package facts for one release. It does not
select accepted program meaning or become another contract registry. Normative compatibility stays
with each contract owner; current public release facts stay in the README and status documentation
after publication is independently proved.

No duplicate `VERSION` file, workspace-wide version table, synchronized contract ladder, Graph
product-version field, or handwritten current contract catalog is introduced. Release tooling reads
the root package version and executable-owned contract identities directly.

## Consequences and reversal

A product release may change any component while leaving every language and storage contract
unchanged, or change only its last component while breaking a contract. An independently
owned contract may advance without forcing unrelated package versions to match. Release validation
rejects a tag that differs from the root package version and rejects a candidate whose executable
contract identities differ from its manifest.

Reverse this decision only if the repository adopts a different single product-release owner and
migrates the tag, release tooling, public documentation, and every maintained consumer in one
dependency-closed cutover. A new language edition or package ecosystem, by itself, is not a reason
to synchronize these identities.
