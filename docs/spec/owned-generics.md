# First-order owned parameters and explicit implementations

Graph 18 extends [direct owned memory](owned-byte-buffers.md) with the closed
`Owned` constraint, the independent `OwnedI64Cell` carrier, and nominal method
contracts selected by explicit static operands. The accepted graph remains the
meaning authority. Native notation, package interfaces, specialization and code
are checked projections of that graph.

`Owned`, `CaptureSafe` and `None` are distinct constraints. An Owned argument must
be exactly ByteBuffer, OwnedI64Cell, an explicit [owned product](owned-products.md)
or [owned choice](owned-choices.md), an [owned sequence](owned-sequences.md)
whose exact element type satisfies Owned,
or an in-scope Owned parameter while checking
a generic body. Ordinary and CaptureSafe parameters cannot receive an owned type,
even when the parameter is unused, its container is empty, or the call is in an
untaken branch. Owned constraints belong to exact graph functions and to the one
Self parameter of an owned contract. Graph functions may compose these parameters
with explicit effect and requirement schemes under
[owned effect applications](owned-effects.md). Named tasks may use them under
[same-task transfer](owned-task-transfers.md).
Nominal records, variants and externals cannot advertise Owned parameters.

Development 0.1.71 adds an orthogonal [transferable obligation](transferable-types.md)
to function type parameters. `owned transferable` retains Owned's affine rules;
ordinary `transferable` parameters admit first-order transferable data. Owned and
CaptureSafe alone do not establish generic cross-task transfer permission.

A generic body is checked symbolically before any concrete application exists.
Instantiation checks the exact substituted type, constraint and parameter modes
again. A direct Owned parameter, result or annotated lexical local has the same
affine rules as its concrete carrier:

- Ordinary parameters precede memory parameters; exact capability resources, when
  present, form a final suffix after memory. Tasks must `consume` memory; only pure
  helpers may declare synchronous read `borrow`.
- Every memory call argument is an exact local. Repeated reads may reborrow;
  any alias involving a consume rejects, in either argument order.
- Consumption transfers the one owner. A moved local cannot be used again.
  Conditional paths join ownership conservatively, including untaken syntax.
- A result may transfer an owner, never a loan. A borrowed parameter cannot be
  consumed, stored, captured, returned or transferred asynchronously.
- Scope exit disposes of remaining local owners. Traps, cancellation and exhausted
  quotas dispose of all remaining owners and loans. This cleanup invokes no
  user-defined method and confers no capability authority.

[Scoped child reads](owned-borrows.md) add borrowed lexical bindings with exact
symbolic `Owned` types. A generic product or choice reader is checked before any
concrete implementation exists. Substitution preserves child/ancestor loan
provenance and exact borrowed method selection. Scope bodies may return unrelated
owners and use authorized effects; protected sources and views cannot be consumed
or escape. Borrowed task parameters remain unsupported.

Owned sequences provide dynamic homogeneous collections under their separate
contract. General nominal owned containers, mutable borrows, escaping captures, memory-bearing indirect
function descriptors, cross-task memory transfer and generic implementation schemes
are outside this increment. Ordinary generic and CaptureSafe contracts retain
their previous meaning. Graph 19 adds fixed structural owned products with complete
consuming decomposition. Ordinary raw-entry structural substitutions remain supported.
Current artifacts retain all lexical annotation roots; an annotation is never an
ownership certificate.

## Independent scalar carrier

OwnedI64Cell has its own canonical type envelope `LKJCEL01` and sealed runtime
token. It stores one fixed-width signed I64; it is not a byte encoding or a
ByteBuffer alias. The closed pure external inventory is:

| Implementation | Parameters, in order | Result |
| --- | --- | --- |
| `core.cell.create` | I64 unrestricted | OwnedI64Cell |
| `core.cell.read` | OwnedI64Cell borrow | I64 |
| `core.cell.replace` | I64 unrestricted, OwnedI64Cell consume | OwnedI64Cell |
| `core.cell.extract` | OwnedI64Cell consume | I64 |
| `core.cell.discard` | OwnedI64Cell consume | Unit |

Creation reserves storage before allocating. Replacement consumes and returns the
same allocation, with no scalar-range conversion. Extraction and discard end its
ownership. Read access requires a scoped loan; consumption rejects while any loan
is live. Origin and carrier type are exact. Cloning token metadata creates an
inert token, never another owner or loan. Shared allocation metadata is not shared
ownership. Raw adapter/runner ingress, ordinary result egress and persistence
cannot manufacture or retain these tokens.

## Nominal contracts and exact static operands

An `OwnedContract` declaration embeds distinct method identities and names, one
Self type-parameter identity with the Owned constraint, and each method's ordered
parameter types/use modes, result, callable kind and exact effect row. A method is
pure or a named task with a closed row of concrete requirement identities. An empty
task row remains a task. Methods have no own generic, effect or requirement scheme.
Ordinary signature types must be closed and first order; this
restriction follows nominal fields and cases as well as structural type children.
A nominal wrapper cannot hide a function, resource, secret, stream or owned value.
Closed recursive, mutual and nested nominal data remain first order. The structural
property is proved under each nominal declaration's ordinary parameter assumptions,
with every actual argument independently checked in its enclosing scope. Even a
phantom argument must be closed and ordinary; recursive binding reuse cannot erase it.
Direct Self parameters form the final affine suffix. Pure methods may borrow or
consume Self; every task Self parameter must consume, including unused parameters.
A result is direct Self or an ordinary closed type. A method need not mention Self,
but the exact Self owner and constraint remain mandatory.

An `OwnedImplementation` declaration names one exact contract and concrete owned
Self, and supplies a complete, unique, canonically ordered method map. Every map
entry selects an exact visible monomorphic graph function with matching ordered
parameter types, modes, result, callable kind and effect row after Self substitution.
The row is equal, not an inferred upper bound: a pure implementation cannot silently
replace an empty-row task, and an implementation cannot add, omit or substitute an
exact requirement. All methods are checked even when unused. Closed externals are
not implementations; ordinary graph wrappers may call them.
Imported contracts, types and methods must belong to the declared exact visible
package closure.

A function may embed named implementation parameters with distinct identities,
an exact contract and an in-scope Owned type parameter as Self. An implementation
call supplies one explicit operand per parameter. An operand either selects an
exact implementation declaration or forwards an exact witness parameter from its
own lexical function. A method call selects a method identity in the witness's
exact nominal contract. There is no implicit implementation search, overload
resolution, capability grant, ordinary runtime dictionary or function value.
A method invocation checks the caller's task kind and declared effect allowance in
addition to its witness and argument types. Deployment supplies a separate exact
grant before an external operation can run. Contract rows participate in relation
extraction, effect/capability summaries, package interfaces and dependency closure;
an otherwise unused public contract retains its required interface metadata.
Multiple implementations for the same Self and signature are valid and remain
distinguishable through package transport, preparation and detached artifacts.

Preparation closes finite applications by exact function and ordered selected
implementation references; existing type/effect closure machinery checks type
applications separately. The resulting Self must equal the actual type binding.
An unbound template is not callable or capturable through ordinary entry. Witness
edges participate in closure and recursion analysis. Tail forwarding is derived
after witness calls have selected their exact graph targets. An active child-read
scope prevents activation replacement, including when it borrows a parameter and
owns no storage. Read-loan lifetime and cleanup obligations remain valid through
other eligible tail transfers and nested calls.
The source-derived interpreter resolves the source witness scope and method maps
independently of these prepared instances.

## Encodings, admission and compatibility

The initial owned-generic increment used graph/owner generation 18. Generation 19
adds structural products and generation 20 adds owned choices. Frozen canonical
owner codecs preserve supported predecessors, and ordinary base type identities
remain unchanged. Task-method effects use the existing canonical method effect
field; they do not introduce another program representation or owner wire layout.
Package interface generation 12 changes method/function layout; interface 10 and
11 have separate frozen representations. In particular, interface-11 function,
constant and component bytes are not interpreted using generation-12 enum tags.

The task-method increment selected compiler unit 18, bytecode 14 and artifact 25. Derived
predecessor artifacts require rebuilding from supported accepted meaning; an old
proof or compiler artifact does not silently become current permission. Authored
pure owned-only requests retain codec 21; product-bearing requests select codec 22
and choice-bearing requests select codec 23. A create/set contract with a task
method selects authored codec 24, distinct change tags and an encoded effect for
each method in that contract. Omitted and explicit pure effects retain identical
predecessor intent bytes. Compact changes use contract 28. Semantic validator 23
invalidates prior acceptance and summary reuse for task-method effects.
Public discovery advertises the actual generations.
Regeneration of maintained artifacts preserves accepted meaning HEADs.
The [child-borrow contract](owned-borrows.md#encodings-and-evidence) owns subsequent
scope-operation, borrowed-binding and derived-layout cuts.

Complete admission includes unused arguments, methods and unreachable expressions.
Canonical source, code and runtime metadata must agree, and independent affine
admission must still reject an invalid graph even if all enclosing digests were
consistently recomputed. There is no inventory-absence shortcut based only on
ByteBuffer. Metadata traversal, witness specialization and recursive closure use
existing finite work/storage admission and cancellation checkpoints; exhaustion
is a resource result, not proof that meaning is invalid. No proof or execution
limit is enlarged by this increment.
