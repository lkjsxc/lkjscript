# Edition 2: HIR And SSA

[Authority](../edition.md)

## Purpose

Define one resolved typed IR path for Edition 2 ADTs, match, Never, numeric
conversions, runtime facts, and logical charges.

## Status

**Current for resolved enum metadata, construction/test/projection HIR and SSA,
the independently verified source-match plan and ordinary-CFG lowering,
Never/structured control, the SSA evaluator, validated bytecode, reference VM,
and forced baseline/proof JIT.** Wider Edition 2 representation plans remain
Accepted Targets.

## Resolved HIR

HIR currently resolves every enum, variant, and field declaration to stable
canonical IDs and retains exact invariant substitutions in declared field and
function types. The Current execution slice adds typed enum construction/projection/test
primitives,
a verified match-plan input, the exact control terminators in
[Never and control](../../../decisions/semantics/never-and-control.md), and distinct nodes for all four
numeric conversions. Semantic effects, runtime facts, layout facts, source
origins, and logical charge sites are separate fields.

The type checker rejects ownership-bearing ADT fields, mixed numeric operations,
invalid recursive shape, nonexhaustive or useless match arms, invalid Never
materialization, and mismatched control targets before lowering.

## Verified SSA

Tag tests and active-field projection are Current for verified hand-built and
source-lowered SSA. Match lowers to SSA CFG only after independent match-plan
verification. SSA has
no Match instruction. It uses exact discriminant tests, active-variant field
projections, block parameters for immutable pattern bindings and joins, and the
control terminators specified by this authority. Numeric conversions are typed
SSA operations or exact versioned runtime calls, never host-language casts.
Logical construction charges remain explicit through optimization.

Verification checks ID/type/substitution identity, active-variant dominance,
field initialization, plan provenance, CFG edges, block arguments, roots,
safepoints, barriers, layout identities, charge placement, effects, and
terminators. Malformed, stale, mismatched, unknown, or over-budget metadata
fails closed.

## One IR Family

The evaluator, bytecode generator, VM, native lowerers, and proof optimizer all
consume this verified family. They may implement ADT primitives and exact
terminators; none may reinterpret source syntax, duplicate match usefulness, or
invent enum tags. Proof optimization must preserve source outcomes, logical
charges, exact roots, and active-variant tracing.
