# Current-format policy receipts through public source republication

Updated 2026-10-29 (Asia/Tokyo).
Status: all six maintained projects publicly republished and independently re-exported on
2026-10-27. Compiler proposals were subsequently adopted through public authoring;
their changed meaning and full staged-tree acceptance remain separate from the receipt-only cut.

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

## Observed maintained-project cutover

The [machine-readable evidence](20261027-policy-publication-evidence.json) records all six
input/output HEAD hashes, transport/container identities, semantic-state commitments, complete
source inventories, bootstrap hashes and public-operation outcomes. Every saved export still
matched both its recorded immutable container and the live pre-cut HEAD. Export reconstruction
was unnecessary, and no transitional receipt reader was introduced.

All six republications and current-only reopening exports succeeded. Each stale pinned
republication rejected with `publication_republish_stale` and left the new HEAD unchanged.
For each complete transport closure, every Owner, Type, Blob, Dependency and Retirement object
key, byte length and independent SHA-256 matched before and after. These complete-container
counts include transitive suppliers; the receipt separately records local inventories.

Five projects were already semantic genesis revisions and retained their semantic revision IDs
while their receipt-bearing HEAD records changed. The selfhost revision became
`rev_016c676fe85bfdc2a476b222ba26a3c6017863ef0a8b1fa8aeb0d46c183967f1`; its semantic state remains
`semantic_state_50c336e1c41ed4515a9ccb5c9db482c48f84513296d0dd22189e8dbc1e4b8cd5`.
Prepared publication compares the complete expected `HeadRecord`, including its record digest;
semantic revision equality alone is not permission to reuse a pre-cut prepared change.

## Subsequent compiler source adoption

After republication, one public transaction adopted 243 changed/new declarations across 44 modules,
including complete final parameter orders and explicit deletion of three obsolete parameters and
two obsolete functions. It accepted revision
`rev_e5fa56ece633987cb2f62a3e7cdda4aec58fa74aa9cc7008f71682ca289b2f51`, with HEAD hash
`f5436f724c2a88123f5db0ade85ec23af808a9dc70635beb490198c688027507`.
This was an ordinary semantic publication: its 13,654 created owners and 239 updates are not
part of the identity-preserving receipt republication. The pre-adoption hashes and inventories
above remain historical cutover evidence. The transaction selected no project tests.

Subsequent public readback verified nine owner pages containing 35,952 live owners, all 2,832
expected namespace entries and all 23 changed parameter orders. V3 completed at
2026-10-28T08:48:25.740Z with status `revision-inventory-and-signatures-verified`; its receipt
SHA-256 is `1aaa3d3ef1e5fe5906f75a0fd27d253bd0d7b80a8614cd5b3fc0164a1c50c428`.
The reader executable is bound separately from the publication executable. Readback and public
transport export left the accepted HEAD unchanged; the exported source container SHA-256 is
`cce8632f8a18faa2da549a3df134a3b679966f0fba5cfb903fa440f3e7166981`.

The new semantic state is
`semantic_state_66d01c7280e302fa390e1449e148893c23b0aea46b1fb7c7dc9033cc81a318ec`,
with 35,952 owners, 41 types and 24,864 retirements. This changed semantic inventory belongs to
source adoption, not the receipt-only republication. Maintained proposal replay subsequently
matched all 641 canonical units and the native backend closure. The focused Tier 1 measurement
observed Mode0 profile6 C2 = C3 and Mode1 profile5 artifact C1 = C2 = C3; exact image hashes,
measurement scope and remaining fixture/gate obligations are recorded in the
[capacity ledger](20261025-capacity-unbounding.md). These observations do not establish raw-source
Tier 2 self-compilation or final staged-tree acceptance.

The [source adoption evidence](20261028-selfhost-adoption-evidence.json) retains the accepted
revision, publication and reader executable hashes, changed parameter identities, transport
digest and focused native outputs separately from the receipt republication evidence.

The retirement-binding negative fixture, atomic memory replay and remaining definition-field
descriptor corrections passed their focused controls, including all 641 canonical units and
native diagnostic 12 for the hash-consistent final retirement mismatch. Applied followup
corrections cover nested-record reconstruction stubs, the scoped wide-address probe and GC
preparation exactness. The followup passed all 14 library and six real public CLI parameter/type
tests. Subsequent public GC passed for all six repositories, preserving every HEAD and complete
source transport SHA-256, digest and revision. Physical packs decreased from 662 files /
499,175,240 bytes to eight files / 79,757,990 bytes; both reported saturation flags were false
for every repository. The GC receipt SHA-256 is
`915238fdee4c3f12ad0bd64acce6c9f465a9cd8e991b482d8333d2b9fa0c24b7`.

All five maintained-asset comparisons matched exact bytes, and the public package checks
passed 116 standard, 44 application and 176 package-plan tests with zero failures and equal
differential results. The asset receipt SHA-256 is
`2e11c863ec3f76f71795464daf3ae9935312b59ae05e102d6601f24a7c13608f`.
Discovery regenerated all eight documents. These post-adoption maintenance observations are
recorded separately in the integration evidence; the historical republication identities
and evidence above remain unchanged. Physical GC/catalog capacity removal remains a separate
storage obligation. Exact-tree bootstrap, literal selfhost verification, explicit ignored
native measurement, changed checks, 29 fresh full gates with zero reuse, release target and
acceptance remain pending.
