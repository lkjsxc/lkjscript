# Aggregate Affine Values

## Status

<!-- LKJ-STATUS id=aggregate-affine-values status=accepted-contract -->

**Accepted contract; implementation is not yet Current.** Current ownership
checking still rejects ownership-bearing nested aggregates in important paths.

## Derived Mode

Every product, active enum payload, option, result, and typed error derives one
mode from its fields:

- `copy`: semantically trivial copy;
- `immutable-value`: reusable source value implemented by borrow, structural
  copy, destination construction, or sealed sharing;
- `affine`: one owner and exactly one drop obligation.

Plain representation-independent fields cross constrained contexts without
runtime ownership work. An aggregate is not made affine merely by a surrounding
mode.

## Move, Borrow, And Drop

Field observation borrows the aggregate and does not clone. A whole-value affine
move transfers every initialized field and its drop plan. Initially, consuming
patterns move the whole aggregate; arbitrary partial moves remain rejected
until open-drop elaboration is verified.

Only the active enum payload is initialized or dropped. Branch joins preserve
one exact aggregate state. Match defaults to borrowed observation; consuming
match requires explicit source intent or a proven last use.

## Construction And Update

Construction records a destination selected by the memory plan: inline, stack,
caller destination, unique storage, ordinary region, or sealed region. Fields
initialize once, publication follows complete initialization, and failure drops
only initialized fields.

Immutable field update borrows immutable inputs and uses a selected copy/share
plan. In-place reuse requires unique last-use proof. No hidden clone is allowed.

## First Conformance Slice

`result path system-error` is the first aggregate-affine target. It must carry
an owned path or typed error through evaluator, validated VM, forced baseline,
forced proof, return transfer, and every failure cleanup path without tracing or
fallback.

## Acceptance

Promotion requires independent HIR and SSA verification of initialization,
whole-value moves, exact active payload, no double drop, no branch leak,
destination failure cleanup, logical resource charges, and four-engine
conformance for representative nested aggregates.
