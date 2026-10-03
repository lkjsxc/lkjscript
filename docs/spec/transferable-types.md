# Explicit transferable type parameters

Status: normative for development 0.1.71. [Status](../status.md) owns source and
distribution acceptance. [Structured parallel tasks](structured-parallel.md) own
evaluation order, custody, quotas and joined completion.

## Constraints and scope

`Transferable` is a structural type obligation, independent of affine ownership
and capture safety. Native function type parameters may declare
`(constraint transferable)`, `(constraint capture-safe transferable)` or
`(constraint owned transferable)`. JSON requests use the corresponding arrays
`["transferable"]`, `["capture-safe", "transferable"]` and
`["owned", "transferable"]`. Compact flat records quote combined values, such as
`constraint="owned transferable"`. Canonical order puts the existing constraint first.
Duplicate or unknown entries, combinations with `none`, and a combination of
`owned` with `capture-safe` reject.

Without `owned`, a transferable parameter admits only ordinary first-order data.
With `owned`, it retains affine consumption and borrowing rules and additionally
permits crossing an admitted task boundary. An open `None`, `CaptureSafe` or
`Owned` parameter alone proves no transferability. Ordinary transferable data also
proves capture safety; owned transferable data remains noncapturable. The existing
capture-safe contract permits some callable types and therefore does not imply
transferability. Concrete owned carriers still prove transfer structurally without
requiring an authored type-parameter declaration.

The new bounds belong only to graph-function type parameters, with exact signature
membership and existing restrictions on owned generic effects and requirements.
Nominal declarations and owned-contract Self retain their prior constraint forms.
A function's owned transferable parameter can satisfy an owned contract's Self
requirement without granting effects or changing the contract identity.

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

A generic task may form a parallel group using its own transferable type parameters
and exact implementation parameters. Each child remains a direct named graph task
with a closed empty effect row. An implementation operand names either an exact
implementation or a parameter of the exact lexical function. Its nominal contract
and substituted Self must agree. Equal Self types do not merge implementations.

Ordinary child parameters precede consuming owned parameters. Loans cannot cross
the boundary. An ordinary/ordinary result is a structural record; any owned child
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
Public raw values cannot manufacture transfer authority. Existing allocation
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

This extension supplies no task handles, detached tasks, channels, asynchronous
borrowing, capability inheritance or user-defined transfer certificates. Region
policies and scheduling remain separate contracts.
