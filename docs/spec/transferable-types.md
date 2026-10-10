# Explicit transfer and sharing obligations

[Status](../status.md) owns source and distribution acceptance.
[Structured parallel tasks](structured-parallel.md) own
evaluation order, custody, quotas and joined completion.

## Constraints and scope

`Transferable` proves safe custody transfer to a fresh invocation. `Shareable`
proves safe scoped read access from joined child invocations. These structural
obligations are independent of one another, affine ownership and capture safety.
Native graph-function type parameters may declare
`(constraint transferable)`, `(constraint capture-safe transferable)` or
`(constraint owned transferable)`. Owned parameters may additionally declare
`(constraint owned shareable)` or `(constraint owned transferable shareable)`.
JSON requests use the corresponding arrays
`["transferable"]`, `["capture-safe", "transferable"]` and
`["owned", "transferable"]`, `["owned", "shareable"]` and
`["owned", "transferable", "shareable"]`. Compact flat records quote combined values,
such as `constraint="owned transferable shareable"`. Canonical order puts Owned
or CaptureSafe first, then Transferable, then Shareable.
Duplicate or unknown entries, combinations with `none`, and a combination of
`owned` with `capture-safe` reject. Shareable requires Owned; ordinary Shareable
parameter forms are not admitted.

Without `owned`, a transferable parameter admits only ordinary first-order data.
With `owned`, it retains affine consumption and borrowing rules and additionally
permits crossing an admitted task boundary. An open `None`, `CaptureSafe` or
`Owned` parameter alone proves no transferability. Ordinary transferable data also
proves capture safety; owned transferable data remains noncapturable. The existing
capture-safe contract permits some callable types and therefore does not imply
transferability. Concrete owned carriers still prove transfer structurally without
requiring an authored type-parameter declaration.

An owned Shareable parameter retains all affine rules while allowing read loans
to span a joined group. It does not authorize moving the owner to a child. An
owned Transferable parameter does not establish Shareable, even when all current
concrete carriers satisfy both predicates. Borrowing during an ordinary synchronous
task call needs only Owned. Neither obligation changes callable kind, effects or
capability grants.

The bounds belong to graph-function type parameters and the supported Owned
parameters of generic implementation schemes, with exact declaration membership
and existing restrictions on owned generic effects and requirements.
Nominal declarations and owned-contract Self retain their prior constraint forms.
A stronger owned parameter can satisfy an owned contract's Self or additional
Owned requirement without granting effects or changing the contract identity.
Method maps must still prove every stronger obligation required by their selected
generic function applications under the exact implementation scope.

## Complete bounded proof

Check generic bodies before any concrete application exists. Every call checks its
actual type obligations, including unused parameters and untaken syntax. A valid
concrete caller cannot rescue an invalid generic template.

Ordinary scalars and containers require the complete ordinary transferable type
closure. ByteBuffer and OwnedI64Cell are concrete transferable owners. Owned
products and choices must satisfy their existing shape rules and prove every
child transferable, including inactive cases. Ordinary aggregate metadata may
contain exact in-scope ordinary transferable parameters. At least one statically
owned field or case remains required, independent of runtime selection.

[Owned sequences](owned-sequences.md) are transferable when their exact element
type is transferable, including when empty. Every actual child crosses independent
source and destination admission. Interrupted adoption retains custody of the whole
sequence until cleanup; a partially transferred sequence cannot escape.

Shareable admission has a distinct structural traversal. ByteBuffer and
OwnedI64Cell support immutable scoped reads. Owned products and choices require
every owned child to be Shareable and every ordinary member to be complete
first-order data, including inactive cases and phantom nominal arguments.
Owned sequences require either a Shareable owned element type or complete ordinary
first-order elements, including when empty. An ordinary element parameter proves
the latter under its exact in-scope ordinary Transferable assumption. An open owned
element requires an exact in-scope Shareable assumption; an owned Transferable
assumption cannot substitute for it. Sharing preserves the source custodian and
allocation origin throughout the complete join.

Every nominal actual argument is checked in its caller scope, including phantom
arguments. Nominal fields and cases are checked under a separate set of ordinary
formal assumptions belonging to that declaration. Functions, task functions,
secrets, streams, capability resources and unresolved foreign parameters cannot
hide behind a nominal wrapper. Ordinary containers cannot contain owners.

The proof follows finite declaration/type edges with context-sensitive traversal
state. A visited recursive node prevents repeated expansion; it is not a completed
positive certificate for another proof. An unsafe member in a mutually recursive
component rejects the whole relevant closure regardless of traversal order or
selected value. Exact owner membership is checked before establishing assumptions.
Traversal and retained proof storage use existing finite admission and cancellation
controls. Exhaustion is a resource result, not evidence of invalid meaning.

## Generic structured groups

A generic task may form a parallel group using its own transfer or sharing obligations
and exact implementation parameters. Each child remains a direct named graph task
with a closed empty effect row. An implementation operand names either an exact
implementation or a parameter of the exact lexical function. Its nominal contract
and substituted Self must agree. Equal Self types do not merge implementations.

Ordinary child parameters precede owned parameters. Consuming inputs prove
Transferable; borrowed inputs prove Shareable. Both call footprints remain active
through preparation and join. Same-root read/read is valid; a read/consume alias,
including an ancestor alias, rejects in either order. Borrowed task results remain
unsupported. An ordinary/ordinary result is a structural record; any owned child
result selects an owned product, including both mixed orientations. This ownership
class is known while checking the generic body.

Preparation resolves each child application and retains every instantiated pair
type, including intermediate pairs absent from function signatures. Execution
resolves child type operands and the pair type under the active caller bindings
before sealing transfer. Sealed custody binds exact prepared application, ordered
types, implementation selection, source and destination invocation identities.
The independent reference evaluator reconstructs those facts from canonical meaning.
Type resolution uses bounded comparisons of admitted identities without constructing
temporary type objects. Raw generic admission reserves any required fallback
substitution storage before cloning names, vectors or canonical encoding buffers.
Scoped read envelopes independently bind exact child application and source/destination
invocation identities while retaining owner custody at the source. Nested groups
delegate only scoped read permission and join before releasing ancestor guards.
Public raw values cannot manufacture transfer or sharing authority. Existing allocation
identity, argument order, aggregate quota and joined cleanup contracts apply.

## Encoding and compatibility

Constraint tags 0 (`None`), 1 (`CaptureSafe`) and 2 (`Owned`) retain their bytes.
Tags 3, 4 and 5 encode transferable, capture-safe transferable and owned transferable
respectively. Transfer-bearing owners require Graph 22 and requests select authored
codec 26. Old envelopes cannot acquire new authority by recomputing their digests.

Package interface 13 carries the expanded constraint inventory while supported
predecessor readers retain their original permission. Compiler 22 and artifact 29
renew derived admission; predecessor units require rebuilding from accepted meaning.
Bytecode 17 retains its instruction layout: existing child type operands and static
implementation operands can represent the symbolic applications. Validator 27,
function projection 12 and compact discovery 30 advertise the changed contracts.
CLI observations 35 and existing type-object envelopes retain their forms.

The encoding numbers above record the historical Transferable increment. New
sharing obligations and borrowed task signatures require their advertised current
owner, request and derived-admission generations. Recomputed hashes cannot turn
an older constraint tag or task signature into current read permission. Exact
generations belong to product discovery and the maintained encoding owners.

This extension supplies no task handles, detached tasks, channels, escaping
borrowing, capability inheritance or user-defined transfer/read certificates. Region
policies and scheduling remain separate contracts.
