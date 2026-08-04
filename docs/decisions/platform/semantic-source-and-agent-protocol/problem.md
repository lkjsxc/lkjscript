# Semantic Source And Agent Protocol: Problem

[Authority](../semantic-source-and-agent-protocol.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Problem

The Current compiler resolves a physical token/form tree directly into typed
HIR. Agents retrieve arbitrary file fragments and edit by string identity.
Formatting drift, repeated forms, stale revisions, and broad diagnostics can
therefore turn a semantically simple edit into an ambiguous text mutation. A
new language edition built on that interface would magnify context cost and
migration risk.

The source schema must not become a second trusted type system or a sibling
backend frontend. It must preserve exact source origin while keeping compiler-
derived type, ownership, effect, layout, and proof facts authoritative.
## Historical Versioned Identities

The removed versioned identities were `lkjscript.semantic-source`,
`lkjscript.agent`, `lkjscript.diagnostic`, and `lkjscript.edit`. The removed
`lkjscript.semantic-source-foundation` identity contained the opaque validated
canonical source tree, deterministic dependency-first loading, exact source-byte
revision identity, stable declaration keys, formatting, and structural source
diagnostics. These identities are Historical and rejected; they grant no Current
interface, alias, or fallback. Current envelopes use stable schema names plus
exact full contract digests.
## Source Authority Boundary

A parser adapter accepts canonical `.lkjscript` source and constructs a
private mutable builder. Public in-memory validation and compilation require a
canonical relative non-dot UTF-8 logical path ending in `.lkjscript`; absolute,
`./`, parent, doubled-separator, and legacy-extension spellings are rejected.
Host loading rejects non-UTF-8 logical paths rather than applying replacement
characters. On the Current Linux acceptance target, a source is opened as a
stable regular-file descriptor, its actual path is resolved through
`/proc/self/fd` and recontained under the canonical package or installed root,
and only then is a per-file/aggregate bounded read performed. A one-byte
sentinel detects growth past the remaining allowance; non-regular files and
metadata/read size changes fail before parser copying. Validation checks marker
matching, source limits, declaration shape, stable-key uniqueness, source
spans, and node-tree well-formedness. Only successful validation yields opaque
immutable `ValidatedSourceTree` authority. Non-Linux descriptor containment is
not an accepted boundary and fails closed rather than weakening this contract.

Consumers cannot construct a validated tree by deserializing an arbitrary
public struct. HIR analysis accepts the validated boundary or a mechanically
checked projection from it; no backend reads source spelling or serialized
claims.

The first cutover must replace the old parser/form authority rather than retain
two independently interpreted source trees. A temporary mechanically checked
adapter may feed unchanged analysis during the cutover. It is removed once HIR
consumes the validated source nodes directly.
## Schema

Schema represents the complete vocabulary of Current canonical source:

- source unit, edition, schema version, canonical relative origin, and imports;
- top-level main, function, product, trait, and implementation declarations;
- declaration visibility as an explicit closed value, even where canonical source
  permits only its default;
- names, type forms, parameters, generic bounds, product fields, trait markers,
  and implementation targets;
- literals, names, bindings, calls, operations, conditionals, loops, local
  mutation, product operations, ownership operations, and every other Current
  expression form;
- comments/documentation if and when the Current projection exposes them; until
  then their absence is explicit rather than reconstructed heuristically;
- exact byte/line/column source spans and migration/source origin; and
- expression holes in the supported development positions.

Closed enums represent node kinds. Generic untyped JSON objects are not
semantic nodes. Source nodes never serialize inferred types, resolved binding
IDs, effects, ownership results, layouts, or optimizer facts as authority.
Queries may attach those derived facts in separately versioned response fields.
## Two Identity Layers

### Stable declaration keys

A stable declaration key is derived deterministically from:

```text
schema version
+ edition
+ exact package identity
+ canonical relative source-unit identity
+ declaration kind
+ declared name or reserved main identity
```

Each component is encoded as a length-framed byte field in schema, version,
edition, package, canonical logical-path, declaration-kind, and declared-name
order. Keys do not depend on byte offsets, declaration order, formatting, or
dense compiler IDs. Exact duplicate detection compares the complete framed key
bytes, not only its digest. Human projections escape field delimiters and are
not key authority. Function, product, and trait names must be spellable source
identifiers before key construction. Rename and move operations report old and
new keys plus the semantic relationship. Duplicate keys are rejected rather
than disambiguated by source order. Package and module identities are Current components of declaration keys.

### Revision-scoped node IDs

Every validated revision assigns dense preorder `NodeId` values. Foundation
revision framing includes each source unit's canonical logical path and exact
input byte length plus SHA-256, so distinct accepted spellings and line endings
cannot share a revision merely because canonical formatting is equal. A node ID
is valid only with its exact repository/tree revision. It is compact compiler
and protocol data, not a cross-revision semantic identity. Transactions that
refer to a node ID from another revision fail as stale.
## Revisions And Preconditions

A snapshot carries:

- a server-issued monotonic revision counter within a daemon session;
- a deterministic whole-tree fingerprint over the canonical schema encoding;
- the base repository identity when available; and
- deterministic per-entity and per-node precondition fingerprints.

A transaction names the exact base revision and expected fingerprint for every
read/modified entity. Fingerprints are a stale-edit check, not authorization.
Commit also compares the exact canonical precondition value, so a hash
collision cannot authorize a different edit. A one-shot CLI snapshot derives
its initial identity from the complete loaded source closure rather than wall
clock, process ID, or filesystem enumeration order.
## Atomic Semantic Edits

Protocol supports these complete operations for schema nodes that exist in
the first implementation:

- insert a top-level declaration at an explicit semantic position;
- replace a top-level declaration;
- delete a top-level declaration;
- rename a declaration and all resolved references in the loaded closure;
- replace an expression subtree;
- insert or delete an expression child only where the parent schema defines an
  ordered child collection; and
- fill or refine a typed expression hole.

Each transaction follows:

1. validate envelope/version, revision, limits, and all preconditions;
2. clone or persistently stage the complete affected semantic state;
3. resolve every target before applying any operation;
4. apply operations in declared order to the staged state;
5. rebuild dense node IDs and stable keys deterministically;
6. run structural validation and optional name/type/effect/ownership checks;
7. compute the semantic diff and diagnostics; and
8. atomically publish one new revision only on success.

Any failure discards all staged changes. Operations never search for an exact
text substring and never partially write source files. File publication uses a
same-directory temporary plus atomic replacement where the host guarantees it;
otherwise the protocol reports unsupported publication before changing files.
