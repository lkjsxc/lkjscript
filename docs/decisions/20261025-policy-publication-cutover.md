# Current-format policy receipts through public source republication

Status: implementation under verification; no maintained project has been republished yet.

## Problem and authority

Optional exact change budgets and honest saturation-bearing observations change operational
request, plan and receipt representations. Existing repository opening authenticates the current
receipt and its parent idempotency transition. Merely changing the receipt decoder would prevent
the updated authoring tool from opening an old maintained project. Two transitional publications
would still leave older receipt-dependent history reachable.

The accepted meaning graph remains authoritative. Reconstructing the compiler from authoring
text would unnecessarily replace durable identities. A permanent old/new receipt decoder would
keep a compatibility obligation that this cut explicitly does not require.

The existing public `package current export --kind transport` operation already exports a complete
code-containing canonical package closure, including private owners, types, retirements, local
blob bytes and exact dependency selections. Package containers exclude publication receipts.
Transport admission verifies their complete source-object inventory and canonical commitments.
The maintained-affine-extraction contributor test already exercises public recreation with those
transports and compares every semantic inventory.

## Decision

Add the supported public operation:

```sh
lkjscript --project PATH package current republish \
  --kind transport --transport TRANSPORT_DIGEST --input-file CONTAINER \
  --expected-head LOWERCASE_SHA256
```

It starts one current-format acceptance and retry history while preserving complete program
meaning, repository/package identities, owner identities, types, retirements, local blobs and
exact dependency selections. Semantic revision identities may change because genesis has no
semantic parents. Both old and new identities are reported. This is an explicit operational
format cut, not an ordinary source edit.

The operation uses the existing exclusive repository lock. Before staging, it checks the exact
hash of the complete current HEAD and authenticates the referenced RevisionRecord. The supplied
transport must bind that exact RevisionCore and semantic-root digest. Matching only the
semantic-state digest is insufficient. The complete source-container SHA-256 is retained in the
new canonical receipt intent and the public result.

The normal whole-source transport admission and full initial-publication preparation run again.
The candidate's owner/type/retirement/dependency/blob inventories and attached dependency
interfaces must equal the admitted source inventories. Normal immutable object staging,
dependency verification, closure verification and atomic HEAD replacement are reused.
Cancellation or rejection before replacement cannot change the accepted HEAD. Visibility and
durability failures after the rename retain the ordinary reconciliation semantics.

The operation never decodes an old receipt or treats an old semantic certificate as fresh
admission evidence. It does not edit existing canonical objects. Old objects can remain as
unreachable physical history until ordinary garbage collection; no legacy parser is retained.
The current graph/type/package formats remain unchanged by the receipt-only discriminator.

## Retry boundary

A fresh history deliberately does not import old idempotency bindings. Previously submitted
requests retain their old exact base revision and current-format request commitments. They
cannot become a new implicit request at the new head: stale bases and old budget/plan profiles
are rejected. Repeating the republication itself with the old HEAD hash also rejects as stale.
The result includes the new HEAD hash for explicit reconciliation.

Tests include a real keyed publication before the cut, then verify that replaying its old
prepared publication is stale and cannot duplicate its effect. A caller choosing a new explicit
base and a new current-format request is issuing new work.

## Bootstrap sequence

1. Build the disclosed Rust bootstrap while it can read current accepted receipts.
2. Export every maintained project through the existing public transport operation. Record the
   bootstrap hash, exact input HEAD hash before and after, container SHA-256 and public export
   receipt. Reject any change of HEAD during export.
3. Finish and test the optional-budget, plan and receipt cut. Build the current-only authoring tool.
4. Republish each immutable export with its exact old HEAD hash. Verify complete meaning and
   identity equality, current-only reopening, fresh history, dependency source admission and
   deterministic stale retry behavior.
5. Apply actual compiler proposals through public authoring on the new current-format heads.
   Source changes are distinct from this identity-preserving operational republication.
6. Regenerate public capabilities documentation and maintained derived inputs through their
   supported operations. Run exact staged-tree acceptance gates and record final Git identity.

The first export preparation in this session covered the six Git-tracked maintained project
HEADs: `applications/lkjournal`, `packages/standard`, `tools/native-guides/project`,
`tools/native-package-plan/project`, `tools/native-policy/project`, and
`tools/selfhost/project`. All six exports left their input HEAD bytes unchanged. Their exact
hashes, lengths and bootstrap identity are in the retained cutover export receipt. Exporting
does not mean that republication or full Tier 2 acceptance has completed.

## Verification

Focused controls cover exact source/HEAD binding, full meaning and identity preservation, stale
keyed retry rejection, malformed/truncated and foreign source transport refusal, cancellation,
previsibility storage/HEAD failures, post-rename recovery, and two concurrent republications of
the same pinned head. Normal repository durability, source transport, authoring, selfhost and
release gates remain required before sealing the cut.
