# Owned applications with explicit effects and requirements

Public 0.1.73 composes same-invocation Owned generic functions and exact
implementation witnesses with the existing effect and requirement schemes.
Accepted graph meaning owns every operand; a witness, effect row or requirement
argument grants no execution authority. Actual acceptance belongs to
[status](../status.md).

## Application contract

An implementation call supplies its native argument groups in this exact order:
`types`, `effects`, `requirements`, `implementations`, then value arguments.
Each group preserves its supplied operand order. The type, effect and requirement
applications obey the same arity, lexical scope, substitution, finite closure and
caller-allowance rules as an ordinary direct call. A generic relay may forward each
operand from its own exact lexical function. Names do not select equivalent
requirements or implementations.

A graph function may declare Owned parameters, implementation parameters, effect
parameters and requirement parameters together. Ordinary data and callbacks precede
consuming owned parameters. An ordinary callback may have a caller-selected effect
row but cannot capture or carry an owned value. Existing synchronous pure borrows
end before the following task operation. Task kind remains distinct from an empty
effect row.

Owned method contracts remain closed and monomorphic. Their implementations must
match exact callable kind, Self substitution, ordered use modes and effect row.
This extension applies to the enclosing generic graph function; it does not add
method-level generic schemes, implicit witness search or runtime dictionaries.
Resource-bearing functions with requirement parameters retain their existing
restriction. Parallel children retain their existing closed application boundary;
nonempty effect or requirement arguments do not enter through implementation calls.

## Authority and execution

The supplied requirement must satisfy its exact interface and admitted operation
constraint. The substituted target row must fit the caller's declared allowance.
Deployment must separately supply the exact grant before an external operation.
Requirement aliases retain canonical accounting and transaction behavior; selecting
an implementation cannot widen allowances, duplicate grants or reset quotas.

Witness specialization may share an immutable template across different effect
and requirement applications. Each application still retains its own substitutions
and exact activation allowance. Production preparation and the canonical reference
evaluator check these bindings independently. Complete dependency relations retain
all requirement and effect operands, including unused and untaken applications.

Value arguments evaluate in authored order. A consumed owner remains within the
same invocation and is returned or released exactly once. A trap, cancellation or
quota refusal releases remaining owners and loans; it does not undo an earlier
capability operation. No retry or live-effect replay is introduced.

## Compatibility and evidence

Graph 23 adds the application operands. Frozen predecessor owner decoders preserve
supported older meaning with empty argument lists; nonempty operands cannot be
encoded under an older graph. Semantic validator 28 independently rechecks the new
signature combinations. Authored request 27 selects a new application encoding
only for nonempty new operands, retaining predecessor empty-application intent.
Compact change 31 exposes both operands through executable discovery and drafting.
Function-definition projection 13 includes their counts and exact references;
continuations from predecessor projections are rejected.

Compiler 23, bytecode 18 and artifact 30 require rebuilt derived programs. Package
interface 13 already represents function schemes and retains its layout, as does
type-object 10. Installation does not rebuild existing bundles or migrate data.

The [maintained example](../../examples/owned-effects/README.md) checks the generic
library before concrete implementations exist, exports three exact packages and
executes full results after source removal. Independent controlled adapters check
operation order, distinct bindings, quota refusal and cancellation cleanup. Strict
artifact checks reject rehashed operand erasure or rebinding; successful producer
validation is not a loader certificate.
