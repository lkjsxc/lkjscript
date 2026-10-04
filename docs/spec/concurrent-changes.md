# Reviewed concurrent changes

Status: selected implementation contract; source and public acceptance remain
tracked in [status](../status.md). This contract adds explicit candidate refresh
to the [reviewed change workflow](semantic-cli.md#reviewed-change).

## Public workflow and authority

Two authors may prepare candidates from the same accepted revision. Publishing
one candidate advances HEAD and makes ordinary publication of the other stale.
Refresh is an explicit, stateless operation:

```text
change refresh (--input RECORDS | --input-file PATH) \
  --plan ORIGINAL_TOKEN --onto REVISION [--output PATH]
change apply (--input RECORDS | --input-file PATH) --plan REFRESHED_TOKEN
```

Refresh also accepts the same direct rename and extraction inputs as `change plan`.
Supply the original input with its original `request.base`, the ordinary `plan_`
token from its original review, and the exact current revision observed through
`status`. Refresh does not rewrite the request, change its base, publish meaning,
accept a refreshed token as its origin, or select a target automatically.

The successful response is `result status=prepared command=change.refresh`. Its
`refresh` record reports `onto`, `original-plan`, `intent-reads` and
`intent-read-guards`. The existing `plan` record returns a new `refresh_` token.
Inspect the renewed candidate and logical plan before explicit apply. `--output`
uses the same bounded atomic plan-file output owner as ordinary planning.

The original token binds the request commitment and original prepared candidate.
The refreshed token additionally binds the original prepared commitment, exact
onto revision and renewed prepared commitment. Tokens are deterministic review
bindings, not secret credentials. Logical plan files are derived review evidence;
they cannot be imported as accepted meaning or substitute for token validation.

## Authenticate intent and its dependency footprint

Capture one immutable current target and authenticate the original revision
through its accepted single-parent history. Reconstruct original preparation
under the current executable and compare its ordinary review token exactly.
An original revision without the required current valid witness cannot provide
a refresh origin. Missing or disconnected history, unsupported parent shapes,
corrupt lineage, cancellation and exhausted read admission fail closed.

Native decoding and authored lowering retain every canonical observation that
determined the original intent. Guards cover positive and negative owner,
namespace, dependency, retirement and ownership lookups. Relation queries guard
their complete ordered result, including an empty result; a truncated range cannot
prove absence. Guarded write bindings cover the exact before-values selected for
replacement or deletion. Every original guarded observation must reproduce at
onto before renewed preparation can proceed.

Different edited owners alone do not prove independence. A changed namespace,
reference selection, dependency, ownership edge or relation-query result can
conflict with otherwise distinct edits. Names are resolved at the original base;
a name that later selects another owner does not redirect the original intent.

Validation summaries and aggregate root/module implementation digests are
recomputed observations, not blanket intent guards. Capture the canonical reads
that select and lower authored operations; renew validation dependencies, impact
and tests against the complete target candidate. An unrelated change to derived
validation context may alter the renewed review without changing authored intent.

The footprint is canonical and independent of observation order. Its digest and
guard count are included in prepared review evidence. Fixed guard-count and byte
limits are exposed by capability discovery; exhaustion remains a resource failure,
not a conflict or a partially complete footprint. Ancestry uses one shared bounded
canonical-read meter, a fixed revision limit and cancellation checks throughout.
Original authored preparation, guard rechecking and renewed preparation share
the request's admitted read and work capacity; separate preparation stages do
not multiply that allowance. Native input decoding retains its separate fixed
admission, as in ordinary planning.

## Preserve identity and authored values

Renewed lowering preserves the original request commitment, selector identities,
ordered allocation sequence and complete authored after-values. Created owners
retain their original identities even when unrelated publication advances HEAD.
Refresh cannot invent new identities, retarget a selector, reinterpret a deletion,
merge two after-values or silently repair a conflicting authored write.

The unchanged guarded intent is lowered against onto, then receives complete
semantic validation, impact analysis and selected-test preparation. Every relevant
type, ownership, lifetime and effect obligation still applies. A renewed candidate
that is invalid at onto cannot publish even when its authored guards agree.
Allocation or after-value disagreement under unchanged guards is corruption;
ordinary changed guards are a semantic conflict.

Supplied type objects retain their complete closure in preparation and logical
evidence regardless of whether storage already contains their exact bytes.
Physical object availability only deduplicates staging. It does not remove a
supplied type, change intent, or turn another candidate's equal type insertion
into a semantic conflict. The response's `summary supplied-types` counts supplied
closure, separately from physical insertions.

A semantic no-change candidate retains the ordinary unchanged outcome and acquires
no publication authority. This is neither an automatic merge nor a branch or
multi-parent-history facility. Graph semantics and layouts retain their existing
owners; refresh provenance belongs to reviewed preparation and accepted history.

## Publication, retries and failure

Apply receives the original authored input and refreshed token. It reconstructs
both original and renewed preparation and verifies every bound commitment.
Publication rechecks the exact reviewed onto revision under the existing lock,
persists complete accepted objects, then atomically exposes the result. A writer
that wins between refresh and apply makes the unaccepted apply stale. No automatic
refresh or publication retry bypasses renewed review.

An accepted idempotency key retains its immutable original receipt. An exact
retry of the same original input and refreshed token reconstructs against its
recorded onto parent and returns that accepted result even after descendants
advance HEAD. It does not reinterpret intent or validate the historical candidate
against current descendants. Reusing the key with another target or candidate
conflicts. Refresh itself always requires explicit current HEAD; the accepted
retry exception belongs to apply.

Invalid origins, intent conflicts, malformed tokens, missing history, corrupt
lineage, budget exhaustion, cancellation and incomplete staging cannot partially
publish. A derived failure cannot undo already accepted meaning or change an
idempotent retry's immutable result. Existing joined cleanup and durable publication
rules remain required.

[Verification](verification.md#reviewed-concurrent-change-obligations) defines
independent positive, conflict, race, retry and resource evidence.
