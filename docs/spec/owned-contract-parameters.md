# Parameterized owned contracts and structural method signatures

Graph 26 extends [explicit owned implementations](owned-generics.md) with ordered
contract arguments and structural owned method types. The accepted meaning graph
remains authoritative. [Status](../status.md) owns exact source acceptance and
public executable availability; this specification does not confer either.

## Contract scope and signatures

An `OwnedContract` retains one distinguished `Self` parameter and declares a
bounded ordered list of additional type parameters. Every parameter has the exact
`Owned` constraint, a distinct identity and the contract as its declaring scope.
The additional list excludes Self. Names do not determine application order.

Native declarations select Self with `(self NAME)` and declare parameters as
children in authored order. Additional argument order is that child order with
Self removed. A minimal worklist contract is:

```lisp
(owned-contract create Worklist (visibility public)
  (self Self)
  (type-parameter create Self (constraint owned))
  (type-parameter create Item (constraint owned))
  (method method_77000000000000000000000000000001 empty
    (parameters) (returns Self))
  (method method_77000000000000000000000000000002 length
    (parameters (Self borrow)) (returns I64))
  (method method_77000000000000000000000000000003 push
    (parameters (Item consume) (Self consume)) (returns Self))
  (method method_77000000000000000000000000000004 pop
    (parameters (Self consume))
    (returns (owned-choice
      (case empty Self)
      (case item (owned-product (field rest Self) (field value Item)))))))
```

Method parameters and results may contain any of these exact contract parameters
inside finite [owned products](owned-products.md), [choices](owned-choices.md) and
[sequences](owned-sequences.md). Closed owned carriers and closed owned structural
types are also permitted. Ordinary signature types remain closed first-order data;
an ordinary nominal or container cannot hide an owned parameter or owner. Every
type child and actual nominal argument is checked, including unused arguments,
phantom arguments and unselected cases.

Ordinary unrestricted parameters precede the owned suffix. Pure methods may
borrow or consume owned parameters; named task methods must consume every owned
parameter. A result is owned or closed ordinary data, never a loan. Methods retain
their exact callable kind and closed effect row and have no method-local generic,
effect or requirement scheme. An empty-row task remains a task.

## Exact implementations and function witnesses

An `OwnedImplementation` names an exact contract, a concrete closed owned Self and
one explicitly ordered owned type argument per additional contract parameter:

```lisp
(owned-implementation create FlatCells (visibility public)
  (contract worklists::Worklist)
  (self (owned-sequence OwnedI64Cell))
  (types OwnedI64Cell)
  (method method_77000000000000000000000000000001 flat-empty)
  (method method_77000000000000000000000000000002 flat-length)
  (method method_77000000000000000000000000000003 flat-push)
  (method method_77000000000000000000000000000004 flat-pop))
```

The complete method map resolves each original method signature with one
simultaneous substitution for Self and every additional parameter, then requires
exact parameter order, type, use mode, result, callable kind and effect row.
The baseline mappings shown above have no function type arguments and select
visible monomorphic graph functions. Generic targets require an explicit mapped
type application under the extension below. All mappings are checked even when
unused. No inferred implementation search or subtyping participates in selection.

[Generic implementation schemes](generic-owned-implementations.md) extend this
form with implementation-scoped Owned parameters and explicit mapped-function
type arguments. They retain complete symbolic admission and exact selection.

A function witness retains its in-scope Owned Self parameter and declares an
ordered argument vector. Arguments may contain exact in-scope Owned parameters
inside supported finite owned structural types:

```lisp
(implementation-parameter implparam_77000000000000000000000000000001
  work Worklist W (types T))
```

Here the enclosing function declares `W: Owned` and `T: Owned`. An implementation
call still supplies one explicit concrete or forwarded witness operand per function
witness. Matching compares the nominal contract, substituted Self and every ordered
argument. Equal method names or equal Self alone do not establish a match. Arity,
order and constraints remain mandatory even when no method mentions an argument.
Contracts without additional parameters use an empty argument vector; the native
`types` child may be omitted in that case.

Forwarding binds the exact witness in its lexical function scope. It grants no
effects or deployment authority. Imported contracts, argument types and targets
must belong to the exact visible dependency closure. Preparation, recursion
analysis, tail forwarding and the source interpreter retain complete applications.

## Structural substitution and independent admission

Substitution is structural and simultaneous. Rebuild product fields, choice cases
and sequence elements from the original signature, preserving names and exact
types. A substituted type that happens to contain another substitution key is
not substituted a second time. Materialized types remain available wherever their
digests are referenced; deriving a digest without its type object is insufficient.

Validate the complete structural application under existing type-depth, child,
proof-work and storage bounds. The production resolver, source-reference reader
and independent ownership oracle each validate application meaning. Transport or
artifact digests cannot replace that admission, even when a malformed producer
consistently recomputes them.

Affine consumption, synchronous borrowing, child/ancestor loan provenance,
authored evaluation order and cleanup retain their existing contracts. Both a
worklist's pop outcomes return its storage owner explicitly. The `item` outcome
also returns the element owner; neither a second owner nor an escaping read view
is created.

## Encodings and boundaries

This increment selects graph/owner 26, package interface 14, compiler unit 26,
bytecode 21 and artifact 33. Authored request 30 applies when additional contract
parameters or application arguments are nonempty; empty applications retain their
predecessor intent bytes and historical commitments. New parameterized-contract
intent tags encode every method effect explicitly. Contracts, implementations and
function witnesses carry their full ordered application data through canonical
drafts, inspection, interfaces, transports, preparation and detached artifacts.
The derived format cut requires rebuilding maintained bundles through their
supported product owners. Historical acceptance and immutable published bytes
retain their original meaning and evidence.

Associated projections, general escaping references,
mutable borrows and method-local generics remain future work.
[Source-tied borrowed results](owned-read-results.md) add an explicit pure-method
read relationship by exact parameter position. Explicit contract
parameters express the worklist capability without selecting a lifetime or region
mechanism. The [native guide](../guides/native-owned-worklists.md) uses independent
storage representations and provisional graph analysis to exercise composition.
