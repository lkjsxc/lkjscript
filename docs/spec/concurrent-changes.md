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
uses the same complete atomic plan-file output owner as ordinary planning.

The original token binds the request commitment and original prepared candidate.
The refreshed token additionally binds the original prepared commitment, exact
onto revision and renewed prepared commitment. Tokens are deterministic review
bindings, not secret credentials. Logical plan files are derived review evidence;
they cannot be imported as accepted meaning or substitute for token validation.

## Symbolic transactions

`tx --input-file delta.lkjx [--run TARGET] [--compact]` resolves and executes one
proposal against a captured HEAD. `--input FORM` is the inline equivalent. Success
is one complete JSON line with fallible buffer growth; failures use ordinary diagnostics.

```text
(tx (base @head)
  (upsert-module math
    (upsert-function square (visibility public)
      (param value I64) (returns I64) (effect pure)
      (body (call std::multiply (local value) (local value))))
    (upsert-test twelve-squared
      (actual (call math::square (i64 12))) (expected (i64 144)))))
```

Upserts preserve matching module, declaration and member identities. A name occupied
by a different kind rejects. Native nesting, complete ordered signatures, type/body
rules and explicit supplier aliases remain authoritative. `param NAME TYPE` is a
parameter upsert. Omitted visibility preserves the existing value or defaults to
private. Qualified names select declarations outside the edited unit; forward names
within the proposal are collected before bodies. Quoted text is never rewritten.
`@module("math")` and `@fn("math::square")` require existing owners of that kind
and scope. An explicit revision may replace `@head`; stale bases never auto-refresh.

The displayed value-parameter list is the complete final order for an edited
function, external declaration or interface operation. An insertion or reorder
lowers to typed `SetParameterOrder` intent when additions and deletions alone would
not produce that order. After explicit deletions, the engine requires a bijection
of the parent's surviving parameter inventory: each exact existing identity or
new request-local symbol occurs once and belongs to that parent. Missing,
duplicate, foreign, deleted or wrong-kind members reject, as do repeated order
operations for the same parent. Ordering changes only the parent's parameter
vector; retained parameter identities and records survive. Parameter type/use
changes and all affected call sites still require ordinary complete-candidate
validation. Type, effect and requirement parameters retain their existing order
and append-only addition rules.

Explicit deletion can share the same atomic transaction with caller/body updates:

```text
(delete function math::obsolete (policy owned-closure))
(delete module obsolete-module (policy reject))
(delete parameter math::combine::obsolete (policy reject))
(delete parameter transport::channel::send::obsolete (policy reject))
```

The selector names an exact module, module-scoped declaration family or value
parameter in the bound base. Parameter paths have the form
`MODULE::CALLABLE::PARAMETER` for a function or external declaration, or
`MODULE::INTERFACE::OPERATION::PARAMETER` for an interface operation. A name
introduced only by the current proposal cannot be selected for deletion.
The policy is mandatory: `reject` refuses owned children, while `owned-closure` retires
the selected owner and its ordinary owned closure. All forms use the existing
`delete.owner` engine, including incoming-reference, duplicate-deletion, retirement
and candidate-integrity checks. A mismatched family or stale base rejects the entire
transaction. Omission from an upsert never implicitly deletes a declaration or
contract child; a removed parameter must have an explicit deletion in the same
complete edit.

The corresponding compact ordering operation is
`set.parameter-order function=DECLARATION parameters="OWNER_OR_SYMBOL ..."` or
`set.parameter-order operation=OPERATION parameters="OWNER_OR_SYMBOL ..."`.
Exactly one parent selector is required. An empty `parameters=""` list asserts an
empty surviving inventory; it does not delete members. Ordering, additions,
explicit deletions and caller repairs share one atomic publication.

The implicit `std` alias requires the already-selected exact builtin dependency,
included by the command starter. A local module named `std` takes precedence.
Transactions do not stage or select dependencies implicitly. Input, declaration
inventories and result encoding have no synthetic aggregate quota. All 46 optional
change dimensions use the same exact policies as native changes: `(policy DIMENSION
unbounded)` selects no allowance, and `(policy DIMENSION N)` selects the exact u64
allowance, including zero and the largest value. Canonical object representation and
remaining recursive lifecycle boundaries are independent pending cuts, recorded in
the [capacity ledger](../decisions/20261025-capacity-unbounding.md).

The ordinary change engine validates the complete candidate and computes its
transitive reverse-dependency test cone. A read-only overlay admits the complete
source closure and canonical test inventory, compiles it, and runs only selected
tests in both production and independent reference evaluators. Optional `--run`
uses an empty argument list and requires a pure target. Execution, cleanup and
complete result encoding finish before the ordinary locked publisher compares the
exact base and advances HEAD. Failed tests, invalid selectors, capacity failures
and stale candidates publish nothing. Verified counts bind the accepted receipt.
An empty semantic delta returns `unchanged`; no historical acceptance or cached
compilation substitutes for current validation.

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
