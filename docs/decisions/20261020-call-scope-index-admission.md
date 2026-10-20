# One-descent callable scope-map admission

## Slice decision

- Boundary: Rust bootstrap proof-work capacity while adding the next E1 source
  validator, within the unchanged 10,000,000-unit callable-flow limit.
- Priority: the ordinary public transaction for `source-table`, `source-sort`,
  `source-edges` and `source-containment` fails with resource diagnostic
  `kernel_callable_flow_work` before publication. This is a demonstrated dependency
  of the current E1 slice, not a general bootstrap optimization campaign.
- Expected effect: eliminate the second tree descent when inserting an exact
  call-site scope substitution. Existing empty substitutions remain explicit;
  no scope-validation result is inferred from absence. Preserve exact keys,
  duplicate consistency checks, cancellation and reservation-before-insertion.
- Acceptance: measure old and new charged work on the same independent call-site
  inventory; compare complete retained mappings and duplicate behavior; prove
  mismatched substitutions reject; prove storage/work refusal and cancellation
  cannot insert a new entry; rerun composed/callable-flow tests and the rejected
  public authoring transaction without increasing its budgets.
- Remaining limits: other source admission work and native execution storage are
  unchanged. A lower modeled proof-work count is not an allocator, RSS, elapsed
  time or billing measurement. This change supplies no Tier 2 semantic completion.

## Contract

A BTreeMap Entry operation performs one exact call-site key search for lookup and
insertion. Prepare and charge new metadata only for a vacant entry, before exposing
it. Occupied entries compare the complete retained substitution with the proposed
map, including an explicitly empty map. Reserve through the same checked metadata
accounting used by the rest of composed admission. Charge one conservative tree
search instead of two because the second search is no longer executed.

## Measured acceptance

The independent 4,096-site fixture charges 4,631,872 units for the previous
two-search path and 2,315,936 for the entry path. Both retain exactly the same
mappings and 393,216 modeled metadata bytes on the verified x86_64 target.
All 77 callable-flow tests pass, including work/storage/cancellation refusal and
conflicting-substitution controls. The previously refused four-module transaction
then publishes 849 owners with evaluator equality under the unchanged public cap.
Subsequent membership/scope publication and the exact reference-kind correction
also pass ordinary symbolic transactions. These are proof-accounting and admission
observations; no native allocator or RSS reduction is claimed.
