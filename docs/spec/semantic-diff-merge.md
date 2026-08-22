# Semantic diff and merge

Status: normative for diff and merge contract 2 and draft contract 4.

## Diff

Diff compares two exact accepted revisions from the same repository and package. It reconstructs
owner state keyed by stable typed identity, not name, source line, encoded offset, or Git path.

For each identity in the union of both revisions it emits zero or more ordered classes:

- `added`: absent before and present after;
- `removed`: present before and absent after;
- `renamed`: the same identity has a different name;
- `moved`: the same identity has a different module or parent;
- `modified`: kind or normalized semantic value changed.

Name fields are removed from semantic comparison for owner kinds where rename is independent,
preventing a rename from being mislabeled as a body modification. The report binds base/result
revision and a domain-separated digest of their root digests. Item, byte, and work budgets apply;
truncation returns the next exact offset.

## History and branch allocation

Revision history is a DAG coordinated with, but not owned by, Git. Semantic branches are simply
different accepted descendants of one revision. Random 128-bit typed IDs allow independent branch
creation without coordination. Equal bytes do not collapse unrelated stable IDs. Exact parents and
repository identity distinguish common ancestry.

Git transports immutable semantic objects and review projections. Git line merging of packed
bytes is invalid. After a Git conflict or independent checkout, the semantic merge protocol must
select exact base, left, and right revisions.

## Three-way merge

The merge request names graph/merge contract, exact base, left, right, work budget, and optional
intent. Base must be an ancestor of both branches, and all revisions must belong to one authority.

Merge keys modules and declarations by stable IDs; dependencies by alias; targets by target ID;
and tombstones by typed deleted identity. For one semantic value:

- identical branch values are accepted;
- one unchanged side yields the changed side;
- two identical changes are accepted once;
- compatible module-field changes may compose; and
- incompatible concurrent create/change, delete/modify, missing owner, or invalid merged meaning
  yields a closed conflict.

The merged candidate is fully canonicalized and validated. Preview returns `ready` and a diff
digest without publication. Apply requires current HEAD to equal left or right, publishes one
revision with both exact parents, and returns one receipt. A result equal to current meaning is
`semantic_no_change`. A moved HEAD is `stale_head`. Any conflict publishes nothing.

Conflict IDs are deterministic domain-separated IDs derived from the exact merge request and
canonical conflict ordinal. Reports are bounded to 10,000 conflicts. Contract 2 returns conflicts
in the merge result; persistent interactive conflict drafts and a dedicated conflict-resolution
command remain unimplemented. Resolution currently occurs by creating an explicit repaired draft
or change and then rerunning merge.

## Identity-sensitive cases

Rename plus independent body change composes when both retain the same declaration ID. A move plus
independent body change composes under the same rule. Two unrelated creations with equal names or
bodies remain different identities and may still fail namespace validation. Delete plus modify is
a conflict. Clone never aliases the original identity. Exact backup restore recreates the backed-up
repository and revision identities in a private stage; it is not a merge operation and cannot
silently synthesize new identity.

## Review projection

The deterministic review projection records repository/revision/package identities, parents,
dependencies, targets, tombstones, and ordered span-free module meaning. It is labeled
`non_authoritative_review_projection`, has a separate digest contract, and cannot be imported or
applied. Full projection is intentionally explicit and out of band; routine review uses bounded
semantic diff and exact `inspect owner` or `history show` expansion.
