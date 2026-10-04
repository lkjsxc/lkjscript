# Structural owned choices

Graph 20 adds `OwnedChoice`, an explicit affine sum of a fixed nonempty set of
named payload types. Exactly one case is selected at runtime. A choice is not an
ordinary nominal variant, Option or Result, and its ownership does not depend on
which case is selected. Even a selected Unit or I64 case must be moved, completely
matched, or dropped rather than copied.

At least one case must carry direct owned data: ByteBuffer, OwnedI64Cell, a finite
owned product or choice, or an exactly scoped Owned parameter. Other payloads must
satisfy the existing closed ordinary first-order proof, following all nominal
members and actual arguments, including phantom arguments and untaken cases.
Development 0.1.71 also admits exact in-scope [transferable ordinary parameters](transferable-types.md)
inside ordinary payloads. Unconstrained open parameters, functions, tasks, streams,
secrets and capability resources are not ordinary choice payloads. Ordinary
records, variants and unrestricted containers still cannot contain owned choices.

## Meaning and lexical scope

```text
(owned-choice (case accepted I64) (case rejected T))

(choose-owned
  (type (owned-choice (case accepted I64) (case rejected T)))
  (case rejected) (local original))

(match-owned
  (type (owned-choice (case accepted I64) (case rejected T)))
  (local outcome)
  (case accepted (binding value (type I64)) (in ACCEPTED_BODY))
  (case rejected (binding original (type T)) (in REJECTED_BODY)))
```

The explicit type is checked meaning, never an ownership certificate. Case names
are unique and canonical type/arm order is by name. Existing field-count, depth,
request-size and proof-work limits apply; no bound is raised. Branch-state forks
reserve their complete slot work before copying, and joins charge each examined
slot. Otherwise-unused live owners are not free proof metadata. Owned case analysis
streams capability-flow states rather than retaining a state for every arm. The type-depth proof
covers the longest path through shared type nodes and materialized generic
substitutions, including mixtures of products and choices.

Construction evaluates exactly one payload expression of the selected case's exact
type. An owned payload must be an exact live owning local, not a borrowed local or
an unbound temporary. An ordinary payload follows its ordinary evaluation rules.
No unselected case expression is evaluated. A trap during ordinary payload evaluation
is not converted into a choice result.

Case analysis requires an exact live owning local of the annotated choice type and
consumes it once. Every declared case must have exactly one arm, with no fallback,
missing case, duplicate or extra case. Each arm declares a distinct payload binding
with the exact annotated case type. Its identity is visible only within its body;
other arm bodies and the source expression cannot reference it. Separate arms may
reuse a name but not a binding identity. The bodies must have the same result type.
The ordinary lexical and ownership validators check every arm before publication,
including arms that a particular execution will not select.

Only the selected body executes. A selected owned payload is transferred, not cloned.
Unused payloads drop at arm exit, and an unmatched unused choice drops with its
selected payload. The consumed parent cannot be reused. Rights to surrounding
owners are checked independently for each arm and conservatively merged afterward;
an owner consumed on any path cannot be used after the match without a new owner.

Choice values compose with fixed owned products and rank-one Owned function
parameters/results. Exact static implementation parameters and cross-package
forwarding retain their existing contracts. A closed choice may serve as an exact
implementation Self where its complete method signatures satisfy those contracts.
Whole-choice synchronous borrowing/reborrowing is permitted. A loan cannot move or
return an owned payload. [Borrowed case inspection](owned-borrows.md) uses an
exhaustive `match-borrowed-owned` scope: owned payloads are read views, ordinary
payloads follow their admitted copy rules, and the whole choice stays guarded
throughout the selected arm. Escaping or mutable references, partial moves and
implicit witness search remain unsupported. Named
tasks may consume and return choices under [same-task transfer](owned-task-transfers.md),
while preserving their independently checked effects and resource requirements.

Canonical native drafts retain the accepted type and operation metadata. Unchanged
re-entry preserves identities. Literal-only edits visit selected construction
payloads, match sources and every arm body, then compare the complete canonical
intent, including case names, exact type operands and binding annotations. Only
complete equality except the selected literals permits identity-preserving edits.
Other edits use the existing whole-body replacement and validation path.

## Runtime ownership, admission and failure

The bootstrap carrier is a separately typed `OwnedChoice` token with an inline
selected-case index and one payload under the existing sealed composite storage
owner. It does not add a second ownership protocol. Exact invocation origin, closed
type, selected case and payload are admitted at construction and checked on transfer.
Raw token cloning produces an inert marker; it does not copy payload ownership.
Borrow tokens retain only scoped read rights. Consumption rejects inert, stale,
foreign, borrowed or actively loaned owners before detaching storage.

Construction reserves the one-element payload vector, wrapper difference and
composite token/control storage before growth. Cancellation checks bracket those
reservations. A rejected reservation or cancelled operation releases its owned
input through ordinary failure cleanup. Complete selection transfers the payload
without duplicating its owned allocation. Mixed product/choice cleanup uses the
bounded iterative composite traversal rather than recursively nesting owner drops.
Surviving inert markers do not keep a payload alive after the actual owner drops.
Cleanup does not run user-defined methods or roll back independent effects.

These charges are cumulative modeled allocation admission, not RSS, allocator size
classes or a live-heap measurement. Retained allocation identity proves the tested
transfer did not replace that payload allocation; it is not a whole-program
zero-copy, speed or memory-use claim.

A rejected application operation can deliberately return its still-owned original
payload in a choice. This is a value-level contract, not exception catching,
automatic quota recovery, transaction rollback or a promise that retry is safe.
Independent effects can survive a later failure. Asynchronous channels still need
separate reservation, acceptance, cancellation and receiver-lifecycle semantics.

## External and retained boundaries

Raw entry/exit values, adapter types, session persistence, data/JSON codecs,
unrestricted containers and retained callable captures reject choices. Rejection
covers unused arguments, untaken cases, hidden nominal members and phantom type
arguments, not merely the selected runtime payload. Type annotations cannot grant
storage authority. Package availability and exact implementation identity confer no
capability grants.

Strict artifact loading validates retained canonical source, complete type closures,
all lexical bindings, ownership and exact compiled instruction meaning independently
of executing the program. Consistently rehashing a modified case index, branch
target, payload local or loan mode does not authorize it. A Local-only function
whose signature contains a choice still requires Graph 20; absence of a new
expression tag cannot disguise its generation.

## Encodings and availability

`LKJCHO01` is a disjoint version-1 type envelope. Ordinary and product type bytes
remain unchanged. Graph 20 appends choice operations, arm metadata and an explicit
owned-choice payload binding kind; Graph 14–19 readers remain available for their
retained source generations. Product-bearing source still needs Graph 19 and
choice-bearing source needs Graph 20 across owner, interface and artifact closures.
Package-interface layout 12 is unchanged.

The choice increment selected compiler unit 18, bytecode 14, artifact 25, authored request
23, compact discovery 27 and semantic validator 21. Requests without the new
extension retain their previous canonical request bytes. Predecessor derived
artifacts and compiler caches require rebuilding from retained accepted meaning;
no operational application-data migration or running-service replacement is implied.
Product version components remain opaque identifiers, not compatibility promises.
The [child-borrow contract](owned-borrows.md#encodings-and-evidence) owns subsequent
scope-operation, borrowed-binding and derived-layout cuts.

The [native guide](../guides/native-owned-choices.md) describes ordinary use. The
[continuation](../campaigns/20261002-owned-choices.md) separates focused tests,
source acceptance, mainline delivery and finalized distribution. Source availability
is not a new public binary release.
