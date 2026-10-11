# Nominal owned records

An `owned-record` declares an affine type whose representation belongs to its exact
defining module. A public declaration lets consumers name and compose the type;
construction, decomposition and field access remain module-private. The accepted
meaning graph owns the declaration, fields, generic constraints and operations.
[Status](../status.md) distinguishes implementation, acceptance and publication.

## Identity, fields and constraints

```lisp
(owned-record create Fifo (visibility public)
  (type-parameter create T (constraint owned))
  (field create incoming (type (owned-sequence T)))
  (field create outgoing (type (owned-sequence T))))
```

Existing named and applied type forms identify the exact package, declaration and
ordered arguments. An equal layout, declaration name or set of field names never
makes two nominal types interchangeable. There is no structural-to-nominal cast.
Empty and ordinary-data-only owned records are affine too: ownership is a property
of the declaration, independent of its instantiated fields.

Every field must admit an owned value or immutable first-order ordinary data.
Type parameters use the existing Owned constraints, optionally strengthened with
Transferable or Shareable, or ordinary Transferable/CaptureSafeTransferable.
Unconstrained and CaptureSafe-only parameters are excluded. All actual arguments,
including unused and phantom arguments, satisfy their exact formal constraints.
An ordinary formal cannot receive an owned actual merely because that actual is
transferable. Callables, streams, secrets and capability resources cannot be hidden
in stored fields or unused owned-record arguments. An unconstrained ordinary
function or ordinary nominal parameter cannot receive an owned record, including
when that parameter is unused. Existing parameter envelopes retain their original
bound meanings; the new nominal declaration itself requires Graph 32.

Finite nesting composes with structural products, choices and sequences. Direct
and mutual recursive owned-record representations, including recursion through
owned collections, reject. Existing type-depth and bounded-preparation policies
remain authoritative; resource exhaustion is distinct from invalid meaning.

## Representation authority and operations

Representation access requires equality of both defining package and module
identities. Equal names or module IDs in another package grant nothing. A different
module in the same package cannot access fields either. Declaration visibility
controls naming the type; it never makes its fields public. Modules are flat and
there is no implicit descendant-module permission.
Moving, borrowing, passing and automatically cleaning up an opaque owner do not
require authority over its fields.

The existing `pack-owned`, `unpack-owned` and `borrow-owned-field` authoring forms
accept an exact nominal annotation. Native authoring resolves nominal fields to
their stable field references; canonical meaning retains those references rather
than substituting a same-shaped structural product. Ordinary `field` projection
also requires representation authority. Renaming a field does not retarget an
accepted operation to a different field.

Pack evaluates every supplied field exactly once in authored order and constructs
one owner after complete type and custody checks. Every field appears exactly
once. Owned operands must be exact live owning locals. An earlier trap prevents
later effects. Empty construction is allowed.

Unpack consumes an exact live owner, binds all fields simultaneously and gives
each field its declared ownership mode. It provides no partial move. Unused owned
fields receive ordinary automatic cleanup. The consumed parent cannot be reused.

Owned-child reads retain their exact source and every ancestor loan. Ordinary
immutable projections follow existing checked projection and allocation policies.
Source-tied pure results may return a selected owned child through the existing
`borrow-from` contract. No view escapes its admitted source lifetime, becomes an
owner or grants an additional effect. An unrelated owner can still be consumed
inside a read scope.

All representation operations are checked even in unused methods and untaken
syntax. Calling a public provider function remains valid: that function executes
under its own admitted declaration, without transferring field authority to its
caller.

## Composition, transport and execution

Owned records participate in exact owned contracts, generic implementation
applications, structural owned composites and sequences. Substitution preserves
the nominal declaration and every ordered argument. Transferable and Shareable
proofs independently inspect the complete representation and all actual arguments;
neither follows merely from an Owned declaration.
Contract type parameters, including non-Self parameters, and generic implementation
parameters retain their existing Owned bounds. Ordinary Transferable parameters on
an owned record do not widen those contracts or implementation schemes; a concrete
application can still supply its exact owned record type.

Interfaces retain the layout evidence needed for independent admission, including
private representation dependencies. Representation-only records do not become
public lookup or authoring authority. Immutable package bodies and artifacts retain
the complete exact closure. This is language encapsulation, not source encryption.
Consumers can inspect evidence without gaining permission to construct that type.
An opaque provider may use another exact package in its private representation.
Its consumer need not import that package: admission follows the provider's
validated dependency closure for layout evidence, while public naming and calls
still require the consumer's own direct import authority. Conflicting exact
selections for one package reject before acceptance.

Production storage uses one existing owned-storage envelope with the closed
nominal type digest. It does not add a second wrapper around a structural product.
Prepared field layouts are derived metadata, bound to the admitted program.
Consumption and borrowing preserve child custody without cloning owned payloads.
Repacking may create a new parent allocation; it does not imply copied children.

Raw input, adapter values, retained values and codecs cannot manufacture nominal
owners or restore authority to inert tokens. Same-shaped foreign nominal values
reject. Complete raw admission, invocation origins, allocation-bound admission,
active-loan checks and independent compiled/source validation remain mandatory.

Reserve before modeled allocation and check cancellation before detaching children.
Traps, reservation refusal, interrupted adoption and lexical exit release every
owned child and loan through existing joined cleanup. There is no user-defined
destructor, implicit effect rollback or retry grant.

## Maintained adoption and compatibility cut

The standard FIFO exposes `Fifo<T>` instead of its incoming/outgoing product.
Its native two-stack algorithm and public empty, length, push, pop, front and
discard functions retain their behavior. Pop returns the queue in both outcomes;
front returns the original element view tied to the queue. The representation
operations stay in the defining module.

Nominal owned meaning requires Graph 32. Current exports select interface 20,
compiler 32, bytecode 27 and artifact 39. Authored requests select codec 36;
discovery exposes the complete declaration and expression forms. Named/applied
type-object envelopes retain their existing shape. Rebuild experimental derived
assets; this increment adds no predecessor migration layer.

Public acceptance authors a generic supplier before consumers, imports exact
transports, and executes detached artifacts after source deletion. It includes
distinct same-shaped identities, module privacy, empty/data-only ownership,
complete generic admission, nested borrowing, transfer and failure recovery.
A provider representation edit followed by an explicit dependency update must
allow unchanged downstream program text to rebuild. The FIFO's independent
ordering oracle remains a separate maintained consumer.

Matched structural/nominal workloads retain allocation-identity and complete
result observations. Require one custody envelope and no extra collection-prefix
scan attributable to nominality. Modeled reservations, allocator traffic, retained
memory, execution time and API billing are separate measurements.
