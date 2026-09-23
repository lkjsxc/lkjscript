# Language meaning

Status: normative for the typed meaning graph.

## Representation and evaluation

### Raw byte observation

`bytes-get(bytes: Bytes, index: I64) -> I64` returns the octet at a zero-based byte
offset as an integer in 0 through 255. Bytes is opaque binary data: NUL, CR, LF,
invalid UTF-8 and UTF-8 continuation octets are observed without conversion.
Negative offsets, offsets at or beyond the byte length, and every offset into an
empty buffer trap. Neither I64 extreme wraps, clamps or counts backwards. Callers
that require a fallback can guard with `bytes-length` and ordinary conditionals.

The general closed external `core.bytes.get` has exactly `(Bytes, I64) -> I64`
and is pure. It preserves the input and aliases, performs a checked indexed read,
and does not allocate or copy a byte buffer. Existing value admission, execution
cancellation and caller budgets remain in force. The production and source-reference
implementations are separate. This does not add byte literals, implicit integer-list
conversion, text character indexing, filesystem authority or a new persisted encoding.
A runtime lacking this intrinsic rejects its external declaration during ordinary
closure admission; unchanged old exact closures retain their previous behavior.


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

Ordered maps use one neutral persistent AVL carrier with immutable shared entry handles.
Insertion, replacement and removal copy only their search and rotation paths. Retained roots
keep their entries and untouched subtrees; existing keys and payloads are never copied by an
edit. Ordered traversal, equality and external bytes depend on contents rather than tree shape.
The existing Bool/I64/Bytes/Text key order and StaticText treatment are unchanged. Literals
evaluate in authored order before their existing duplicate-key failure boundary; intrinsic
arguments, including a get-or fallback for a present key, remain strict.

Each evaluator independently reserves map storage before growth. One newly allocated node
charges one collection item and `size_of(Node) + 2*sizeof(usize)` bytes, including its Arc
reference counts; Node contains one entry handle, two child links and its height. A new entry
charges `size_of(Entry) + 2*sizeof(usize)` bytes, including the inline key and value, with no
second item charge. A key converted from Text/Bytes additionally reserves its owned buffer
before copying. Existing payloads move into new entries; shared payloads, keys and subtrees
incur no fresh deep-copy charge. Even transient rotation nodes remain cumulatively charged.
Absent removal allocates no tree storage. Neither finite value admission (depth 256 and
1,000,000 aggregate items), single-allocation bounds nor project defaults are raised.
Trusted foreground execution continues to omit cumulative quotas unless explicitly selected.

Map projection reserves owned option/result/variant boxes before cloning their spine and
stops at shared aggregate handles. Ordered entries reserve their result buffer, each key's
new Arc buffer, each two-field record's vector, reference counts and field-name buffers,
then the ordinary persistent-list construction. Bulk raw ingress moves a bounded temporary
ordered builder into the carrier. It supplies no type, origin, effect or affinity certificate;
both evaluators independently admit all children and map metadata, including retained captures.
Cancellation is checked before reservations and successful result installation. Refused
reservations install no partial root and do not alter their ledger; earlier reserved work is
not refunded. Iterative disposal covers rejected raw payloads, partially built trees and aliases.
Map-node disposal uses bounded inline traversal storage. Terminal payloads, a single shared
payload and unary raw-value chains need no cleanup heap growth; branching aggregates reuse owned
vectors or the existing raw-disposal worklist. That non-fallible cleanup bookkeeping remains
separate from evaluator construction reservations: it cannot publish values, grant execution
or stop releasing ownership because a construction quota was exhausted.
Physical map observations saturate independently of quota accounting and are not allocator/RSS
measurements. No persisted encoding or migration follows from this runtime representation.

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

The standard pure graph function
`list-window<Item>(items: List<Item>, start: I64, count: I64) -> List<Item>` returns an ordered
subsequence. Let `length` be the input length, `lower = min(max(start, 0), length)` and
`size = min(max(count, 0), length - lower)`. The result contains the input values at indexes
`lower` through `lower + size - 1`, or is empty when size is zero. Negative indexes do not count
back from the end. Clamping precedes end-index addition, including at both signed I64 extremes.
The native function uses a private ordinary tail-recursive helper, indexed reads and persistent
append. It invokes no callback and preserves input values and order. Item remains unconstrained;
ordinary type, affinity, result encoding and execution admission still apply. This adds no
intrinsic, borrowed slice, lazy stream or implicit iteration budget.

Instantiated composite types are disposable derivations of exact rank-one substitutions.
Production preparation closes all concrete signatures, constants, tests, ports, constructors and
explicit calls over their nominal members; the canonical evaluator independently closes accepted
owners. Both use the existing validation-work, type-depth and allocation limits, check cancellation,
and charge new traversal and metadata before growth. Preparation steps and cumulative metadata bytes
are reported separately from runtime payload work. Derived types use canonical identities and are
never a second editable or serialized type authority.

Language constructs are typed semantic records in canonical owner objects. Executable-discovered
change records and structural expression blocks describe bounded authored intent; neither a saved
request nor a definition projection is a separately editable program authority. The request and
logical plan remain proposals and review evidence. Names locate meaning, while typed stable IDs own
references, continuity, generic parameters, and selected expression/member sites.

Evaluation is strict and left-to-right except `if` and variant `match`, which evaluate only the
selected branch. `let` bindings and `do` expressions evaluate in declared order. Capability
operations and lexical transactions preserve that order.

Pure and task graph function bodies are tail contexts. The selected `if` branch, `let` body
after ordered bindings, last sequence item, and selected `match` arm inherit that context.
A direct call or `invoke` there transfers to an exact graph function without retaining the
outgoing activation, in both production and canonical reference execution. This includes self
and mutual recursion, fully or partially bound descriptors, explicit rank-one type/effect
applications, and admitted package boundaries.

The invoke callee is evaluated first, then arguments once in left-to-right order. Concrete type
and effect arguments are resolved in the outgoing scope and complete target, captured-prefix,
resource and task-row admission finishes before transfer. A task target must fit the outgoing
activation's closed allowance and actual canonical grants. The callee installs its own row;
a task-to-pure transfer removes task permission, and pure-to-task is invalid even for an empty
task row. Outgoing locals and substitutions are discarded; the original return continuation
and operand-stack base are preserved. Arguments, conditions, binding initializers, preceding
sequence items, scrutinees, constructors, and projections do not inherit tail context.
Constants, tests, port expressions and closed externals retain their entry/execution contracts.
Graph functions called from them receive the internal guarantee. A transaction body retains its
commit/rollback continuation; its call is ordinary. Helpers beneath an ancestor transaction
may transfer while that exact ancestor retains transaction ownership. An unused affine local
does not prevent a task transfer: dropping its descriptor neither releases the invocation's
table entry/admission capacity nor performs any queue completion, failure or stream operation.
The admitted final-consume helper protocol still transfers the exact right once, after ordinary
arguments finish, without changing private/same-package/acyclic signature restrictions.

Tail eligibility is derived from accepted meaning and strict loaded code, never stored meaning
or an authoring option. Transfers check cancellation and consume execution work without resetting
fuel, allocation, collection, operand, or capability accounting. Every admitted VM graph call,
including a transfer, incurs its existing cumulative local-allocation charge without refunds.
Non-tail calls retain call-frame admission. The unchanged defaults are 10,000,000 VM instructions,
4,096 live call frames, 1,000,000 operand values, 268,435,456 allocated bytes, 1,000,000 collection
items, and 100,000 capability calls. Reference work remains independently counted in expression
and value units. Removed return/jump instructions are not counted as executed work.

For a fixed finite prepared program and fixed non-tail nesting/cleanup depth, a longer admitted
terminal graph-call chain uses constant additional live frames, local slots and substitution/
allowance state. Payload retention and cumulative allocation remain separately bounded. This
does not guarantee termination, unlimited execution or IO speed. Infinite tail recursion exhausts
work or cancels. Failure releases owned
execution state and produces no successful value receipt; pure execution never advances semantic
HEAD or changes operational data. Residual operands, mismatched ownership, forged terminal
control flow or a falsely certified omitted transaction continuation reject at their owning
boundary. Valid calls with pending work retain that work and remain ordinary calls.

## Types and values

The closed current type surface is:

- `Unit`, `Bool`, checked signed `I64`, binary64 `F64`, immutable `Bytes`, UTF-8 `Text`, and compile-time
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

## Binary64 computation

`F64`, authored as `f64`, is an ordinary immutable scalar in structural and nominal data, lists,
explicit generics, pure binding, permitted retained state and typed persistence. It adds no
implicit coercion, capture permission or authority. F64 is excluded from map and application-data
keys. I64 retains its exact integer domain and checked arithmetic.

The domain includes finite binary64 values, subnormals, both zeros, both infinities and one quiet
NaN with bits `0x7ff8000000000000`. Every NaN-producing operation normalizes to that value; every
other bit is preserved. Add, subtract, multiply, divide and square root use nearest-even rounding
after each operation, in authored order. There is no reassociation, implicit fused multiply-add,
flush-to-zero, selectable rounding mode or floating exception flag. Overflow produces infinity;
underflow rounds gradually. Nonzero divided by zero produces the appropriately signed infinity;
zero divided by zero and other invalid arithmetic produce canonical NaN. Negation and absolute
value preserve these rules. The supported native primitive set makes no transcendental promise
beyond correctly rounded square root. Rust's [binary64 and square-root contract](https://doc.rust-lang.org/std/primitive.f64.html#method.sqrt)
supplies these operations; its unspecified NaN payload is normalized explicitly.

`core.value.equal` compares F64 leaves numerically: NaN is unequal to everything, including itself,
and the two zeros are equal. This rule recurses through every aggregate even when both operands
share one immutable object. `core.f64.less` and `core.f64.less-equal` are false if either operand
is NaN. `core.f64.is-finite` and `core.f64.is-nan` allow ordinary libraries to choose failure policy;
there is no implicit total floating order.

Representation equality instead compares normalized bits: NaN is reflexive and zero signs differ.
Canonical identity, evaluator agreement and graph-test expected-value comparison use representation
equality at F64 leaves and preserve all previous nonfloat comparison behavior. They do not invoke
program equality to compare NaN observations. VM and reference execution retain separate dispatch
and recursive comparators.

`core.f64.from-i64` explicitly rounds an I64 to nearest-even binary64. `core.f64.to-i64-result`
truncates toward zero and returns `{valid: Bool, value: I64}`. It rejects nonfinite values and
values outside `[-2^63, 2^63)`, returning false and integer zero as deterministic failure filler.
The upper bound is strict: binary64 rounding of `i64::MAX` is already `2^63`.

Floating literals have flat form `expression.f64 as=$VALUE value=TOKEN` and structural form
`(f64 TOKEN)`. Both lower to identical typed intent. Tokens use the locale-independent JSON
decimal grammar (optional fraction/exponent), or exactly `nan`, `inf`, `-inf`. Integer-form decimal
tokens are legal here. Whitespace, leading plus, hexadecimal notation, underscores and locale
separators are invalid. Decimal overflow is invalid; underflow rounds to a signed subnormal or
zero. Conversion uses the complete original token and correctly rounded Rust decimal parsing.
Equivalent decimal spellings produce the same canonical numeric meaning; existing reviewed
request and idempotency bindings remain in force.

`core.f64.parse-result` applies that grammar to Text and returns `{valid: Bool, value: F64}`;
failure returns false and positive zero. `core.f64.to-text` uses Ryu shortest round-trip finite
formatting, with `-0.0`, `nan`, `inf`, `-inf` for the distinguished spellings. Text presentation
does not define graph identity. Numerical accumulation, merging, finiteness policy and invalid
result variants belong in ordinary libraries. Floating addition and merging are not associative;
different evaluation orders need not produce identical bits.

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

There is no ambient overload resolution, global mutation, set type, user scheduler
primitive, dynamic evaluation, type-class/trait constraint, or implicit generic inference.

## Explicit rank-1 generics

Pure and task graph functions and closed external functions may declare an ordered list of type parameters.
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

Ordinary recursive generic applications obey finite parameter flow. Each vertex is one exact
package/function/ordered ordinary type-parameter slot. For every syntactic direct call and named
function value, each caller-parameter occurrence in target argument position j contributes an edge
to target slot j. The edge is plain only when the entire argument is that parameter; a canonical
constructor enclosing it makes the edge expanding. Closed arguments contribute no source edge.
All argument arities, kinds, scopes and capture-safe constraints remain checked, including unused
parameters, private functions, unused descriptors and untaken branches. Bind and Invoke use already
instantiated monomorphic descriptors and introduce no further type-application syntax.

Admission rejects an expanding edge whose endpoints lie in the same strongly connected slot
component. Permutation, duplication, projection and closed resets are allowed. For example,
`f<A,B> -> f<B,A>`, `f<A,B> -> f<A,A>` and `f<A,B> -> f<I64,List<A>>` are finite; the last grows
along an acyclic slot edge and then resets. Mutual `f<T> -> g<List<T>>`, `g<U> -> f<I64>` is also
finite. `f<T> -> f<List<T>>` and `f<A,B> -> f<B,List<A>>` reject. Every child type participates,
including structural fields, containers, nominal arguments (even phantom arguments), and pure/task
function parameter and result types. Variance does not cancel constructor growth. Substitution is
simultaneous from the caller's original ordered vector; same-spelled parameters of different
declarations have distinct identities.

Finite syntax supplies finitely many ground seeds and constructors. Inside a strongly connected
component only plain forwarding remains, and growth can cross only finitely many components.
Thus exact callable instantiation is finite for the current rank-one type language. Nominal member
expansion retains its separate finite-flow rule: nominal types cannot synthesize callable applications
or feed computed type projections back into arguments. Finite exact effect unions multiply this
closure by a finite set, preserving simultaneous type/effect permutation and union. Exact package
dependencies are acyclic, so local callable SCC admission requires no foreign private bodies;
complete suppliers are independently admitted for execution. An interface alone proves no body closure.

The rule is semantic admission before publication and executable loading, independent of runtime
reachability or execution fuel. A large finite closure may still exhaust preparation work/storage;
this is distinct from `kernel_callable_expansion`. Once an expanding SCC is proved, bounded witness
rendering retains semantic rejection and explicit omission information. This guarantees neither
runtime termination nor constant memory, affine/resource eligibility, additional effects or grants,
or hostile-code isolation. There is no constraint dictionary, higher-rank quantification, implicit generic application, specialization in
accepted meaning, or order-dependent inference. Compiler/runtime erasure or specialization is
derived and cannot change graph meaning or artifact determinism.

Pure and task graph functions may also own ordered rank-one effect parameters. Each has a stable
`effectparam_` identity, an exact function owner and position, and a mutable local name. Type and
effect scopes are separate. An effect row is a sorted unique set of exact requirement references
and in-scope effect-parameter references. Authored unions normalize; noncanonical encoded rows
reject. Direct calls and named function values supply exactly one ordered row argument per effect
parameter, including unused parameters and unreachable applications. There is no inferred row,
wildcard, subtraction, or grant lookup by parameter name.

Effect substitution traverses parameter/result types and nested callable, aggregate and nominal
applications. Unknown rows can be forwarded but do not identify concrete operations. Symbolic
containment requires the same exact parameter identity. Closed requirement unions are finite and
idempotent, so recursive effect forwarding, permutation and union need no additional application
shape restriction. Preparation admits distinct closed applications under checked work/storage
bounds; the ordinary-type recursive-call restriction above remains in force.

## Function values, binding, and invocation

The public `function-value` expression identifies one named function and supplies all required type
arguments and effect arguments. Pure `Function(P0,...,Pn)->R` and
`TaskFunction(P0,...,Pn)->R ! E` are distinct exact types, with no implicit coercion or effect
subtyping. An explicitly task callable remains task when its row is empty.
`bind` evaluates its callee first and then an ordered prefix of runtime arguments exactly once,
left-to-right. For `Function(P0,...,Pn-1)->R`, binding k exactly typed arguments, with
`0 <= k <= n`, produces `Function(Pk,...,Pn-1)->R`. It never executes the target. Empty binding
preserves the value without allocating an environment; complete binding creates a zero-argument
function. Rebinding concatenates the prefixes in order. `invoke` evaluates its callee and remaining
arguments in that same order, then calls the exact named target with prefix and remaining arguments.

The accepted `Bind { callee, arguments }` owns ordinary expression children and contains no body or
runtime data. At execution, a callable carries exact prepared-program provenance, fully resolved
rank-1 type/effect arguments, and one flat immutable prefix. It retains values, never grants,
adapters, credentials, resources, a task scope, transactions, the creator's frame,
locals, or substitution map, and can escape, be shared, and be invoked repeatedly. An eligible pure
tail invocation transfers directly to the ultimate target with the complete argument list.

Capture-safe stored types are scalars including `StaticText`, records, variants, lists, maps,
options, and results with recursively safe members, and checked pure callables with safe
environments, including task callables. Every nominal field and case is inspected, including absent affine cases. A function
signature is a leaf for capture safety: its future parameters and result are not stored captures.
Secrets, streams, capability resources, other live resources, and aggregates containing them reject.
A stored type parameter requires an explicit capture-safe constraint from its exact in-scope
function declaration; unconstrained parameters reject at declaration validation. A generic helper may capture a function whose signature contains type
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

Pure contexts may create, bind, return, copy and compose task descriptors without executing them.
Binding preserves task kind and the entire effect row. Task invocation requires a task calling
context, containment in that activation's declared substituted allowance, and actual checked
component bindings. A globally available grant cannot widen the activation's allowance. Resources
may be acquired and consumed lexically within the callback, but resource-taking functions cannot
be indirect callbacks and retained prefixes cannot contain live resources. Pure ports remain pure;
task ports use explicit closed task-callable contracts. The selected component entry remains an
authority boundary, with no task-as-pure representation or purity exception.
Lexical lambdas, anonymous bodies, automatic free-variable capture, mutable environments, argument
holes/reordering, and durable captured environments are outside this language slice.

## Expressions and bindings

The complete graph expression kinds are unit/bool/i64/text/static-text literals, variable,
conditional, lexical let, sequencing, direct call with explicit type/effect/requirement arguments, function
reference with explicit type/effect/requirement arguments, prefix binding, invocation, record construction and projection, variant
construction and match, list, map, capability operation, lexical capability transaction, and
lexical transaction completion outcome.

Compact change records expose unit, bool, i64, text, and static-text literals; lexical variables and
constants; conditionals and sequencing; direct calls; lexical `let`; nominal or structural record
construction and field projection; variants and exhaustive matches; typed lists and maps; exact requirement
capability calls; lexical `transaction` and `transaction-outcome`; named `function-value` expressions with ordered explicit
type/effect/requirement arguments; and `bind` and `invoke` with ordered expression arguments.
`add.type-parameter`, `add.effect-parameter` and `add.requirement-parameter` add ordered stable
parameters through the current declaration surface. All 23 public expression forms also have
structural syntax inside `expression.block as=$ROOT` / `expression.end`. Declarations, signatures,
types, rows, requirements and dependency selection keep their explicit record forms. The generated
[change grammar](../generated/change-grammar.md) is the exhaustive public inventory;
[the change input specification](semantic-cli.md#structural-expression-bodies) owns block framing and lexical input.

Structural syntax lowers each expression occurrence and each lexical binder into ordinary typed
authored intent. A block exports only its root, which has the same exactly-once ownership rule as a
flat expression fragment. Its nested expressions and bindings are private request notation, while
the accepted owners remain inspectable through ordinary definition and owner discovery. A genuine
flat parent may own the root; no outer edge may extend the block or address its private children,
and a block cannot splice a separately defined flat expression into its interior.

Sequential and nested lexical shadowing are permitted. A let initializer sees the preceding
environment, then its new distinct binder is visible to later initializers and the body. The
enclosing environment is restored on scope exit. Match payloads exist only in their own arm, and
transaction bindings only in their transaction body. Bare structural local names select the nearest
binding in that block; explicit typed parameter references and exact local identities retain the
ordinary kind and scope rules. There is no cross-block lexical capture or implicit closure.

Each syntactic local read is a separate expression occurrence. Only a let binding provides
evaluated-once value reuse. Structural notation preserves left-to-right arguments, sequence items,
record fields and map entries, callee-before-arguments order and selected-branch evaluation. It
neither changes transaction completion nor infers generic applications, grants or retries.

`transaction-outcome` explicitly instantiates the ordinary standard result family with one type
argument, like nominal variant construction. Inference independently checks that the body has
that type and returns `TransactionOutcome<T>`. Its exact declaration/case references are explicit
named references in both notations and remain in the typed reference inventory. The
[completion contract](effects-capabilities.md#completed-lexical-transaction-outcomes) defines
finalization, the private body value, condition/conflict branches and independent-effect scope.

Equivalent flat and structural normalized authored trees retain canonical intent bytes, request
commitments and same-base prepared candidates. Semantic binding Names, optional type annotations
and explicit applications remain meaningful. Whitespace, comments and request-local label spelling
are not meaning. All-form coverage does not make bare names equivalent to every exact-addressed
flat graph: a flat reference may still select an earlier same-named binder after shadowing. Such
exact editing remains available in the flat notation.

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

Value equality is type-directed and deterministic, with the F64 numerical rule above. Function values and live resources do not
support semantic equality; resource and secret values do not provide durable equality. Tests own
actual and expected typed expressions and pass only when bytecode and the independent semantic
reference interpreter produce equal representation values and failure observations.

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
Pure and task graph functions, records and variants may declare the constraint. Generic tasks
carry explicit type and effect arguments under the same scope/constraint admission. Closed external signatures must match the intrinsic inventory, whose existing
parameters have empty constraints.

At every direct call and named function-value instantiation, every explicit argument must satisfy
its parameter's constraint in the caller's declaration scope and exact package closure. This check
also applies in unreachable branches and when the callee never captures or uses that parameter.
A constrained caller parameter can discharge the same callee constraint; an unconstrained or
foreign-scope parameter cannot. Safe stored children compose through lists, map keys and values,
options, both result branches, every structural or nominal record field, and every variant payload,
including absent cases and empty containers. Already admitted nominal cycles use bounded visitation.
Pure Function and TaskFunction descriptors remain capture-safe leaves even when signatures mention
unconstrained parameters. Actual callable admission verifies exact pure/task target, prepared origin
and retained environment. Binding a task descriptor acquires no grants; invocation still requires
the calling task effect allowance and exact deployment bindings.

A constrained body may bind its parameter or aggregates of it. An unconstrained body cannot bind
an actual value of that parameter. There is no implicit constraint inference, subtyping, dictionary,
specialization, generic extraction, anonymous lexical capture inference, affine captures, user-defined
trait or higher-rank quantification. Explicit named task descriptor binding is supported.
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
arguments, defaults, higher kinds or resource polymorphism. Generic task functions use explicit
type/effect arguments; concrete task signatures
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
unbounded generic callable instantiation, or change execution fuel and resource policy.

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

## Explicit requirement parameters

A pure or task graph function may own an ordered rank-one list of requirement parameters, separate
from its ordinary type and effect-row parameters. Each has a stable identity, one owning function,
a mutable local name, an exact interface reference and a canonical finite minimum operation set.
The empty set is legal and still constrains the interface. Operation identities must belong to
that exact interface. Closed external intrinsics cannot declare these parameters.

A requirement operand has a typed concrete-requirement or requirement-parameter discriminant.
Parameter operands resolve only in their exact owning function; identical names in different
functions are unrelated. Every direct call and named function-value expression supplies one ordered
requirement argument per formal. No argument is inferred, defaulted or selected by a mutable name.
Arity, kind, scope, exact interface, minimum operations and visibility are checked independently
of ordinary and effect arguments, including unused formals and unreachable expressions.

A concrete argument must supply the formal's exact interface and every minimum operation.
A forwarded parameter must declare a constraint that entails the callee's constraint. Validation
checks the generic body, so a favorable concrete application cannot authorize an operation outside
that body's declared minimum. Requirement operands may occur in task rows, nested task-callable
types, capability calls and lexical transactions. Unknown effect-row parameters remain unions
that cannot identify a particular operation.

Substitution is simultaneous across exact scopes and traverses nested callable and nominal types,
rows and applications. Substituting concrete requirement q for R yields the whole exact atom q.
Minimum constraints do not attenuate q: an explicitly supplied callback whose row contains q may
use an additional operation permitted by q, although the generic body cannot name it through R.
Rows still use normalized set union, exact equality and the existing concrete coverage relation.
Different requirement identities never merge merely because their interfaces, names or grants agree.

Named callable descriptors retain their closed ordered requirement arguments along with exact
provenance and ordinary/effect applications. Binding preserves this data. Pure factories may
construct, bind and return task descriptors without running operations. No descriptor retains a
grant, adapter, credential, live resource, transaction, creator frame or mutable substitution scope.
Prepared application identities include the requirement vector even when ordinary parameter and
result types coincide. Invocation and direct/tail calls check current allowances and concrete grants.

Requirement applications range over a finite exact reference universe. Recursion may simultaneously
permute requirement, ordinary and effect arguments under the existing finite parameter-flow rule.
This does not admit expanding ordinary-type cycles. Preparation work/storage exhaustion and
cancellation remain separate from semantic invalidity; formal operation constraints introduce no
quota or counter reset.
