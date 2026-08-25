# Identity economy for semantic entities

Date: 2026-08-25 UTC.

## Status

Proposed. The inventory and storage-shape audit are complete. Commits `ad16477d` and `44670505`
retain the private typed-token/key foundation with package-local stored keys and package-qualified
external references. Dependency-closed authority migration, cross-package round trips, and
complete-workflow measurements are required before acceptance.

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
  implementation. Binding initializers remain inside the exact enclosing body scope.
- Make every external exact selector carry package and parent scope. A bare local token is never a
  valid public selector. Omit the package from stored scoped-map keys because the owning semantic
  root already binds exactly one package.
- Keep declaration-member ordering as typed intrusive links independent from identity. Preserve
  local identity across rename, insertion, and reorder; allocate a new token after deletion and
  recreation or a move to another parent scope. Expression and binding membership is token-sorted
  only for canonical storage; expression child vectors and the root graph own program order.
- Contract the global owner, witness, relation, diff, retirement, and tombstone domains to genuine
  package-level owners. Store fine records in one package-wide persistent map keyed by exact
  parent/body scope, local domain, and token. Bind its logical `MapContentRoot` into semantic state
  while keeping its page root in publication evidence. This avoids both whole-parent rewrites and
  a root/catalog per small scope.
- Remove allocated documentation and annotation identities. Store semantic documentation by
  attachment target plus content digest and annotations as unique attachment-target/key slots in
  the scoped map. Nonsemantic attachments live in a separately revision-bound presentation record
  and must not enter semantic revision identity.
- Keep immutable type and blob structures content-addressed, request symbols ephemeral, and
  compiler/runtime indexes derived and dense.
- Bind deterministic local allocation to the exact base through a typed normalized change digest,
  parent scope, local identity domain, declared symbol ordinal, and a deterministic collision
  counter. Request-local spelling and transport bytes must not affect that digest.

## Alternatives

- Retaining every global owner has the lowest migration cost but preserves metadata, index,
  tombstone, request, and layout debt.
- Names, positions, or content digests alone are smaller but break continuity under rename,
  insertion, reorder, exact reference, deletion/recreation, and merge.
- Scoped syntax over the existing universal owner maps changes vocabulary without delivering the
  storage or locality contraction.
- A map/content root per parent scope improves isolation for unusually wide bodies but creates a
  second-level catalog, durability rules, and fixed overhead for many tiny scopes before evidence
  shows a benefit.
- Reducing token width before removing global infrastructure adds collision risk while saving less
  than scoping and contextual encoding.

Operation parameters remain exact members of an interface operation, but they are not executable
local values: the current language has no operation body or runtime binding for them. The cutover
therefore removes the unsupported operation-parameter local-expression form instead of carrying a
dead capability into the current contract.

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

If prefix-scoped edits in the package-wide map are materially nonlocal for wide bodies, promote
physical sections to parent-owned content roots plus an external catalog without restoring global
semantic identity. Promote an identity to broader scope only when a demonstrated cross-parent
continuity consumer cannot carry the parent scope.
