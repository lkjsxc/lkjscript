# Semantic Program Graph specification

## Authority and closed schema

The Semantic Program Graph (SPG) is the only mutable program authority, and `lkjscriptd` is its
only live writer. The SPG specifies typed semantic entities, containment, ordering, and references;
it does not require a generic property graph, pointer graph, or graph database as the physical
storage layout. A published `Snapshot` is immutable and revision-labelled. Authoritative semantic
state is workspace identity, stable node identity, ownership and ordered child slots, typed
operation data, direct value references, allocator state, tombstones, revision, and selected
package entry. Names are presentation and lookup metadata stored in the snapshot; they are not
identity. Every package, module, product type, sum type, function, product field, sum variant, and
function parameter name is valid UTF-8 and contains at least one UTF-8 byte. Names are unique within
the exact sibling groups `workspace.packages`, `package.modules`, `module.types` (product and sum
declarations together), `module.functions`, `product.fields`, `sum.variants`, and
`function.parameters`; names in different groups may coincide. Derived blockers, query facts, diffs,
Core IR, managed-reference maps, ownership plans, diagnostics, and timings are not mutable
graph authority.

`src/schema.rs` owns the closed node and operation vocabulary and its stable boundary tags. Unknown
kinds, operation codes, attributes, slots, value forms, and tags reject. The implemented shape is:

```text
WorkspaceRoot -> Package -> Module -> ProductType -> ProductField
                              \-> SumType -> SumVariant
                              \-> Function -> Parameter / Region -> Block -> Operation
Operation -> ordered owned Region slots -> Block -> BlockArgument / Operation
```

A function may have no body while incomplete. Every attached region has exactly one ordered block
in the current schema. A function-body block ends in `return`; an `if` arm or `for_i64` body ends in
`yield`. Blocks own ordered arguments where their derived region role requires them: a loop body has
`loop_index` and `loop_carried`; a payload-bearing match arm has one `match_payload` argument; and
function, conditional, and nullary match-arm blocks have none. A value is a
function parameter Node ID, block-argument Node ID, or `(operation Node ID, checked output index)`.
Containment, direct definition references, and value uses remain distinct. Every non-root live node has one owner and is reachable from the
root. Observable graph order is explicit or sorted.

## Identity and history

A `WorkspaceId` is 128 bits. A `NodeId` is `(WorkspaceId, nonzero u64 serial)`. Root serial 1 is
created at revision zero; allocation is monotonic. Names, positions, hashes, compiler indexes, and
addresses never determine identity. A `DraftSymbol` is a transaction-local proposal label matching
`[a-z][a-z0-9_]*` in 1 through 64 UTF-8 bytes. It never enters semantic state or determines persistent allocation order.

Allocation is staged. A rejected or validate-only request changes no published allocator state.
Persistent identity non-reuse is the semantic contract: a deleted serial can never identify a later
entity. Artifact format 5 currently proves this by physically retaining deletion tombstones and all
saved snapshots; that representation and full-retention strategy are not themselves a mandate for
future physical storage. Under the active format, every serial below the allocator frontier is live
or tombstoned. Adjacent history requires stable root identity, monotonic allocation and tombstones,
no resurrection, stable
kind/owner/child continuity for surviving nodes, unchanged relative order for surviving body
children, and no clearing of a selected entry from a surviving package. `SetEntryFunction` may
select or replace an entry. Rename and a compatible same-constructor scalar/operand update preserve identity. A surviving
product or sum declaration retains its exact ordered member IDs. A surviving field retains owner,
ordinal, and type; a surviving variant retains owner, ordinal, and payload contract. Public
transactions do not append, remove, reorder, or retype members under surviving declaration identity.

`RefineHole` is the only identity-preserving constructor transition. It changes exactly:

```text
Hole(expected type) -> complete non-terminator operation with the same single result type
```

It preserves the hole Node ID, owner, body position, and all existing uses of output zero. The
replacement may use existing values or transaction-local values created in the same batch; the
final structural order, scope, dominance, type, ownership, and result-index validation still
applies.
Refinement to a hole, a terminator, a different result contract, or from an already-complete
operation rejects. A nominal hole may refine only to exact regionless product construction, variant
construction, or projection; match is ineligible. There is no reverse refinement or generic morph
operation. History validation
recognizes only this explicit transition, and the semantic diff reports `OperationRefined` rather
than delete/create identity churn.

## Typed transactions and compact receipts

An `ApplyTransactionRequest` names workspace, exact base revision,
`TransactionMode::{Commit, ValidateOnly}`, an optional committed-request idempotency key, an ordered
closed batch of `TransactionOp` values, and a bounded `TransactionResponseSpec`. Public creation is
`CreatePackage`, `CreateModule`, atomic `CreateProductType`, atomic `CreateSumType`, structured
`CreateFunction` with an optional body, and `InsertExpression`; maintenance may
`DefineFunctionBody` only for an existing function Node ID, set the entry, rename, replace a
compatible operation or operand, refine a regionless hole, or delete an owned subtree.
`InsertExpression` names a block that exists
in the base snapshot, allowing deeper structured programs to be assembled across bounded
transactions without exposing a parallel low-level scaffolding API.

A structured function or expression draft is a typed proposal only. Parameters, optionally bound
expressions, loop indexes, loop-carried values, and match payloads declare explicit draft symbols.
Regions, blocks, block arguments, omitted expression bindings, and return/yield terminators are
private implied nodes. Shared or forward-referenced expression results require an explicit symbol.

A structured value position may contain an anonymous inline expression. The retained inline set is
derived from the same operation descriptors as graph validation: `const_unit`, `const_bool`,
`const_i64`, `const_bytes`, `add_i64`, `lt_i64`, `call`, `construct_product`, `project_field`,
`construct_variant`, `bytes_len`, `bytes_at`, `bytes_slice`, `bytes_equal`, and `bytes_concat` are complete,
non-terminating, single-result operations with no owned region and may be inline. `hole`, `if`,
`for_i64`, and `match_sum` remain explicit. An inline expression has one
use by construction, has no public draft symbol, cannot be selected in the receipt, and normalizes
to an ordinary persistent operation with an ordinary stable Node ID. Shared values, repairable
placeholders, receipt selections, and maintenance targets remain explicitly bound.

Normalization uses one explicit worklist. It follows transaction operations and structured body
operations in request order, normalizes each inline operand child depth-first from left to right
before its parent, normalizes product fields and match arms into declaration order before allocating
their contained semantics, and keeps owned-region bodies after their owning operation. Function
return and structured-region yield values follow the same inline-child rule before the implied
terminator. The complete nested proposal is discarded. No proposal path, nesting, or anonymous name
is retained in semantic state.

`ConstBytes` stores exact immutable literal octets in the operation node. One literal is limited to
4,096 octets and the checked aggregate across explicit and inline literals in one transaction is
65,536 octets. These limits apply before persistent identity allocation. Public base64 is only a
proposal/transport spelling; it is not retained as semantic state.

An explicit operation sequence in that canonical postorder and its inline spelling allocate the
same persistent Node IDs and produce the same authoritative snapshot, artifact bytes, semantic
change list, and execution behavior. Draft-symbol spelling remains irrelevant. Canonical request
bytes, rather than normalized graph meaning, remain the idempotency fingerprint input, so differently
spelled explicit and inline requests do not become semantic deduplication aliases.

Expansion validates every public symbol and reference before allocation and normalizes labels to
private keys. Persistent IDs are then
allocated in the flattened canonical edit order, independent of symbol spelling. Duplicate, empty,
invalid, overlength, unknown, or wrong-category symbols reject with the exact `draft_symbol`; a
bounded deterministic `draft_path` identifies failures on inline or private implied nodes. Product bindings
and match arms normalize into declaration order. Calls may name later functions or form mutual
references, and nominal types may name later declarations. Final graph validation remains unchanged
and authoritative; proposal labels and private keys are discarded.

A client may select at most 64 explicit created symbols for the receipt. Private implied symbols
cannot be selected or returned. Duplicate, undeclared, private, or non-created selected symbols
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
hash, publication flag, total created count, only selected symbolic bindings, exact total change
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
uses persistent graph identities rather than reconstructing private draft symbols. Entry lowering
requires a complete selected-entry dependency closure; holes never lower. Unused incomplete
definitions do not block an otherwise complete entry.

## Derived workbench facts and reference domains

A workbench context packet is a pure bounded observation of one exact immutable workspace revision.
It carries its workspace, revision, machine-schema digest, closed purpose, targets and options,
canonical packet digest, included node facts, typed query outcomes, legal transaction/expression
codes, and explicit omission facts. Packet purposes select deterministic fact families; they do not
rank facts with a model. Target traversal includes the target-owned closure, exact direct
dependencies and their signatures, and owner anchors without pulling unrelated owner siblings.
Workspace review is deliberately broader. Full scan queries remain the correctness oracle.

Each included node receives one canonical packet-local alias `n1`, `n2`, and so on in Node-ID order.
Plan spelling adds `@` to distinguish an alias from a Node ID or `DraftSymbol`. An alias resolves to
exactly one persistent Node ID only under the packet's workspace, revision, schema, and digest.
Aliases are never semantic identities, names, transaction-local symbols, allocator inputs, history
facts, artifact fields, or implicit current-head lookups. A mutation using an old packet still names
the packet revision as its exact transaction base and therefore rejects with the ordinary revision
conflict after head movement. A historical pure Run may deliberately use an old exact revision.

A compact edit plan is an ephemeral proposal projection. Parsing resolves packet aliases and
produces the existing closed `ApplyTransactionRequest`; it allocates no persistent identity and adds
no semantic form or validator. The selected CLI command supplies commit versus validate-only mode.
Plan spelling, packet layout, aliases, and derived text are discarded before transaction
normalization and therefore cannot affect candidate allocation order, snapshot hashes, artifacts,
change digests, or execution. A derived semantic view is one-way review text and is not required to
parse or round-trip. Saving, changing, deleting, or corrupting a packet or view cannot alter any
workspace revision.

## Artifact and durable HEAD

A `.lkjscript` artifact uses format version 5 and semantic schema identity `lkjscript-spg005`;
format-4 and older artifact bytes reject without a compatibility reader. It has fixed magic and
semantic schema ID, little-endian integers, checked u64 counts, canonical node/tombstone order,
allocator/root state, and a BLAKE3 snapshot hash. `ConstBytes` stores a checked canonical length
followed by raw octets; public base64 and runtime handles are absent. The exact
artifact/decode policy accepts at most 67,108,864 bytes (64 MiB) for the complete artifact and at
most 1,048,576 UTF-8 bytes (1 MiB) for each encoded name. Commit preflight applies the same artifact
policy before publication. Decode rejects policy overflow, truncation, integer overflow, invalid
UTF-8, unknown tags, duplicate or wrong-workspace IDs, empty or category-duplicate names, invalid
containment or references, hash mismatch, and trailing bytes. Count work is bounded from remaining
artifact bytes and exact minimum record widths; there is no separate semantic node or tombstone
ceiling. Durable workspace IDs and revision file names must use their one canonical path spelling.
Accepted decode followed by encode is byte-identical. Core IR, managed handles, ownership plans,
machine code, caches, profiles, receipts, and protocol frames are absent.

`LKJHEAD7` directly replaces the old HEAD format; there is no compatibility reader. `LKJHEAD6`
rejects because protocol-v8 canonical JSON and the concat proposal vocabulary changed the persisted
idempotency fingerprint meaning. It
is a checked, independently bounded (16 KiB) non-semantic publication record containing head
revision/hash and, when present, one compact keyed fingerprint/receipt with exact bounded symbolic
returned bindings. The HEAD7 grammar has no numeric-symbol interpretation and
stores enough receipt data for exact idempotency replay. It never contains a full diff or allocation map.
Restart decodes every retained artifact, validates adjacent history, and recomputes/validates
receipt facts against retained snapshots before accepting HEAD. The fingerprint uses the v8 domain.
Corrupt or old HEAD bytes reject.
