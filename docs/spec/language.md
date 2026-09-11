# Language meaning

Status: normative for the typed meaning graph.

## Representation and evaluation

Runtime lists are immutable flat logical sequences. Their private carrier is a bounded 32-way
index trie with a tail of at most 32 elements. Length and handle sharing are O(1); indexing is
O(log_32(N+1)); ordered traversal is O(N). Append copies the changed branch spine and a bounded
tail of shallow immutable element handles. It never visits or copies a complete old prefix above
that tail, even when old roots remain aliased. Internal levels do not increase language value
depth. Shape, capacity, pointers, and sharing never affect equality, type/layout identities,
semantic digests, artifacts, or typed-data bytes. The carrier confers no origin, affinity, capture,
serialization, or retention permission.

The logical length ceiling is 1,000,000 under the existing collection bound. Each newly reserved
leaf or branch charges 32 collection slots, including unused capacity. Its allocation byte charge
is the in-memory node size plus two `usize` reference-count words; each new element handle charges
the value payload size plus two reference-count words. The inline list header is part of its value
slot. Checked bulk construction also charges its temporary raw-value conversion buffer. Raw ingress
still validates and charges every logical occurrence, including repeated shared children, and
separately charges owned carrier metadata. Sharing does not waive admission. All reservations use
checked arithmetic before allocation, are cumulative without refunds, and depend on no observed
reference counts. Other collection costs and default limits remain unchanged.

For old length L, let P = floor((L-1)/32) for L > 0, otherwise zero, and let H be the least
nonnegative height with P <= 32^H. Every append reserves one leaf and one new element handle.
Only a full tail is promoted: it reserves no branches if P=0, H+1 branches if P=32^H,
and H branches otherwise. Thus at most 32(H+2) slots are reserved per append. An ordinary tail
append copies at most 31 existing element handles; a promoted tail copies none. Each copied
branch has at most 32 shared child slots. Untouched subtrees incur no new allocation charge.

Construction and raw traversal check cancellation independently of observations, including before
result installation. Failure preserves old aliases and releases temporary execution ownership.
Iterative raw disposal remains stack safe for over-depth rejected values with shared list leaves.
JSON output explicitly creates a bounded contiguous JSON array; typed-data encoding traverses the
carrier directly. Physical work observations distinguish node visits, reserved and copied slots,
element handles, nodes, and full external-buffer materializations. Counters grant no fuel or
admission and are separate from logical validation visits.

The standard pure graph function
`list-map<Input,Output>(items: List<Input>, mapper: Function(Input)->Output) -> List<Output>`
uses `list-fold-left`, an empty output, and a private generic step bound to the mapper. The step
invokes its mapper once and appends the successful result. Caller arguments evaluate left to right;
callbacks run once per item in ascending index order, and never for empty input. Task mappers and
wrong signatures reject before semantic acceptance, including for empty input. Traps, exhaustion,
and cancellation stop later callbacks and produce no successful partial list. Nested maps retain
their callback and failure order. For total callbacks with sufficient budgets, mapping identity
preserves values and mapping `function-compose(f,g)` equals mapping g then f by values.
Function signatures containing type parameters remain capture-safe leaves; this introduces no
capture of a bare unconstrained type parameter. Ordinary transient callable list elements retain
their existing restrictions on equality, persistence, and resource containment.

Instantiated composite types are disposable derivations of exact rank-one substitutions.
Production preparation closes all concrete signatures, constants, tests, ports, constructors and
explicit calls over their nominal members; the canonical evaluator independently closes accepted
owners. Both use the existing validation-work, type-depth and allocation limits, check cancellation,
and charge new traversal and metadata before growth. Preparation steps and cumulative metadata bytes
are reported separately from runtime payload work. Derived types use canonical identities and are
never a second editable or serialized type authority.

Language constructs are typed semantic records in canonical owner objects. There is no maintained
source grammar. Executable-discovered compact change records describe bounded authored intent; the
request and logical plan are non-authoritative projections. Names locate meaning, while typed stable IDs own
references, continuity, generic parameters, and selected expression/member sites.

Evaluation is strict and left-to-right except `if` and variant `match`, which evaluate only the
selected branch. `let` bindings and `do` expressions evaluate in declared order. Capability
operations and lexical transactions preserve that order.

Pure graph function bodies are tail contexts. The selected `if` branch, `let` body after
ordered bindings, last sequence item, and selected `match` arm inherit that context. A direct
call or pure `invoke` there transfers to an exact pure graph function without retaining the
outgoing activation, in both production and canonical reference execution. This includes self
and mutual recursion, explicit rank-one instantiation, and admitted package boundaries.

The invoke callee is evaluated first, then arguments once in left-to-right order. Concrete type
arguments are resolved in the outgoing scope and existing call validation completes before
transfer. Outgoing locals and substitutions are discarded; the original return continuation
and operand-stack base are preserved. Arguments, conditions, binding initializers, preceding
sequence items, scrutinees, constructors, and projections do not inherit tail context.
Constants, tests, port expressions, task bodies (including empty-requirement tasks), and closed
externals are not eligible outgoing frames. Pure functions called from them retain the internal
guarantee. An ancestor's transaction and affine ownership are never elided by a pure helper.

Tail eligibility is derived from accepted meaning and strict loaded code, never stored meaning
or an authoring option. Transfers check cancellation and consume execution work without resetting
fuel, allocation, collection, operand, or capability accounting. Every admitted VM graph call,
including a transfer, incurs its existing cumulative local-allocation charge without refunds.
Non-tail calls retain call-frame admission. The unchanged defaults are 10,000,000 VM instructions,
4,096 live call frames, 1,000,000 operand values, 268,435,456 allocated bytes, 1,000,000 collection
items, and 100,000 capability calls. Reference work remains independently counted in expression
and value units. Removed return/jump instructions are not counted as executed work.

This is constant control space for a tail chain, not constant application heap or a termination
or latency guarantee. Infinite tail recursion exhausts work or cancels. Failure releases owned
execution state and produces no successful value receipt; pure execution never advances semantic
HEAD or changes operational data. Residual operands, impure transfer authority, or an outgoing
owned transaction must reject at the owning validation or runtime boundary.

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

Execution admits raw values before installing any local or operand. Admission checks exact
types, nominal and prepared-program identities, resource provenance and containment with an
iterative traversal, checking cancellation at every node. Depth remains 256 and aggregate
collection admission remains 1,000,000 items. Allocation includes newly allocated checked
metadata and remains cumulative; rejected admission installs no partial value.

Each evaluator carries its own construction-controlled classification: affine-free, direct
capability resource, or affine nominal variant. A nominal variant with any direct capability
resource case is affine even in an empty case. Aggregates cannot contain either affine class.
These classifications are disposable execution derivations, never serialized or authored.
Affine-free does not imply durable or serializable.

Local and argument affine eligibility requires constant work per admitted value, independent
of its descendants and of nominal case count. Internal constructors combine checked immediate
children, sharing preserves the proof, and projections preserve the selected child's proof.
Closed intrinsics preserve this boundary. Raw invocation, decoder, host, adapter and retained
resident values require admission; a declared result type or a visibility check is not a
certificate. Exact arity, type substitutions, effects, task scope and live-resource checks
remain required. Legitimate algorithmic, codec and allocation-accounting work is not removed.

An invalid raw result stops downstream execution and releases local resources. An already
performed external effect retains its actual possible visibility; admission does not roll it
back. Lexical transactions retain their existing rollback semantics. Failed admission neither
advances semantic HEAD nor produces a successful result receipt.

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
function forms reject. Partial moves, affine containers, resource polymorphism, resource-capturing closures, async or
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
no constraint dictionary, higher-rank quantification, implicit generic application, specialization in
accepted meaning, or order-dependent inference. Compiler/runtime erasure or specialization is
derived and cannot change graph meaning or artifact determinism.

## Pure function values, binding, and invocation

The public `function-value` expression identifies one named function and supplies all required type
arguments.
`bind` evaluates its callee first and then an ordered prefix of runtime arguments exactly once,
left-to-right. For `Function(P0,...,Pn-1)->R`, binding k exactly typed arguments, with
`0 <= k <= n`, produces `Function(Pk,...,Pn-1)->R`. It never executes the target. Empty binding
preserves the value without allocating an environment; complete binding creates a zero-argument
function. Rebinding concatenates the prefixes in order. `invoke` evaluates its callee and remaining
arguments in that same order, then calls the exact named target with prefix and remaining arguments.

The accepted `Bind { callee, arguments }` owns ordinary expression children and contains no body or
runtime data. At execution, a callable carries exact prepared-program provenance, fully resolved
rank-1 type arguments, and one flat immutable prefix. It retains values, never the creator's frame,
locals, or substitution map, and can escape, be shared, and be invoked repeatedly. An eligible pure
tail invocation transfers directly to the ultimate target with the complete argument list.

Capture-safe stored types are scalars including `StaticText`, records, variants, lists, maps,
options, and results with recursively safe members, and checked pure callables with safe
environments. Every nominal field and case is inspected, including absent affine cases. A function
signature is a leaf for capture safety: its future parameters and result are not stored captures.
Secrets, streams, capability resources, other live resources, and aggregates containing them reject.
A stored type parameter requires an explicit capture-safe constraint from its exact in-scope
pure declaration; unconstrained parameters reject at declaration validation. A generic helper may capture a function whose signature contains type
parameters, or a concrete safe prefix, but cannot capture a bare unconstrained `T`.

Binding has precisely the effects of evaluating its callee and captures. A failed capture stops
later evaluation and installs no callable. Earlier completed effects retain their actual visibility;
only an enclosing transaction can roll back its staged work. Retention admission is bounded,
iterative, cancellable at every visited node and before installation, and counted separately from
input and raw-result admission. Environment edges count toward depth 256 and capture slots toward
cumulative collection and allocation bounds. Internal reads and calls preserve checked values
without rescanning retained descendants or copying a captured collection's payload.

Callable environments are runtime-only and have no semantic equality or serialization. Constants
may evaluate binding under the existing pure constant rules; their evaluated environment is not
accepted data. Durable literals, external encodings, operational stores, queues, backups, and retained
session state reject function values, including functions nested in aggregates. Exported factories
may return private helpers as callables while ordinary package lookup still enforces visibility;
transported code closure includes those helpers.

Ordinary pure expression contexts reject task function values. Component port preparation may bind
an explicitly selected task function under component capability rules; this does not make task
functions freely passable values. `bind` always requires a pure callee, even with an empty prefix
inside port preparation. Task code may capture ordinary capability results into a pure callable.
Lexical lambdas, anonymous bodies, automatic free-variable capture, mutable environments, argument
holes/reordering, task closures, and durable captured environments are outside this language slice.

## Expressions and bindings

The complete graph expression kinds are unit/bool/i64/text/static-text literals, variable,
conditional, lexical let, sequencing, direct call with explicit type arguments, function
reference with explicit type arguments, prefix binding, invocation, record construction and projection, variant
construction and match, list, map, capability operation, and lexical capability transaction.

Compact change records expose unit, bool, i64, text, and static-text literals; lexical variables and
constants; conditionals and sequencing; direct calls; lexical `let`; nominal or structural record
construction and field projection; variants and exhaustive matches; typed lists; exact requirement
capability calls; lexical transactions; named `function-value` expressions with ordered explicit
type arguments; and `bind` and `invoke` with ordered expression arguments. `add.type-parameter` adds an
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
reference interpreter implement direct calls, named function values, bind, invoke, and explicit generic
instantiation independently and are compared in tests. No maintained text is rendered or parsed by
build, check, run, service, or worker paths.

The graph persists declarations and explicit type arguments, not monomorphized runtime addresses.
Accepted values contain no grant, credential, host coordinate, or live resource. Validation and
resource accounting do not make an accepted program a hostile-code sandbox.

## Explicit capture-safe rank-one parameters

A type-parameter owner carries the closed constraint set `[]` or `["capture-safe"]`.
The canonical set tags are 0 (empty) and 1 (capture-safe); unknown tags, names and duplicates
reject. Omission in authoring creates the empty set. Constraints attach to the stable parameter
identity and its exact declaration and are interface meaning even when the parameter is unused.
Pure graph functions, records and variants may declare the constraint. Generic tasks remain
inadmissible. Closed external signatures must match the intrinsic inventory, whose existing
parameters have empty constraints.

At every direct call and named function-value instantiation, every explicit argument must satisfy
its parameter's constraint in the caller's declaration scope and exact package closure. This check
also applies in unreachable branches and when the callee never captures or uses that parameter.
A constrained caller parameter can discharge the same callee constraint; an unconstrained or
foreign-scope parameter cannot. Safe stored children compose through lists, map keys and values,
options, both result branches, every structural or nominal record field, and every variant payload,
including absent cases and empty containers. Already admitted nominal cycles use bounded visitation.
A pure Function type remains a leaf even when its signature mentions unconstrained parameters;
actual callable admission still verifies pure target, exact origin and safe runtime environment.

A constrained body may bind its parameter or aggregates of it. An unconstrained body cannot bind
an actual value of that parameter. There is no implicit constraint inference, subtyping, dictionary,
specialization, generic extraction, task closure, user-defined trait or higher-rank quantification.
Capture safety does not confer equality, serialization, session retention or capability authority.
Existing lifetime, affine resource, pure evaluation order, tail transfer and data rules still apply.

An out-of-scope parameter fails exact declaration-scope validation before callee constraint checking.
Well-scoped constraint failures identify the callee, exact parameter, supplied type, required constraint
and a bounded stored-type path or missing scope assumption. The existing validation-work and type-depth budgets
bound traversal and temporary proof storage. Compiled signatures retain the exact ordered constraints;
strict artifact loading compares them to canonical owners, including unused parameters. Production
preparation derives disposable proofs from compiled inputs and the complete instantiated type closure;
reference preparation derives them independently from canonical owners. Proofs are scoped to that
validated preparation and exact closure, never persisted by TypeObject identity alone. Raw invocation
checks resolved type arguments and real captures before locals or body execution. Repeated invocation
propagates admitted immutable values without rewalking retained payloads. Proof metadata, visited sets
and pending traversal nodes consume their owning work/allocation budgets before growth.

## Parametric nominal data

A record or variant owns an ordered vector of stable type-parameter identities. Each parameter has
exact declaration scope and the same explicit `none` or `capture-safe` constraint as a pure function
parameter. Every positive-arity reference supplies all arguments, in order, through an explicit
application; a plain named reference is valid only for a zero-arity declaration. There are no inferred
arguments, defaults, higher kinds, generic tasks or resource polymorphism. Concrete task signatures
and component ports may contain closed ordinary applications.

Constructors carry the declaration and ordered type arguments. Field and case selectors retain stable
member identities, resolve against that declaration, and substitute the complete application. Matches
remain exhaustive. All arguments discharge exact arity, scope and constraints even for unused or
phantom parameters. Definition edits validate the complete candidate, including all affected callers;
adding parameters to a monomorphic declaration requires updating its dependent references atomically.
Names may change without changing these identities.

Recursive nominal definitions are admitted by non-expansive parameter flow. A slot is the exact
declaration reference and its ordered stable type-parameter identity. For every application of B
in a member type of A, each occurrence of A.i in argument j contributes an edge A.i to B.j. It is
plain when the whole argument is exactly A.i, and expanding when a structural constructor (including
a nominal application) surrounds the occurrence. Nested applications contribute their own edges.
An enclosing field/list/case does not grow an application's argument: `List<tree<T>>` forwards T
plainly. All members, signatures, inactive cases and phantom arguments participate. A closed
argument contributes no source-parameter edge; its nominal references still join the exact closure.

Admission requires that no strongly connected slot component contain an expanding edge. Direct
and mutual recursion, finite permutation/duplication/selection, growth on acyclic slot paths, and
replacement by closed arguments are admitted. Thus `tree<T> = leaf(T) or branch(List<tree<T>>)` is
admitted while `grow<T> = stop or next(grow<List<T>>)` rejects even if only stop is constructed.
A zero-arity wrapper returning to a fixed generic instance is not rejected merely for its
declaration cycle. Existing wholly monomorphic recursion keeps its meaning, and an uninhabited
recursive record need not be rejected.

An expanding edge on a slot cycle can grow an occurrence on every lap. Without one, growth lies
only on the finite acyclic condensation graph; plain cycles select among finitely many forwarded
terms. Finite roots therefore generate finitely many complete canonical applications. Duplication,
permutation and many roots can still exhaust distinct work/storage limits. Preparation visits each
exact instance once and retains recurrence edges. Structural TypeObjects and immutable runtime
values remain finite; this does not add heap cycles, infinite values, structural recursive equality,
polymorphically recursive generic calls, or change execution fuel and resource policy.

An instance layout and its preparation provenance bind the complete canonical application, including
phantom arguments. Same-template instances with identical fields but different arguments are distinct.
Raw ingress rejects a wrong instance or foreign preparation before invoking a downstream callback.
Production layout tables and canonical reference derivation are independent disposable structures.

Eligibility examines every argument and substituted stored member, including inactive variant cases
and empty containers. Ordinary values exclude resources and streams. Capture safety additionally
excludes secrets and unconstrained parameters; function signatures remain leaves, while actual pure
function origin and retained environments are checked separately. Equality, JSON, typed-data encoding
and session state impose their own stronger requirements. A function-containing application can be
ordinary and capture-safe without being comparable, durable or retainable session state. Application
encoding does not extend existing Option/Result codec support.

The maintained standard graph owns `pair<First,Second>`, `pair-new`, `pair-first`, `pair-second` and
`pair-map`. Mapping invokes the first callback and then the second, once each; a trap in the first
prevents the second. These declarations have ordinary graph bodies and no host-specific policy.
