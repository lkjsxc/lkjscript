# Language meaning

Status: normative for the typed meaning graph.

## Representation and evaluation

Language constructs are typed semantic records in canonical owner objects. There is no maintained
source grammar. Executable-discovered compact change records describe bounded authored intent; the
request and logical plan are non-authoritative projections. Names locate meaning, while typed stable IDs own
references, continuity, generic parameters, and selected expression/member sites.

Evaluation is strict and left-to-right except `if` and variant `match`, which evaluate only the
selected branch. `let` bindings and `do` expressions evaluate in declared order. Capability
operations and lexical transactions preserve that order.

## Types and values

The closed current type surface is:

- `Unit`, `Bool`, checked signed `I64`, immutable `Bytes`, UTF-8 `Text`, and compile-time
  `StaticText`;
- opaque `Secret`, typed live `Resource` handles, and exact-interface
  `CapabilityResource<Interface>` values;
- nominal records and variants plus structural adapter records;
- homogeneous lists and deterministic ordered maps;
- option and result;
- task-scoped byte streams;
- function types; and
- a stable type-parameter reference inside one explicit generic declaration.

There are no implicit coercions. `I64` arithmetic traps on overflow and division edge cases.
Indexing and collection growth are checked. Text is valid UTF-8; portable identifier rules avoid
normalization-dependent identity. Maps permit bool, i64, bytes, or text keys and iterate by a
specified total order. Runtime values are bounded to depth 256 and 1,000,000 aggregate collection
items.

Live resources, secrets, streams, database transactions, queue leases, and runtime handles never
enter durable graph values. A durable literal has one canonical typed encoding and an owning
decoder bound.

## Affine capability resources

`CapabilityResource<Interface>` is canonical graph type meaning bound to one exact interface
reference. It is task-local, runtime-only, non-equal, non-serializable, and cannot be fabricated by
a literal, constant, decoder, external, pure function, callback, or constructor. Only an exact
requirement capability call whose result has the same exact interface may acquire one. The runtime
value retains that acquiring requirement as authority.

Every parameter has canonical use meaning: `unrestricted`, `borrow`, or `consume`. Nonresource
parameters must be unrestricted. A direct capability-resource operation parameter must be an
explicit borrow or consume. One private, same-package, nongeneric task function may instead have
exactly one final direct capability-resource parameter with `consume` use. That parameter carries
one canonical `resource_requirement` reference to a requirement in the function effect whose exact
interface matches the resource type. The binding is graph meaning and is never inferred from a
name, order, deployment grant, or runtime handle. Resource results and every other
resource-containing function signature reject.

Affine flow follows ordinary left-to-right evaluation order. A borrow observes one live lexical
owner and preserves it. A consume moves that owner; every later use on a reachable path rejects
before publication. For an admitted direct resource-bearing call, all unrestricted arguments
finish first and evaluation of the final argument commits transfer of one exact live owner. The
callee may borrow it, consume it through the bound requirement, drop it, or forward it through
another admitted direct call. Caller and callee must use the same exact requirement identity, and
the resource-bearing direct-call graph must be acyclic. A call failure, cancellation, or resource
exhaustion after transfer does not restore caller ownership; unwinding drops remaining task-local
authority without an implicit external queue transition.

Dropping an unconsumed resource is allowed. A nominal variant may contain one direct resource
payload: matching consumes the outer owner and makes the payload live only in the selected arm. A
join retains an owner only when every reachable arm retains the same provenance. Records,
structural records, lists, maps, options, results, streams, function values, constants, tests, and
nested nominal values cannot contain a resource. Multiple, borrowed, nonfinal, public,
package-visible, cross-package, generic, indirect, recursive, captured, or result-bearing resource
function forms reject. Partial moves, affine containers, resource polymorphism, closures, async or
detached tasks, and general linear must-use semantics are absent.

## Declarations, effects, and capabilities

A module owns imports, exports, declarations, documentation, annotations, identities, and
relations. Declaration kinds are record, variant, interface, closed external function, pure
function, task function, constant, component, and test.

Records own ordered stable field identities, mutable names, and exact types. Variants own stable
case identities and optional payload types. Interfaces own stable operation identities,
parameters, result, idempotency/possible-visibility class, and relevant limits. Constants own one
typed pure expression.

Functions own stable value-parameter identities, exact result, effect, and body. A pure function
may call only pure meaning. A task function declares the capability aliases and exact interfaces it
may perform. Components bind requirements and ports; deployment grants remain external authority.

There is no ambient overload resolution, global mutation, floating point, set type, user scheduler
primitive, dynamic evaluation, type-class/trait constraint, or implicit generic inference.

## Explicit rank-1 generics

Pure graph functions and closed external functions may declare an ordered list of type parameters.
Each parameter has a stable `typeparam_` identity and a mutable declaration-local name. Parameters
may occur recursively in parameter types, result types, structural record/list/map/option/result/
stream/function types, bodies, direct calls, and named function values.

Generic application is explicit and order-independent:

- a direct call supplies exactly one type argument for every declared parameter;
- a named function value is instantiated by the same exact ordered arguments before it receives a
  monomorphic function type;
- validation resolves each type argument in the caller's scope and substitutes it recursively
  through parameter and result types; and
- omitted or excess arguments, an out-of-scope type parameter, duplicate parameter name, or missing
  substitution rejects.

Generic task functions reject. Recursive generic cycles may pass their own type parameters in the
same order; a cycle that changes ordered type arguments rejects as polymorphic recursion. There is
no constraint dictionary, higher-rank quantification, partial application, specialization in
accepted meaning, or order-dependent inference. Compiler/runtime erasure or specialization is
derived and cannot change graph meaning or artifact determinism.

## Named function values and invocation

The public `function-value` expression identifies one named function and supplies all required type
arguments.
`invoke` evaluates a function-valued expression, then evaluates arguments left-to-right and calls
the named function. Function values contain stable named-function provenance, not a code address,
lexical scope, or captured environment.

Ordinary pure expression contexts reject task function values. Component port preparation may bind
an explicitly selected task function under component capability rules; this does not make task
functions freely passable values. Lexical lambdas, anonymous functions, closure capture, partial
application, and durable captured environments are not implemented.

## Expressions and bindings

The complete graph expression kinds are unit/bool/i64/text/static-text literals, variable,
conditional, lexical let, sequencing, direct call with explicit type arguments, function
reference with explicit type arguments, invocation, record construction and projection, variant
construction and match, list, map, capability operation, and lexical capability transaction.

Compact change records expose unit, bool, i64, text, and static-text literals; lexical variables and
constants; conditionals and sequencing; direct calls; lexical `let`; nominal or structural record
construction and field projection; variants and exhaustive matches; typed lists; exact requirement
capability calls; lexical transactions; named `function-value` expressions with ordered explicit
type arguments; and `invoke` with ordered expression arguments. `add.type-parameter` adds an
ordered stable parameter to a pure function created or selected through the current function
surface. Generic task functions, map expressions, and arbitrary topology creation remain outside
this compact slice. The generated [change grammar](../generated/change-grammar.md) is the
exhaustive public inventory; this specification does not duplicate its fields and edges.

Bindings and expression sites receive typed IDs only where operations, diagnostics, or relations
need robust selection. Structural paths are canonical within the owning declaration. Paths, source
span padding, and dense compiler indexes are not global semantic identities.

All accepted references resolve to exact package/module/declaration/member identities, and
canonical relations retain those stable bindings. Imports store exact package/module identities,
exports store declaration IDs, expression references store exact declaration references, targets
store exact component identities, and target-owned HTTP routes store exact port identities. Module rename therefore does not rewrite
importers or targets. Declaration rename changes its owning module and name summary without
rewriting callers. Declaration move is not yet local because an exact declaration reference
deliberately includes its owning module identity.
Unresolved or ambiguous references may occur only in typed non-executable draft holes.

## Equality, tests, and failures

Value equality is type-directed and deterministic. Function values and live resources do not
support semantic equality; resource and secret values do not provide durable equality. Tests own
actual and expected typed expressions and pass only when bytecode and the independent semantic
reference interpreter produce equal values and failure observations.

A typed `Result` is ordinary expected program data. Trap, capability failure, possible external
visibility, resource exhaustion, cancellation, corruption, and infrastructure failure are
distinct runtime classes and cannot be silently converted. Exact adapter contracts define which
external failures become typed operation results.

## Validation, compilation, persistence, and security

Acceptance checks namespace uniqueness, visibility, imports/exports, stable identity shape,
generic parameter scope/substitution/recursion, type agreement, effect closure, capability
membership, exact resource provenance and language-order borrow/consume flow, branch joins and
escape, component requirements and ports, target bindings, test types, expression/binding shape,
canonical relations, and exact dependency closure.

A precondition-free transaction may prepare locally when it contains only eligible pure-function
body replacements, only independent empty-module creations, only module renames, or only
declaration renames. Body
replacement checks the selected modules and their recursive local import dependencies; module
rename checks the renamed modules and their outgoing import dependencies; declaration rename
checks changed owning modules and exact namespace summaries. Every request with preconditions,
every mixed request, and every other change uses complete package reconstruction,
canonicalization, and validation. Focused tests compare local results
with the complete oracle; inability to prove eligibility widens rather than narrowing.

Typed source owners retain the internal summary, fact, and validator compatibility identities. The
semantic summary produces disposable content-addressed module summaries. Semantic facts bind their
exact inputs and digests, graph-owned test owners, and typed
reverse dependency edges in three persistent maps. The accepted revision authenticates the map
roots with a revision-independent semantic certificate. The four local transaction paths update
those facts by path-local delta, but the dependency frontier does not yet select general
validation.

Compiler lowering consumes validated graph structures directly. The bytecode VM and semantic
reference interpreter implement direct calls, named function values, invoke, and explicit generic
instantiation independently and are compared in tests. No maintained text is rendered or parsed by
build, check, run, service, or worker paths.

The graph persists declarations and explicit type arguments, not monomorphized runtime addresses.
Accepted values contain no grant, credential, host coordinate, or live resource. Validation and
resource accounting do not make an accepted program a hostile-code sandbox.
