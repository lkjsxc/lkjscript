# Identity economy for semantic entities

Date: 2026-08-25 UTC.

## Status

Proposed. The inventory and design audit are complete; scoped-storage implementation,
cross-package round trips, and complete-workflow measurements are required before acceptance.

## Problem

The normalized kernel currently represents modules, declarations, targets, declaration members,
parameters, lexical bindings, expressions, documentation, and annotations as independent global
owners. Each fine owner enters the semantic owner map, witness summaries, ownership and relation
indexes, transactions, diffs, retirements, package interfaces, and cold artifact records.

For a fine owner with one relation, the current fixed metadata floor is approximately 543 bytes
before the owner object, summary envelopes, ownership value, object indexes, and persistent-map
page overhead. Member vectors are also sorted by allocated identity, so random allocation order
leaks into semantic presentation and runtime layout. Hot execution already lowers exact graph
references to dense indexes and task-local slots.

## Proposed decision

- Retain durable package-level identity for modules, declarations, and targets.
- Retain typed 128-bit local continuity tokens for fields, cases, interface operations,
  requirements, ports, type parameters, parameters, expressions, and bindings.
- Scope member tokens to their declaration or operation. Scope body tokens to an exact body owner
  and role, such as function body, constant value, test actual or expected value, port
  implementation, or binding value.
- Make every external exact selector carry package and parent scope. A bare local token is never a
  valid public selector.
- Keep logical collection ordering explicit and independent from identity. Preserve local identity
  across rename, insertion, and reorder; allocate a new token after deletion and recreation or a
  move to another parent scope.
- Contract the global owner, witness, relation, diff, retirement, and tombstone domains to genuine
  package-level owners. Parent-owned member and body records may use segmented physical maps for
  sparse edits without making segment identity semantic.
- Remove allocated documentation and annotation identities. Store documentation by canonical
  content attachment and annotations as unique owner-scoped class/key slots. Nonsemantic
  attachments must not enter semantic revision identity.
- Keep immutable type and blob structures content-addressed, request symbols ephemeral, and
  compiler/runtime indexes derived and dense.
- Bind deterministic local allocation to repository, exact base, normalized request contract and
  digest, idempotency key, parent scope, local identity domain, declared symbol ordinal, and a
  deterministic collision counter.

## Alternatives

- Retaining every global owner has the lowest migration cost but preserves metadata, index,
  tombstone, request, and layout debt.
- Names, positions, or content digests alone are smaller but break continuity under rename,
  insertion, reorder, exact reference, deletion/recreation, and merge.
- Scoped syntax over the existing universal owner maps changes vocabulary without delivering the
  storage or locality contraction.
- Reducing token width before removing global infrastructure adds collision risk while saving less
  than scoping and contextual encoding.

## Evidence required for acceptance

The prototype must preserve rename, move, insert, reorder, subtree replacement, exact diagnostics,
deterministic replay, idempotency, branch allocation, collision rejection, delete/recreate, package
interface export, diff, history, and clean/incremental agreement. Foreign parent, body, package,
binding, and local-domain references must reject.

Measurements will compare the current and scoped representations for a 10,000-node body, 10,000
small functions, wide record/variant/interface/component shapes, a public generic dependency, and
long sparse history. Retained observations include global and local ID bytes, owner and index
entries, canonical and package-interface bytes, page/object reads and writes, synchronization,
compiler reuse, artifact bytes, elapsed time, and peak RSS. No provider-token or cost claim will be
inferred from byte counts.

## Consequences

This is a one-time private-format rewrite across kernel codecs, references, validation, authored
allocation, witness maps, package interfaces, compiler lowering, artifacts, reference execution,
queries, diffs, and tests. Compatibility readers, aliases, and migration codecs will not remain.
Dense runtime layouts can survive. Cross-package member references become wider than the current
bare global token, so the retained measurements must distinguish contextual local references from
fully qualified external references.

## Reversal condition

If parent-owned segmented storage makes a sparse edit proportional to the complete body or member
collection, refine the local physical index without restoring global semantic identity. Promote an
identity to broader scope only when a demonstrated cross-parent continuity consumer cannot carry
the parent scope.
