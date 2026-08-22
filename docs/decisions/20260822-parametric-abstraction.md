# Rank-1 parametric abstraction

Date: 2026-08-22 UTC.

## Status

Accepted and implemented as a narrow meaning-graph-4 vertical slice: explicit rank-1 type
parameters on pure and closed external functions, named noncapturing pure function values, and
`invoke`. The slice is intentionally smaller than the earlier generic collection proposal.

## Implemented semantics

Every type parameter has a declaration-owned stable `typeparam_` identity and mutable local name.
A pure function or closed external function owns an ordered parameter list. Parameter/result and
supported structural types may refer to those parameters. Accepted direct calls and named function
values supply one explicit ordered type argument per parameter; validation resolves and
substitutes them recursively and deterministically.

A `function_ref` denotes one named pure function with all generic arguments applied. It carries
stable semantic provenance, not a runtime address. `invoke` evaluates that function value once,
then evaluates arguments left-to-right and calls it. Function values have no equality, ordering,
serialization, partial application, lexical environment, or captured values.

Generic task functions reject. Recursive generic calls may pass the enclosing function's type
parameters in the same order; polymorphic recursion that changes type arguments rejects. Accepted
meaning contains declarations and explicit applications, not compiler instances. Bytecode and the
semantic reference interpreter perform their own substitution/execution routes and must agree.

## Maintained consumers

- The standard graph owns `core.identity<T>(value: T) -> T` and one additional graph test, bringing
  the maintained standard package to 7 tests.
- `lkjournal::service::json-response` calls `core.identity` with explicit `Text`; its exact
  two-package closure passes 12 tests across both execution tiers.
- The binary-only `command` template calls the same built-in generic with explicit `Text` and is
  created, inspected, changed, checked, built, run, backed up, and restored through the copied
  executable acceptance workflow.

These consumers establish the implemented core mechanism. They do not establish a generic
collection library, constraints, inference, or an incremental instantiation cache.

## Authority and compilation

The accepted graph owns type-parameter identities, declaration order, explicit type arguments,
function references, and invocation. Type inference, monomorphic prepared indexes, compiler dense
IDs, and specialization are derived and may not change meaning or deterministic artifact output.
Current compilation reconstructs the exact package closure; there is no persistent incremental
compiler-unit or generic-instantiation cache.

Type abstraction grants no capability. Pure function values cannot hide task effects, deployment
grants, secrets, live handles, or ambient state. Arity mismatch, foreign/out-of-scope type
parameters, invalid recursion, generic task functions, malformed identities, and resource
exhaustion reject before execution.

## Deferred mechanisms

The standard package does not yet provide generic list length/get/append/map declarations, and
`lkjournal` has not removed collection-specific wrappers through such a library. Generic records
and variants, constraints or trait-like dictionaries, type-argument inference, higher-rank values,
lexical lambdas, closure capture, partial application, generic task functions, and dynamic type
reflection remain unimplemented.

Add constraints or generic data only when multiple maintained consumers require the same exact
static abstraction. Reconsider closure capture only when named functions and explicit parameters
cannot express at least two complete consumers and the proposal covers capture ownership,
lifetime, effects, identity, recursion, compilation, and both execution tiers. No extension may
create hidden source generation, widen capability authority, introduce textual macros, or add TLS
surface.
