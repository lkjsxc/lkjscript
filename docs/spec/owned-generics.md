# First-order owned parameters and explicit implementations

Graph 18 extends [direct owned memory](owned-byte-buffers.md) with the closed
`Owned` constraint, the independent `OwnedI64Cell` carrier, and nominal method
contracts selected by explicit static operands. The accepted graph remains the
meaning authority. Native notation, package interfaces, specialization and code
are checked projections of that graph.

`Owned`, `CaptureSafe` and `None` are distinct constraints. An Owned argument must
be exactly ByteBuffer, OwnedI64Cell, an explicit [owned product](owned-products.md),
or an in-scope Owned parameter while checking
a generic body. Ordinary and CaptureSafe parameters cannot receive an owned type,
even when the parameter is unused, its container is empty, or the call is in an
untaken branch. Owned constraints belong to exact first-order graph functions
without effect or requirement parameters, and to the one Self parameter of an
owned contract. Named tasks may use them under [same-task transfer](owned-task-transfers.md).
Nominal records, variants and externals cannot advertise Owned parameters.

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

General owned containers, mutable borrows, escaping captures, memory-bearing indirect
function descriptors, task memory signatures and generic implementation schemes
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
parameter types/use modes, result and pure callable kind. Methods have no own
generic scheme. Ordinary signature types must be closed and first order; this
restriction follows nominal fields and cases as well as structural type children.
A nominal wrapper cannot hide a function, resource, secret, stream or owned value.
Closed recursive, mutual and nested nominal data remain first order. The structural
property is proved under each nominal declaration's ordinary parameter assumptions,
with every actual argument independently checked in its enclosing scope. Even a
phantom argument must be closed and ordinary; recursive binding reuse cannot erase it.
Direct Self parameters form the final affine suffix. A result is direct Self or
an ordinary closed type. A method need not mention Self, but the exact Self owner
and constraint remain mandatory.

An `OwnedImplementation` declaration names one exact contract and concrete owned
Self, and supplies a complete, unique, canonically ordered method map. Every map
entry selects an exact visible monomorphic pure graph function with matching
ordered parameter types, modes, result and kind after Self substitution. All
methods are checked even when unused. Empty-effect tasks and closed externals are
not method implementations; ordinary graph wrappers may call closed externals.
Imported contracts, types and methods must belong to the declared exact visible
package closure.

A function may embed named implementation parameters with distinct identities,
an exact contract and an in-scope Owned type parameter as Self. An implementation
call supplies one explicit operand per parameter. An operand either selects an
exact implementation declaration or forwards an exact witness parameter from its
own lexical function. A method call selects a method identity in the witness's
exact nominal contract. There is no implicit implementation search, overload
resolution, capability grant, ordinary runtime dictionary or function value.
Multiple implementations for the same Self and signature are valid and remain
distinguishable through package transport, preparation and detached artifacts.

Preparation closes finite applications by exact function and ordered selected
implementation references; existing type/effect closure machinery checks type
applications separately. The resulting Self must equal the actual type binding.
An unbound template is not callable or capturable through ordinary entry. Witness
edges participate in closure and recursion analysis. Tail forwarding is derived
after witness calls have selected their exact graph targets. Read-loan lifetime
and cleanup obligations remain valid through tail transfers and nested calls.
The source-derived interpreter resolves the source witness scope and method maps
independently of these prepared instances.

## Encodings, admission and compatibility

The initial owned-generic increment used graph/owner generation 18. Current
graph/owner generation 19 adds structural products. Frozen canonical owner codecs
preserve generations 14–18, and ordinary base type identities remain unchanged.
Package interface generation 12 changes method/function layout; interface 10 and
11 have separate frozen representations. In particular, interface-11 function,
constant and component bytes are not interpreted using generation-12 enum tags.

Current compilation uses compiler unit 17, bytecode 13 and artifact 24. Derived
predecessor artifacts require rebuilding from supported accepted meaning; an old
proof or compiler artifact does not silently become current permission. Authored
owned-only requests retain codec 21; product-bearing requests select codec 22.
Compact changes use contract 26. The semantic validator identity is 19.
Public discovery advertises the actual generations.
Regeneration of maintained artifacts preserves accepted meaning HEADs.

Complete admission includes unused arguments, methods and unreachable expressions.
Canonical source, code and runtime metadata must agree, and independent affine
admission must still reject an invalid graph even if all enclosing digests were
consistently recomputed. There is no inventory-absence shortcut based only on
ByteBuffer. Metadata traversal, witness specialization and recursive closure use
existing finite work/storage admission and cancellation checkpoints; exhaustion
is a resource result, not proof that meaning is invalid. No proof or execution
limit is enlarged by this increment.
