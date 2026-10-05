# Explicit generic owned implementations

Status: normative. [Status](../status.md) owns source acceptance and public
availability.

An owned implementation can declare an ordered type scheme and apply generic
graph functions in its method map. The exact implementation identity and its
ordered application arguments remain explicit program meaning. This extends
[parameterized owned contracts](owned-contract-parameters.md) without introducing
implementation search or a new runtime ownership mode.

## Schemes and mapped functions

```lisp
(owned-implementation create Flat (visibility public)
  (type-parameter create T (constraint owned))
  (contract owned-worklists::Worklist) (self (owned-sequence T)) (types T)
  (method method_88000000000000000000000000000011 std::sequence-empty (types T))
  (method method_88000000000000000000000000000012 std::sequence-length (types T))
  (method method_88000000000000000000000000000013 std::sequence-push (types T))
  (method method_88000000000000000000000000000014 std::sequence-pop (types T)))
```

Implementation parameters have distinct semantic identities, the implementation
as their declaring scope, and the exact Owned constraint. Their authored order
determines application order. Self, ordered contract arguments and mapped function
arguments can contain these exact parameters inside supported finite owned
structural types. An empty scheme remains a concrete implementation.

The complete scheme is admitted before any caller or concrete item exists. Every
mapped function has an explicit ordered type application satisfying its declared
constraints. Mapping compares the simultaneously substituted parameter types,
use modes, result type, callable kind, closed effect row and borrowed-result source
position. Pure and named task mappings remain distinct even for empty task rows.
Mapped functions have no effect, requirement or implementation parameters in this
increment; implementation schemes have no implementation prerequisites.
An exported scheme maps exported functions so an importing package can admit
their exact signatures through its selected interface. A private scheme can map
private functions inside its own package.

All mappings and all scheme parameters are checked, including unused methods and
phantom parameters. A closed target or type argument still requires its complete
admitted closure. Equal names, types or method bodies do not merge implementation
identities. Adding a dependency cannot change a selected implementation.

## Explicit applications and forwarding

```lisp
(implementation-call owned-worklists::build
  (types OwnedI64Cell (owned-sequence OwnedI64Cell))
  (implementations generic-elements::Cells
    (implementation generic-storage::Flat (types OwnedI64Cell)))
  (local inputs))

(method-call (implementation generic-storage::FlatReader (types T))
  owned-read-results::IndexRead method_8a000000000000000000000000000001
  (local storage))
```

The `implementation` operand names one exact visible declaration and supplies one
type argument per scheme parameter. Bare names and `concrete@` selectors denote
zero-argument applications. Existing `parameter@FUNCTION@implparam_HEX` operands
forward their exact lexical function witness unchanged.

Applying a scheme substitutes Self and its contract arguments simultaneously;
method invocation then applies the stored mapped-function type vector under that
same substitution. These stages retain their separate declaration scopes. A
function may apply a scheme to its own eligible symbolic parameters, as well as
closed owned types. It does not gain stronger bounds or execution grants from
the selection. Wrong arity, scope, constraints or complete contract application
reject before publication or execution.

Source-tied results retain their exact input provenance across both substitutions.
The lexical `borrow-call` scope and existing sealed result packet own cleanup;
generic implementation applications create no escaping view or mutable alias.
Task applications preserve authored argument order, affine consumption, exact
effect allowances and deployment authority. Structured children retain their
existing transferable bounds and joined cleanup.

## Finite preparation and independent admission

Complete callable-flow analysis follows direct calls, forwarded witnesses and
mapped scheme targets across exact package interfaces. It includes untaken syntax
and uninvoked methods. A cycle that structurally grows its type application cannot
silently create infinitely many specialized functions. Reject such an expanding
cycle as semantic invalidity before publication. Ordinary recursion, parameter
permutation and closed type resets retain their admitted finite applications.
Bounded analysis exhaustion remains a resource failure, separate from a discovered
expansion or invalid signature.

Canonical meaning, compact/native authoring, drafts, public inspection, package
interfaces, transport, compiler metadata and detached artifacts retain every
scheme parameter and application vector. The canonical validator, independent
ownership oracle, reference reader and strict loader admit these relationships
independently. Recomputed hashes cannot make omitted bounds, substituted witnesses,
wrong mapped types or forged borrowed-result relationships valid.

The new semantic and derived encodings advance at their maintained owners.
Unsupported experimental predecessors reject clearly; maintained bundles are
rebuilt through ordinary public check/build. No compatibility migration is
required. Original historical and immutable publication bytes retain their own
meaning and evidence.

## Reusable storage witness

The [native example](../../examples/generic-owned-implementations/README.md) exports
Worklist and IndexRead algorithms with flat/chunk32 implementation schemes before
the downstream item package exists. Independent cells, one-octet buffers and a
nested owned product then use those same declarations without item-specific
storage functions. The product's observer and consuming finish include its two
ordinary metadata fields, producing an independent observable result.

The consumer selects and observes source-tied views, drains the original owner,
reuses empty storage and transfers items between representations. A reverse-index
scheme for the same Self and contract exposes exact identity through stable first
ties. A generic task scheme and transferable parallel wrapper complete owned
cleanup in joined children. Public tests retain literal inputs, canonical edits,
independent complete results and detached execution after source removal.

Associated projections, method-local generics, implementation search, general
lifetime inference and representation-dependent composite views remain future
work. [Verification](verification.md#generic-owned-implementation-obligations)
owns this increment's acceptance obligations.
