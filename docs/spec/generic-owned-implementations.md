# Explicit generic owned implementations

Status: normative. [Status](../status.md) owns source acceptance and public
availability.

An owned implementation can declare an ordered type scheme and explicit
implementation prerequisites, then apply generic graph functions in its method
map. The exact implementation identity, type arguments and complete prerequisite
tree remain explicit program meaning. This extends
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

Type parameters have distinct semantic identities, the implementation
as their declaring scope, and an Owned constraint optionally strengthened with
Transferable, Shareable or both. Their authored order
determines application order. Self, ordered contract arguments and mapped function
arguments can contain these exact parameters inside supported finite owned
structural types. An empty scheme remains a concrete implementation.

The complete scheme is admitted before any caller or concrete item exists. Every
mapped function has an explicit ordered type application satisfying its declared
constraints. Mapping compares the simultaneously substituted parameter types,
use modes, result type, callable kind, closed effect row and borrowed-result source
position. Pure and named task mappings remain distinct even for empty task rows.
Mapped functions may have implementation parameters, with one exact mapped operand
per parameter in declared order. Effect and requirement parameters remain excluded.
An exported scheme maps exported functions so an importing package can admit
their exact signatures through its selected interface. A private scheme can map
private functions inside its own package.

All mappings and all scheme parameters are checked, including unused methods and
phantom parameters. A closed target or type argument still requires its complete
admitted closure. Equal names, types or method bodies do not merge implementation
identities. Adding a dependency cannot change a selected implementation.
Every implementation actual and mapped function application satisfies its formal's
exact declared bounds under the actual's package-qualified defining scope. An
Owned-only contract remains reusable by a stronger scheme; a method map cannot
erase the stronger requirements of its selected function. Unused callable or
witness metadata acquires no additional transfer or sharing obligation; values
crossing an invocation boundary prove their complete carrier obligation separately.

## Explicit prerequisites and method maps

An implementation prerequisite has its own stable identity and name, one nominal
owned contract, a Self type and every ordered additional contract argument. Its
declaring scope is the implementation, independently of the contract and mapped
function scopes. Its types can mention the implementation's owned type parameters.
Prerequisites do not imply implementation search, effects or deployment grants.

```lisp
(owned-implementation create Maximum (visibility public)
  (type-parameter create T (constraint owned))
  (type-parameter create W (constraint owned))
  (implementation-parameter implparam_8e000000000000000000000000000001
    element owned-worklists::Element T)
  (implementation-parameter implparam_8e000000000000000000000000000002
    reader owned-read-results::IndexRead W (types T))
  (contract Selection) (self W) (types T)
  (method method_8e000000000000000000000000000001
    owned-read-results::select-max (types T W)
    (implementations parameter@Maximum@implparam_8e000000000000000000000000000001
      parameter@Maximum@implparam_8e000000000000000000000000000002)))
```

Here `Selection<Item>::best(Self borrow) -> Item` declares `ReadFrom(0)`. The
mapping reuses the existing generic selector without a specialized storage or
element wrapper. Its two operands forward exact prerequisites in the scheme's
scope to the mapped function's independent parameter order.

A mapped operand is either a direct prerequisite from that same implementation
or a finite concrete application tree with no lexical parameter anywhere inside
it. Concrete trees may contain eligible symbolic type arguments; “concrete”
describes selected implementation identities, not necessarily closed types.
Constructing a concrete application around a scheme prerequisite in a method map
is unsupported. Wrong scope, missing/extra operands, mismatched complete contract
applications and invalid unused mappings reject before publication.

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
type argument per type parameter and one implementation operand per prerequisite:

```lisp
(implementation composable-adapters::Maximum
  (types OwnedI64Cell (owned-sequence OwnedI64Cell))
  (implementations generic-elements::CellKeys
    (implementation composable-adapters::DelegatingReader
      (types OwnedI64Cell (owned-sequence OwnedI64Cell))
      (implementations
        (implementation generic-storage::ReverseReader (types OwnedI64Cell))))))
```

Optional clauses occur at most once in `types`, then `implementations` order.
Omitted and explicitly empty clauses denote empty vectors, without inference.
Bare names and `concrete@` selectors denote applications with no arguments.
`parameter@SCOPE@implparam_HEX` forwards a witness from its exact function or
implementation scope. A function expression may build a nested application from
its own lexical prerequisites, subject to the finite-construction rule below.
Every ordered nested selection participates in application identity. Equal outer
declarations and types cannot merge applications with different prerequisite trees.

Applying a scheme substitutes Self and its contract arguments simultaneously;
method invocation applies the stored mapped-function type vector and maps its
prerequisite operands through that same exact application environment. These
stages retain their separate declaration scopes. A
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

Before exact callable-context exploration, a finite conservative graph connects
declarations and methods. A parameter method call includes potential targets from
every admitted implementation matching its nominal contract. A function expression
that constructs a concrete prerequisite application around a lexical parameter
rejects when its source-to-target construction edge lies in a recursive component
of that graph. The diagnostic is
`kernel_callable_recursive_witness_construction`: unsupported **potential**
recursive prerequisite witness construction. An unused same-contract alternative
can therefore make an otherwise acyclic selected application unsupported. This
does not claim actual witness growth. Plain forwarding, permutations, projections,
fully concrete resets and symbolic construction outside those potential recursive
components retain their admitted applications. Exact type-growth admission remains
independent. Canonical witness trees obey explicit depth and node-count bounds;
out-of-bound stored meaning is semantically inadmissible, and malformed encoded
trees reject during loading. Public authored-input expansion, derived analysis
and preparation have separate resource admissions; exhausting those capacities
does not establish semantic invalidity.

Source analysis currently retains separate type-provenance slots for every typed
prerequisite path, including paths through shared witness nodes. A large acyclic
application can therefore exhaust proof capacity while its retained witness DAG
would remain small. Compact storage does not establish cheap source admission.

Prepared applications retain their complete witness environments. Compatible
immutable code may share after each application has been materialized; admission
still charges those construction and clone costs. Such sharing does not establish
cheaper preparation or a runtime speedup.

Canonical meaning, compact/native authoring, drafts, public inspection, package
interfaces, transport, compiler metadata and detached artifacts retain every
scheme parameter, prerequisite contract, mapped operand and nested application.
The canonical validator, independent
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

The [composition witness](../../examples/composable-owned-implementations/README.md)
exports `Maximum`, `DelegatingReader` and a consuming task adapter before concrete
items exist. It maps the existing selector directly, nests readers over distinct
flat/reverse leaves, preserves borrowed provenance through forwarding, then drains
and reuses the same owner. Different observers and first ties distinguish complete
prerequisite selections. Joined children exercise exact consuming task applications.
This remains a language witness; it makes no compiler self-hosting or performance
claim.

Associated projections, method-local generics, implementation search, general
lifetime inference and representation-dependent composite views remain future
work. [Verification](verification.md#generic-owned-implementation-obligations)
owns this increment's acceptance obligations.
