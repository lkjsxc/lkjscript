# Residual Generic Compare Vertical

## Status
<!-- LKJ-F residual-generic-compare-vertical current eqYzVUhKfvvbx9cABxrV6TJ_gXiHswa5uYwrPeRIeFA -->
**Current at platform revision 18 for the bounded direct-call vertical below.**
The complete polymorphic value plane remains Experimental under its Accepted
Target.

## Contract
A direct generic function may apply one `equal-value` operation to two values
of the same naked type parameter. Resolved HIR records the minimal ordered
hidden requirement `[transport, compare]`. A compare-only body does not receive
`independent-owner` or `dispose` authority.

The producer derives that demand from resolved typed HIR. The independent HIR
verifier traverses the typed body separately and rejects missing, extra,
reordered, or mismatched requirements. Type-parameter equality is legal only
through this hidden witness path; it does not grant aggregate equality to a
concrete type that lacks it.

## Executable Route
Verified SSA uses `MemoryWitnessCompare` with the exact type parameter and two
operands. Its identity, effects, ownership use, optimization reconstruction,
and malformed-input checks are exhaustive. Validated bytecode opcode 82 binds
one frame-local witness slot and rejects an absent compare requirement, stale
slot, unsupported operation, or mismatched call binding.

The evaluator and VM authenticate the installed witness before value equality.
Baseline and proof lowering pass the exact representation-specific witness
locator and two opaque structural keys to the structural island. The island
resolves compare authority and storage, validates both owners, and invokes the
existing iterative semantic owner equality service. Source spelling,
specialization, and fallback grant no authority.

## Evidence
`crates/lkjscript-app/tests/fixtures/residual-compare.lkjscript` derives exactly
`[transport, compare]`, executes equal and unequal selected structural strings,
and returns `41` in evaluator, VM, forced baseline, and forced proof. Both
native tiers require nonzero native and direct-call entry counts and zero VM
fallback. Mutated SSA requirement and witness records reject.

The same cut repairs two generic ABI defects: native result locals now derive
opaque-key form from the callee's generic result rather than the mere presence
of hidden witnesses, and dynamic call-result cleanup resolves the exact
representation-specific witness slot.

## Explicit Exclusions
- no residual encode, decode, list-store, or list-load operation;
- no repeated residual compare body or mixed compare/ownership body claim;
- no capture or indirect generic callable ABI;
- no persistent structural-owner list or structural-list process import;
- no source `sealed` type and no complete-plane promotion.
