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

Effect and implementation applications are ordered inline fields in current
owners, interfaces, requests and compiler metadata. These records, bytecode and
artifacts share schema epoch 1. Empty applications use the same record shape as
nonempty applications. Discovery, drafts and definitions expose complete operand
counts and exact references.

Independent admission checks every signature combination; strict loading compares
canonical applications with executable operands. Foreign epochs reject, and
maintained bundles are rebuilt through supported product owners. Installation
does not authorize data migration or execution.

The [maintained example](../../examples/owned-effects/README.md) checks the generic
library before concrete implementations exist, exports three exact packages and
executes full results after source removal. Independent controlled adapters check
operation order, distinct bindings, quota refusal and cancellation cleanup. Strict
artifact checks reject rehashed operand erasure or rebinding; successful producer
validation is not a loader certificate.
