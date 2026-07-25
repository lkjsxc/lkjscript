# Edition 2: Value, Effects, And Metering

[Authority](../edition-2.md)

## Purpose

Separate source-semantic effects, value semantics, runtime facts, and logical
resource charges.

## Status

**Accepted Target, not Current.** Current effect summaries and physical runtime
metrics are not reclassified by this contract.

## Semantic Effects

Semantic effects describe source-observable behavior and are separate from
runtime facts. Constructing an ordinary product or enum is pure value
construction apart from the effects of evaluating its child expressions in
specified order. Ordinary products and enums have no identity; placement,
boxing, reuse, or allocation elimination cannot be observed through equality or
control flow.

## Runtime Facts

Resolved HIR/SSA separately records `may_allocate`, `may_collect`, safepoint,
exact roots, read/write barriers, runtime-call requirements, and placement
eligibility. These are verified lowering and execution facts, not semantic
effects. Optimization may alter physical facts only while preserving semantic
behavior, exact tracing, traps, outcomes, and required metering.

## Logical Construction Metering

Each semantically evaluated aggregate construction performs a deterministic
logical construction charge after child evaluation and before the value is
available. Deterministic profiles retain that charge even when scalar
replacement, constant folding, placement, reuse, or allocation elimination
removes physical allocation. Exhaustion occurs at that semantic point and
publishes no partial value.

Normal modes expose labeled physical allocation, collection, memory, and
runtime-call metrics. Physical or estimated metrics are never reported as
semantic construction counts. Logical and physical categories have distinct
identities and cannot share a counter merely because an initial boxed
implementation makes their values coincide.
