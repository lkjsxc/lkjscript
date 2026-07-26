# Edition 2: Execution And Acceptance

[Authority](../edition.md)

## Purpose

Define the cross-engine evidence required before Edition 2 slices or cutover can
become Current.

## Status

**Current for source match, Never/structured control, the four numeric
conversions, the typed-error/generic-prelude cutover, exact atomic migration,
and the full Edition 2 source-corpus cutover.** Differentials cover the
evaluator, reference VM, forced baseline, and forced proof execution with zero
fallback. Ordinary compilation accepts only exact Edition 2 source.

## Engine Contract

Resolved HIR and verified SSA feed the independent SSA evaluator, reference
bytecode/VM, baseline native compiler, and proof-checked optimizing JIT. The VM
and native tiers implement validated ADT construction, tag tests, active-field
access, exact tracing, numeric conversion, logical charges, and structured
terminators. Match itself exists only as a verified plan lowered to SSA CFG.

Forced baseline and forced optimizing modes must enter generated code with zero
fallback. Edition 2 numeric conversion, ADT allocation/access, Never control,
and representative match CFG must reach actual generated calls; emission,
disassembly, metadata, or VM observation alone is not native evidence. The
proof JIT independently checks its complete proof before source effects.
Native heap-call metadata carries the concrete substitution identity and stable
semantic layout identity of every constructed typed-error payload; allocating
an error under a reserved, synthetic, or outer-result identity is forbidden.
Malformed tags, layouts, plans, roots, safepoints, charge sites, control targets,
bytecode, runtime-call identities, and proof metadata fail closed.

## Focused Gates

Each slice requires positive and malformed parser/schema, type, HIR, SSA,
evaluator, bytecode, VM, baseline-native, and optimizing-native tests as
applicable. Required differentials compare exact values, F64 bits, typed errors,
traps/outcomes, logical charges, evaluation order, witness diagnostics, and
resource exhaustion. Recursive and nested ADTs require exact active-variant
trace and forced-collection tests.

## Cutover Gate

The Current cutover satisfies these retained gates:

1. all 125 tracked `.lkjscript` sources, including all 121 under `src/`, are
   atomically migrated;
2. the Current Option/Result generic-ADT replacement and typed-error
   differentials remain green with no obsolete machinery;
3. evaluator, VM, forced baseline, and forced proof-JIT differentials pass;
4. Profile V2 pre-allocation boundaries pass exact-limit, limit-plus-one,
   overflow, adversarial, and deterministic-diagnostic tests; and
5. migration check/diff/publish, rejection, identity, rollback, immutable
   fixture, and no-hidden-mode tests pass.

Docker remains a separate packaging gate. A failed or unrun gate remains
explicit and cannot promote a later capability.
