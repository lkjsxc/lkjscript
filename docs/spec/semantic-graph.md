# Semantic Program Graph specification

## Authority and data classes

The Semantic Program Graph (SPG) is the only mutable program authority. `lkjscriptd` is its only
live writer. A published `Snapshot` is immutable and revision-labelled.

Durable data is classified as follows:

- **semantic/workspace state:** workspace identity, stable node identities, ownership and ordered
  body slots, typed operation data, direct value references, allocator state, tombstones, revision,
  and selected package entry;
- **presentation:** package, module, function, and parameter display names; names are never identity
  or references, but the bootstrap snapshot hash includes them so an artifact detects every stored
  state change;
- **derived:** completeness blockers, query summaries, semantic diffs, types derived from operation
  contracts, Core IR, diagnostics, and timings;
- **executable:** verified Core IR and interpreter values, which are never written into an SPG
  artifact.

No source text, syntax node, source span, arbitrary property map, arbitrary edge label, compiler
index, runtime address, diagnostic, profile, or cache is canonical graph data.

## Closed schema

`src/schema.rs` owns the closed Rust schema and explicit stable boundary tags. Unknown tags reject.
The implemented containment shape is:

```text
WorkspaceRoot -> Package -> Module -> Function -> Region -> Block -> Operation
                                      \-> Parameter
```

A function may have no body while incomplete. An attached function region has exactly one ordered
block in this bootstrap. A block has ordered non-terminator operations and exactly one separately
owned `return` terminator. Direct references and operands are distinct from owned children.

A value is either a stable parameter Node ID or `(operation Node ID, checked output index)`. Dense
Core IR value IDs never appear in a snapshot, query, diff, protocol request, or artifact.

Every non-root live node has exactly one owner and occurs exactly once in that owner's typed child
slot. All live nodes are reachable from the one root. Ordered slots use vectors; canonical table
iteration uses `BTreeMap`, so observable order does not depend on hashing or allocation.

## Identity

A `WorkspaceId` is 128 random bits. A `NodeId` is `(WorkspaceId, nonzero u64 serial)`. The root is
serial 1 and allocation is monotonic. Names, content, map position, hashes, and addresses do not
participate in Node ID allocation.

Transactions stage allocation. Rejection and dry-run leave the published allocator unchanged.
Deletion tombstones every owned ID; old retained snapshots keep their nodes, and later allocation
never reuses a tombstone. Every serial below the allocator frontier is exactly one live node or one
tombstone. Revision zero has root serial 1 only. Durable adjacent revisions require a stable root,
monotonic allocator and tombstones, no resurrection, and stable kind/owner/operation contract for
surviving IDs. Rename and compatible constant replacement preserve Node ID.

A `LocalHandle` is a u32 transaction-local symbol. All create handles are assigned staged IDs in
operation order before mutation begins. Handles never enter a snapshot, artifact, diff, or query.

## Typed transactions

Every transaction names one workspace and base revision, optionally supplies an idempotency key,
and carries an ordered batch of closed `TransactionOp` variants. Implemented operations create each
bootstrap node kind, attach function bodies and package entries, rename named nodes, replace a
contract-compatible operation, replace an operand, and delete an owned subtree.

The daemon checks a retained idempotency record first only to recognize an exact already-committed
retry whose original base revision remains retained and fingerprint-bound. That replay does not
reapply mutation. Every unseen request then requires the current base revision. Mutation occurs in a cloned staged
map. The final candidate is validated in deterministic ID and slot order, diffed, encoded, durably
committed, and only then published in memory. A successful non-dry transaction publishes one
revision. Empty and canonical no-change transactions reject with `NoChange`.

Validation checks workspace and kind domains, ownership, typed slot targets, unique sibling lookup
names, parameter ordinals, value scope and order, operation arity and result indexes, and exact
operand types. Rejection returns structured fields and publishes no revision, durable head,
allocator change, or tombstone.

Deletion is iterative. It rejects if any surviving direct reference points into the owned subtree;
it never silently cascades through independent dependents.

## Incompleteness

A missing function body and `OperationKind::Hole { expected }` are valid incomplete states. Holes
have stable Node IDs and exact expected types. Queries return structured blockers. Entry lowering
walks only the selected entry dependency closure and rejects any reachable blocker; holes are never
lowered to a value or trap. Package-wide queries also report blockers in unused definitions.

## Artifact

A `.lkjscript` artifact has fixed magic, artifact format version 1, a fixed semantic schema ID,
little-endian fixed-width integers, explicit u64 section counts, canonically ordered node and
tombstone tables, allocator state, root identity, and a BLAKE3 snapshot hash. The hash covers the
workspace, revision, allocator, tombstones, graph, and stored display names.

Decoding applies an explicit resource policy at the artifact boundary, then rejects truncation,
overflow, invalid UTF-8, unknown tags, duplicate IDs, invalid references, wrong kinds, invalid
containment, hash mismatch, and trailing bytes. Decode followed by encode is byte-identical. The
artifact contains no Core IR, machine code, cache, profile, session alias, or protocol frame.
