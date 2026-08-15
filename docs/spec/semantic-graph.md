# Semantic Program Graph specification

## Authority and closed schema

The Semantic Program Graph (SPG) is the only mutable program authority, and `lkjscriptd` is its
only live writer. A published `Snapshot` is immutable and revision-labelled. Canonical semantic
state is workspace identity, stable node identity, ownership and ordered child slots, typed
operation data, direct value references, allocator state, tombstones, revision, and selected
package entry. Names are presentation and lookup metadata stored in the snapshot; they are not
identity. Derived blockers, query facts, diffs, Core IR, diagnostics, and timings are not mutable
graph authority.

`src/schema.rs` owns the closed node and operation vocabulary and its stable boundary tags. Unknown
kinds, operation codes, attributes, slots, value forms, and tags reject. The implemented shape is:

```text
WorkspaceRoot -> Package -> Module -> Function -> Region -> Block -> Operation
                                      \-> Parameter             \-> BlockArgument
Operation -> ordered owned Region slots -> Block -> BlockArgument / Operation
```

A function may have no body while incomplete. Every attached region has exactly one ordered block
in the current schema. A function-body block ends in `return`; an `if` arm or `for_i64` body ends in
`yield`. Blocks own ordered arguments where their derived region role requires them: a loop body has
`loop_index` and `loop_carried`, while function and conditional blocks have none. A value is a
function parameter Node ID, block-argument Node ID, or `(operation Node ID, checked output index)`.
Containment, direct definition references, and value uses remain distinct. Every non-root live node has one owner and is reachable from the
root. Observable graph order is explicit or sorted.

## Identity and history

A `WorkspaceId` is 128 bits. A `NodeId` is `(WorkspaceId, nonzero u64 serial)`. Root serial 1 is
created at revision zero; allocation is monotonic. Names, positions, hashes, compiler indexes, and
addresses never determine identity. A `LocalHandle` is a u32 transaction-local symbol and never
enters semantic state.

Allocation is staged. A rejected or validate-only request changes no published allocator state.
Deletion tombstones every owned ID, retained snapshots preserve deleted nodes, and later nodes
never reuse tombstones. Every serial below the allocator frontier is live or tombstoned. Adjacent
history requires stable root identity, monotonic allocation and tombstones, no resurrection, stable
kind/owner/child continuity for surviving nodes, unchanged relative order for surviving body
children, and no clearing of a selected entry from a surviving package. `SetEntryFunction` may
select or replace an entry. Rename and a compatible same-constructor scalar/operand update preserve
identity.

`RefineHole` is the only identity-preserving constructor transition. It changes exactly:

```text
Hole(expected type) -> complete non-terminator operation with the same single result type
```

It preserves the hole Node ID, owner, body position, and all existing uses of output zero. The
replacement may use existing values or transaction-local values created in the same batch; the
final structural order, scope, dominance, type, ownership, and result-index validation still
applies.
Refinement to a hole, a terminator, a different result contract, or from an already-complete
operation rejects. There is no reverse refinement or generic morph operation. History validation
recognizes only this explicit transition, and the semantic diff reports `OperationRefined` rather
than delete/create identity churn.

## Typed transactions and compact receipts

An `ApplyTransactionRequest` names workspace, exact base revision,
`TransactionMode::{Commit, ValidateOnly}`, an optional committed-request idempotency key, an ordered
closed batch of `TransactionOp` values, and a bounded `TransactionResponseSpec`. Public creation is
`CreatePackage`, `CreateModule`, structured `CreateFunction`, `DefineFunctionBody`, and
`InsertExpression`; maintenance sets the entry, renames, replaces a compatible operation or operand,
refines a regionless hole, or deletes an owned subtree. `InsertExpression` names a block that exists
in the base snapshot, allowing deeper structured programs to be assembled across bounded
transactions without exposing a parallel low-level scaffolding API.

A structured function or expression draft is a typed proposal only. Parameters, expressions,
holes, loop indexes, and loop-carried values declare explicit local handles. Regions, blocks,
block arguments, and return/yield terminators are structurally implied canonical nodes. Expansion
uses an explicit work stack, scans all explicit handles first, rejects zero/duplicate handles and
request depth/item overflow, and counts every call argument whether it appears in a structured
expression or fine-grained operation replacement/refinement under the same 65,536-item request
policy. It assigns collision-free private handles for implied nodes, then allocates all explicit and
implicit Node IDs in depth-first canonical node order before applying any edit.
Calls may therefore name later function handles and mutual references. Final graph validation is
unchanged and authoritative; the draft and private handles are discarded.

A client may select at most 64 explicit created handles for the receipt. Private implied handles
cannot be selected or returned. Duplicate, undeclared, private, or non-created selected handles
reject before publication. A node created and deleted in the same accepted transaction
still contributes to `created_count`, may be selected in the receipt, and ends tombstoned in the
new snapshot. Deletion is iterative and rejects when a surviving reference points into the
subtree; it never cascades through independent dependents.

Preparation clones the current snapshot, stages identities and edits, validates the final graph,
derives the exact deterministic semantic change list, encodes the candidate artifact and compact
receipt, and preflights exact protocol and durable bytes. Rejection publishes no revision, artifact,
HEAD, allocator movement, tombstone, or in-memory snapshot. Empty and semantic no-change requests
reject. A successful commit durably publishes exactly one revision before publishing it in memory.

The receipt is deliberately bounded: workspace, base and resulting/predicted revision, snapshot
hash, publication flag, total created count, only selected handle bindings, exact total change
count and `ChangeDigest`, and before/after completeness facts. The digest is domain-separated and
binds the workspace, revisions, both canonical snapshot hashes, ordered change count, and exact
change payloads. The full change list is available only through a revision-bound paginated diff
query.

Validate-only uses the same staging, validation, artifact encoding, response encoding, and durable
publication preflight as commit. It returns the same predicted identity, hash, count, digest, and
selected bindings with `published=false`, consumes no identities, and writes nothing. An
idempotency key on validate-only rejects.

A committed idempotency key is bound to the deterministic transaction fingerprint, including the
response projection, and one compact receipt. Exact retry returns that receipt without reapplying
the mutation; different reuse rejects. Only one keyed outcome is retained per workspace. A later
keyed commit replaces it; an unkeyed commit retains the existing keyed record.

## Incompleteness and queries

A missing function body and an exact typed `Hole` are valid incomplete semantic states. Blockers,
incoming uses, body slices, visible values, legal constructors, and repair contexts are derived by
full scans of an immutable revision. For a nested structured hole, repair context exposes enclosing
region roles and visible block-argument identities, ordinals, roles, and types; a repair therefore
uses persistent graph identities rather than reconstructing private draft handles. Entry lowering
requires a complete selected-entry dependency closure; holes never lower. Unused incomplete
definitions do not block an otherwise complete entry.

## Artifact and durable HEAD

A `.lkjscript` artifact uses format version 2 and semantic schema identity `lkjscript-spg002`;
older artifact bytes reject without a compatibility reader. It has fixed magic and semantic schema ID, little-endian integers, checked u64
counts, canonical node/tombstone order, allocator/root state, and a BLAKE3 snapshot hash. Decode
rejects truncation, overflow, invalid UTF-8, unknown tags, duplicate or wrong-workspace IDs, invalid
containment or references, hash mismatch, and trailing bytes. Count work is bounded from remaining
artifact bytes and exact minimum record widths; there is no separate semantic node or tombstone
ceiling. Durable workspace IDs and revision file names must use their one canonical path spelling.
Accepted decode followed by encode is byte-identical. Core IR, machine code, caches, profiles,
receipts, and protocol frames are absent.

`LKJHEAD3` directly replaces the old HEAD format; there is no compatibility reader. It is a checked,
independently bounded (16 KiB) non-semantic publication record containing head revision/hash and,
when present, one compact keyed fingerprint/receipt. It never contains a full diff or allocation
map. Restart decodes every retained artifact, validates adjacent history, and recomputes/validates
receipt facts against retained snapshots before accepting HEAD. Corrupt or old HEAD bytes reject.
