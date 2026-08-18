# Typed semantic program model

This specification owns the accepted program representation, identity domains, revisions,
transactions, continuity, and durable workspace artifacts.

## Authority

One immutable `Snapshot` is the authoritative meaning of one accepted workspace revision. It
contains the workspace identity, revision, root, durable allocator frontier, durable tombstones,
closed typed semantic items, explicit order, references, package entry selection, and canonical
hash. Names are lookup and presentation metadata, not identity.

JSON, editable documents, context packets, reviews, diffs, caches, Core IR, ownership plans, runtime
handles, and storage paths are not authority. Every proposal normalizes into the same typed
transaction and passes the same validator before publication.

The physical bootstrap representation remains a `BTreeMap<NodeId, Node>`. “Node” describes that
representation; it does not grant every item durable continuity and does not require a graph
database.

## Closed semantic vocabulary

The current ownership hierarchy is:

```text
workspace root
  -> package
    -> module
      -> product declaration -> product field
      -> sum declaration -> sum variant
      -> function -> parameter / function body

function body
  -> region -> block -> block argument / operation / terminator
structured operation
  -> region -> block -> block argument / operation / terminator
```

`src/schema.rs` owns the closed kinds, operation forms, types, fields, and stable tags. Unknown
kinds, fields, tags, references, result indexes, owners, or control forms reject. Every non-root item
has one exact owner and is reachable from the root. Observable lists have explicit canonical order.

## Identity domains

### Workspace identity

A `WorkspaceId` is 128 random bits rendered as 32 lowercase hexadecimal characters. It names one
authority namespace, not content.

### Durable entity identity

A durable `NodeId` is rendered as `WORKSPACE:SERIAL`, where `SERIAL` is a nonzero canonical decimal
integer below `2^63`. The root uses serial 1. Accepted creation advances one workspace counter.
Rejected and validate-only proposals do not advance it.

Durable identity is assigned to:

- the workspace root;
- packages and modules in the current model;
- product and sum declarations;
- product fields and sum variants;
- functions and function parameters;
- explicit `Hole` operations, because repair targets them across revisions.

These classes have continuity consumers in lookup, calls, nominal types, public values, entry
selection, repair, history, and review. A deleted durable serial is tombstoned and cannot be reused.

### Function-local identity

Regions, blocks, block arguments, ordinary operations, and implied terminators use a
revision-bound function-local `NodeId` rendered as `WORKSPACE:lFUNCTION.ORDINAL`. Its encoded high
bit separates it from the durable domain; `FUNCTION` names the owning durable function serial and
`ORDINAL` is a nonzero body-local canonical ordinal.

Local references express order, sharing, dominance, binders, control, and diagnostics inside one
exact body. They:

- cannot cross workspace or owning-function domains;
- cannot be used as nominal declarations, call targets, or external continuity anchors;
- do not advance the durable allocator;
- do not enter durable tombstones;
- may reuse the same spelling in another revision because revision/body binding supplies the
  domain.

A compiler diagnostic identifies the exact revision, durable function, and local origin. Private
Core IR IDs, runtime handles, draft symbols, context aliases, and artifact offsets remain separate
domains.

### Proposal and derived identities

`DraftSymbol` is a transaction-local label. A context alias is packet-local and digest-bound. A
snapshot hash identifies canonical immutable snapshot bytes. None is a semantic entity ID.

### Reusable-release identity

A reusable release is a different immutable authority domain. Workspace construction projects one
exact package closure into canonical release-local IDs and erases `WorkspaceId`, `Revision`,
workspace durable serials, function-local numbering, allocator frontier, and tombstones. A
`ReleaseItemId` has meaning only with its exact `ReleaseId`.

`ReleaseId` is deliberately the domain-separated digest of the complete validated canonical
release payload. In this closed immutable domain, full digest equality is exact release equality;
the collision and second-preimage assumptions and conflicting-byte rejection are part of the
contract. `ReleaseContentDigest` uses a second domain and is integrity evidence only. Coordinate,
user version, dependency slot, export name, local alias, file path, compiler ID, and runtime handle
are never substitutes for exact release/item identity.

Nominal equality across release boundaries is equality of `(ReleaseId, ReleaseItemId)`. Therefore
two structurally identical declarations in distinct releases do not unify, while two paths through
a diamond to the same exact release do. Local import proxies preserve the foreign pair; they do not
turn a dependency nominal into a workspace-local or consumer-release nominal. Compiler flattening
derives private dense IDs from these pairs without changing public equality.

The complete release and composition contract is owned by
[reusable-release.md](reusable-release.md).

## Function bodies and anchors

A function body is an immutable typed structure represented by function-local items. Terms use
exact local references for sharing; structured regions state control and binders explicitly. The
validator checks scope, order, dominance, result index, types, owner chains, branch contracts, and
terminators without parser syntax.

`ReplaceFunctionBody` preserves the durable function entity, rejects if it would implicitly remove
a durable hole anchor, deletes only the previous local body items, and canonically rebuilds local
ordinals. It creates no durable identity or tombstone. A no-op canonical replacement rejects under
the normal no-change rule.

A `Hole(expected_type)` is the sole durable body anchor currently implemented. `RefineHole`
preserves its durable ID, owner, position, output zero, and incoming uses while replacing it with one
complete regionless operation of the same result type. Wrong type, terminator, second refinement,
foreign reference, or incompatible nominal operation rejects.

## Declaration and function replacement

Changing a declaration's shape or a function's signature creates a new durable entity. The caller
must create the replacement, rewrite all exact nominal/call/member uses, update package entry when
needed, and delete the old entity. Deletion preflight rejects while any surviving use remains.

The active contract does not infer continuity from names, order, or structural similarity and does
not persist a member-continuity map. Consequently replaced declarations and members appear as
deleted and created entities. A future continuity-map operation requires a consumer beyond review,
exact owner/kind/type rules, injectivity, persisted history semantics, and two independently useful
migrations.

Two public-path controls exercise replacement today: the `Limits` to `DeploymentLimits` migration
with coordinated behavior changes, and an independently shaped variant replacement with rename,
reorder, and an added alternative. The latter document is 1,085 bytes. Their differing rewrite
needs did not justify a generic mapped-migration endpoint.

## Transactions

An `ApplyTransactionRequest` contains:

- exact workspace and base revision;
- `commit` or `validate_only` mode;
- an optional commit-only idempotency key;
- one ordered closed list of edits;
- a bounded returned-symbol projection.

High-level creation and body edits normalize to private canonical edits. Normalization is iterative,
resolves declarations and calls against the complete proposal, assigns durable IDs only after
boundary checks, and assigns body-local ordinals within each function domain. Draft spelling and
insignificant document formatting cannot change allocation.

Preparation clones the base snapshot, applies canonical edits, validates the complete candidate,
derives the semantic diff, encodes the artifact and exact response, and preflights publication.
Rejection changes no revision, identity frontier, tombstone, file, HEAD, or in-memory state.
Validate-only performs the same relevant semantic, artifact, and response work but publishes
nothing. Commit publishes exactly one revision.

The receipt includes exact base/result revision, canonical snapshot hash, publication flag, durable
created count, selected bindings, semantic change count/digest, and completeness facts. Local body
churn is summarized as `FunctionBodyChanged`; durable entity changes remain individually visible.

Idempotency binds canonical protocol-v10 transaction bytes, including the requested receipt
projection. Exact replay returns the retained receipt. Reuse with different bytes rejects.

## History

Revisions are contiguous from zero and immutable. Adjacent history requires:

- stable workspace and root;
- monotonic durable allocator and tombstones;
- no durable identity resurrection;
- stable kind, owner, and immutable member contract for surviving durable entities;
- valid function-local domains in each independent revision;
- no cleared entry on a surviving package;
- only the explicit hole-refinement constructor transition.

Anonymous body structure has no cross-revision continuity. Structural diff may report body item
counts and exact revision-local modifications, but similarity never creates semantic identity.

## Durable workspace format

Each revision is one full canonical artifact:

- magic `LKJTSM\0\x06`;
- artifact version 6;
- semantic schema `lkjscript-tsm006`;
- fixed little-endian checked fields and lengths;
- canonical item order;
- domain-separated BLAKE3 snapshot hash;
- strict trailing-byte rejection.

`LKJHEAD8` binds the current workspace, head revision/hash, and retained idempotency receipt under a
domain-separated BLAKE3 checksum. Data is written and synchronized before atomic HEAD replacement.
Failure injection covers every publication step; ambiguous durable outcome stops the engine with
`commit_outcome_unknown`.

Restart decodes every contiguous retained snapshot and validates every adjacent transition. Missing,
truncated, oversized, noncanonical, wrong-schema, hash-mismatched, foreign-workspace, or inconsistent
history rejects. Format 5 / `LKJSPG\0\x05`, `lkjscript-spg005`, and `LKJHEAD7` reject directly.

Integrity checks rely on collision and second-preimage resistance of full 256-bit BLAKE3 outputs.
Every stored artifact payload and HEAD body is rehashed on load. A digest collision or an existing
digest paired with different bytes is corruption; a digest is neither authorization, a signature,
nor semantic entity identity.

Full snapshots are retained because the identity-pressure corpus produces constant 443-byte
artifacts through 32 body replacements, and the eight-revision application remains 8,354–9,457
bytes per revision. Reconsider delta/checkpoint or immutable-object storage when restart or retained
bytes become material on a representative larger corpus. Such a cutover must define object
validation, root retention, interruption-safe garbage collection, and one reconstruction oracle.

## Workspace, release, application, and executable domains

The `.lkjscript` file is a workspace revision artifact. It includes development history contracts
and is not a reusable dependency or distribution format. Release artifact version 1 is a canonical
workspace-independent semantic unit with explicit exports, exact dependencies, and release-local
identity. Application artifact version 2 embeds one exact validated release graph with entry,
invocation contract, policy, and application cases. It contains no workspace identity. Core IR,
ownership plans, runtime tags, and values remain derived executable state.

Workspace, reusable release, application, bundle/container, and derived executable domains remain
distinct. A digest acquires the narrow identity role stated by its owning specification and no
provenance, authorization, signature, workspace continuity, or runtime meaning by implication.
