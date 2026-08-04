# Aggregate Affine Values

## Status
<!-- LKJ-F aggregate-affine-values accepted-contract f14a7Gdty3254QMAwwPOB9ICERk_br-acNiRpKhuhm8 -->


**Current for the generic nonrecursive deterministic island.** Compact
structural-root execution and four-engine evidence cover deterministic products,
enums, options, results, strings, paths, bytes results, exact destinations,
whole-value transfer, and cleanup. General recursive products and enums,
persistent pairs/lists, partial moves, and borrowed returns remain outside this
Current slice.

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

## Deterministic Closure

One recursive algorithm evaluates a monomorphized type graph. Inline and static
values, deterministic leaves, and already eligible nonrecursive aggregates are
closed. A list/pair, captured closure, unknown type argument, recursive
product/enum SCC, or registered legacy field blocks closure. The algorithm does
not inspect declaration names; `option`, `result`, and built-in errors use the
same enum machinery as user declarations.

No mixed bridge is legal. A traced aggregate cannot contain a dynamic structural
root or affine resource, and a deterministic aggregate cannot contain or borrow
a collector-dependent value. HIR verification and bytecode validation each
reconstruct this property independently.

## Destination Construction

Construction uses a private typed destination selected as inline, stack, caller
result, unique structural owner, ordinary region, or sealed-region builder. Each
field initializes once. An enum initializes its tag and only the active payload.
Publication consumes the complete destination; incomplete and double
initialization are errors.

Every fallible field names an exact cleanup plan. Abort ends live loans, drops
initialized affine fields and deterministic roots in reverse successful
initialization order, preserves the primary failure, attaches only bounded
cleanup failures, and publishes no root. Destination forwarding and unique reuse
are optimizations only after proof and do not erase logical construction charges.

## Borrowing And Transfer

Field observation and non-consuming match borrow the whole aggregate. Direct
calls infer a shared borrow when the parameter signature permits it. Loan end is
placed after the last reachable use across branch and loop fixed points. A
consuming match may transfer the complete active payload only when its context
consumes the value or exact last-use analysis proves no later use.

Whole-value move invalidates the old structural key before publishing or
transferring the new owner. Whole-value drop removes the root projection and
executes the domain's exact reverse-order drop plan. Arbitrary partial field
moves remain rejected.

## First Conformance Island

Current generic fixtures include `option string`, `result path system-error`,
`result bytes system-error`, a path/string product, byte-vector/scalar product,
an enum containing a deterministic product, and nested option/result. They run
through evaluator, validated VM, forced baseline, and forced proof, including
conditional construction failure, return transfer, and a semantic process-cell
snapshot.

Eligible groups have zero collector allocation, collector-capable call,
collecting safepoint, GC root map/materialization/writeback, or barrier. Forced
native execution requires nonzero generated entries and zero VM or collector
fallback. `product` has left tracing; `enum` remains registered until every
production instantiation migrates.

## Acceptance

The Current slice has independent HIR and SSA verification of closure,
initialization, whole-value moves, exact active payload, borrow containment, no
mixed edge, no double drop, no branch leak, destination failure cleanup, logical
resource charges, and four-engine conformance for these fixtures.
