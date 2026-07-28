# Tracing Family Ratchet

## Status

**Accepted contract; this is an intermediate migration gate, not the final
no-tracing-collector gate.**

## Rule

`LKJ-MEMORY-TRACING-RATCHET` owns one sorted closed set of object families that
may select `legacy-traced`. It derives the implementation, inventory, Current
State count, and island exclusions from one registry.

The gate fails when:

- a new family becomes traced;
- a migrated family regresses;
- an unregistered family selects legacy tracing;
- a migrated island type allocates, roots, barriers, or collects;
- source, documentation, and implementation counts differ.

The set may decrease in an ordinary accepted migration. It may increase only
through an explicit accepted architectural reversal with retained evidence and
a changed ratchet contract. Analysis failure is never such a reversal.

## Initial Migration

The first island removes scalar boxes, transitional buffers, byte values, and
paths from the allowed set. Structural values remain exact registered legacy
families. Collector infrastructure remains available only to those families.

## Final Gate

`LKJ-RUNTIME-NO-TRACING-COLLECTOR` remains disabled until the registry is empty
and collector code, roots, barriers, stack maps, polls, metrics, and tests are
removed. Passing the intermediate ratchet cannot support a whole-runtime
collector-free claim.
