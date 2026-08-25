# Semantic revision identity and acceptance evidence

Date: 2026-08-25 UTC.

## Status

Accepted and implemented in the normalized publication path. That path is not yet the complete
released command surface, so this record does not claim public cutover.

## Decision

A revision is a history identity, not a validation or storage receipt. Its identity commits to:

- the semantic-state contract and graph contract;
- the repository identity;
- zero, one, or two canonical parent revision identities; and
- one storage-independent semantic-state digest.

The semantic state commits to package identity and name plus the sorted logical owner,
dependency, and retirement bindings. Immutable owner and dependency object digests transitively
commit their referenced semantic records. Repository identity is excluded from the state digest
but included in revision identity. Physical page roots, page splits, pack placement, catalogs,
indexes, and caches are excluded.

Parent identities are intentionally part of revision identity. Equal semantic state reached from
different accepted parents therefore has one content digest but distinct revision identities.
This preserves exact history and makes the content/history distinction explicit.

A separate publication binding locates the physical semantic root and binds parent record
locators, validation witness, validation certificate, validator contract, idempotency history,
normalized transaction, semantic diff, and publication receipt. HEAD selects a revision record,
so later evidence may produce a different record and HEAD binding without changing the revision
identity. Verification rejects evidence for another repository, semantic state, physical root,
package, or witness contract.

Verification and benchmark receipts remain outside accepted publication and use their own typed
digest domain.

## Alternatives

- Hashing the physical semantic root was rejected because a page or root codec change would
  rename unchanged meaning.
- Hashing witness and validator identities was rejected because validator upgrades are new
  acceptance observations, not semantic edits.
- Using only semantic content and omitting parents was rejected because the maintained history
  model needs distinct branch and merge nodes.
- Treating transaction, diff, idempotency, or receipt objects as revision meaning was rejected;
  each is independently bound publication or operational data.

## Evidence

The persistent-map suite compares sparse summaries with an independently reconstructed logical
radix commitment, including randomized edits and alternative physical splits. Kernel tests show
equal logical snapshots retain one semantic-state digest across repository and physical-root
changes. Publication tests show evidence, validator, transaction, diff, receipt, idempotency, and
physical-root rebinding changes the revision record digest but not the revision identity. Focused
publication tests also reject mismatched state, witness, parent, and repository bindings.

## Consequences and remaining work

Physical semantic roots remain required locators and integrity objects, but they are not revision
meaning. Compiler, package, backup, and public status consumers must carry semantic state and
physical roots as distinct typed fields. A public revalidation operation is still absent; the
record model permits it, but acceptance must not be claimed until that operation has atomic
publication tests. The predecessor revision implementation remains pending deletion at public
cutover.

## Reversal condition

Revisit the field set only if a normative semantic or history consumer cannot be authenticated by
semantic state, repository identity, and parent revisions. New validation, cache, layout, or
receipt fields are not sufficient grounds to expand revision identity.
