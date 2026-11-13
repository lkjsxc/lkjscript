# Language

This file describes the implemented generation-two subset. Status names the
unimplemented parts of the intended language.

## Source and values

The surface is Rust-compatible. Integer literal fallback is i32; annotations and
suffixes retain distinct types, including usize/u64 and isize/i64. Arithmetic is
checked. Signed minimum literals are accepted; out-of-range integer and non-finite
floating literals are rejected. Explicit casts follow Rust conversion behavior.
Integer wrapping_add, wrapping_sub and wrapping_mul use modulo arithmetic at the
receiver's width; the argument must have the same integer type. The checked_add,
checked_sub and checked_mul methods are ordinary source-library implementations
returning Some(result) or None on overflow for all ten supported integer types.
Both operands evaluate once in receiver-then-argument order. Ordinary arithmetic
operators remain checked and trap on overflow. Booleans and Unicode scalar
characters are distinct from integers. Characters
occupy four bytes; surrogate code points and values above U+10FFFF are rejected.
Characters compare by scalar value, cast to integers, and can be constructed by
casting u8. Other integer-to-character casts and character arithmetic reject.
Character patterns include scalar ranges and account for the surrogate gap.
Expressions evaluate
left to right; logical operators short circuit. Assignment and the supported
primitive compound assignments evaluate the RHS before the destination and its
index expressions; compound assignment then reads the destination value. Each
operand executes once. There is no unwinding.

Functions, structs, enums, constants, type aliases, generic declarations, trait
implementations, matches, if, while, loop and for are implemented. For-loop
continue advances the iterator; an inclusive maximum endpoint does not overflow.
Assertions, panic and the implemented vec forms are fixed syntax. Named
diagnostics reject unsupported constructs.

Unparenthesized block forms in statement position end before a following binary
operator, call or index that can start a new expression; dot and question-mark
postfixes continue the expression. Parentheses select ordinary value-expression
parsing, as do initializers, arguments, returns and operator right-hand sides.
Match arms use the same block boundary and require commas after grouped or
postfix expressions. An implicit block statement must produce unit unless it
diverges; an explicit semicolon may discard any value with ordinary cleanup.
Statements and tails after an unconditional return are still checked. `if let`
and `while let` lower to ordinary matches and loops. `let else` requires its
failure branch to diverge and extends borrowed initializer temporaries through
the binding scope. Result/Option `?` branches to ordinary early return; Result
error types must currently be equal. Loops accept a value on `break`, unified
across their exit edges. Struct update evaluates explicit fields in source order,
then the base once, transferring only the remaining fields.

Integer, byte and character literal patterns support inclusive, half-open and open ranges:
`a..=b`, `a..b`, `a..`, `..b` and `..=b`. Endpoints share ordinary integer literal
type, suffix, radix, sign and overflow checks. Empty/reversed ranges reject.
Exhaustiveness unions closed intervals over the concrete integer domain, including
signed minima and u64::MAX; it does not enumerate the domain. Source arm order
controls execution. Every arm is checked, including bodies after a wildcard or
binding default and bodies in unused generic definitions. Scalar matches through
references read the referent and preserve mutable-reference reborrowing.
Named/runtime range endpoints remain unsupported. Nested tuple, struct and enum
patterns, reference binding modes, or-patterns and match guards execute in source
order. Failed pattern tests and guards do not move payloads. Guard bindings are
protected by shared loans until the guard succeeds; mutation or moving an affine
binding in a guard is rejected. Overlapping or alternatives retry the guard in
source order. All alternative products share their source tree and continuations,
including guarded products, nonuniform reference bindings and irrefutable lets.
Pure pattern tests are cached before guards; impossible subtrees are skipped.
The selector retries feasible choices in source order without expanding a static
Cartesian product. An effectful guard can still require visiting many choices.
Guards cannot assign or mutably borrow a tested place or its ancestor; unrelated
untested fields remain writable. Copy bindings are frozen too. A wildcard does
not inspect storage. Discriminant-only later arms can use the cached test after
a guard consumes an owner; any later payload access still requires ownership.
Targeted K loop transfers preserve enclosing source loops when lowering needs an
internal loop; invalid depths reject even in unreachable K.

Enums retain distinct unit, tuple and named-field variant shapes. Named constructors
evaluate initializers in written order and store payloads in declaration order.
Tuple and tuple-variant rest patterns select typed prefix/suffix fields. Unannotated
let patterns preserve the initializer's reference binding mode. Named patterns support
reordered fields, renamed/shorthand bindings, wildcards and rest patterns. Constructor and pattern fields are checked for names, completeness,
duplicates, type and declared shape. Unit/empty-tuple variants also accept empty
braces, as in the common Rust subset. Enum fields inherit enum visibility; explicit
field visibility is invalid. Nonempty tuple numeric-brace syntax remains unsupported.

Explicit ref/ref mut match bindings refer to the original storage. Nonmoving
matches preserve the owner, mutability and destructor timing. Distinct fields of
one proven enum variant may be mutably borrowed together; other tags or the same
field do not gain disjointness. Existing-place matches may combine affine moves and
explicit reference bindings of disjoint fields. Tag tests name projected enum
places, so testing a nested payload preserves its owner and lexical cleanup.

Source bundles declare their root, source files and optional source-library root.
The owned loader handles inline modules, file modules (`child.rs` or
`child/mod.rs`) and nested directories. Missing, ambiguous and duplicate inputs
have distinct diagnostics. Source spans and K functions retain file IDs;
diagnostics identify the original file. The host supplies bytes only.

Resolution uses module/declaration identities, lexical scopes, imports/aliases,
qualified `crate`/`self`/`super` paths and prelude priority. Private, `pub(self)`,
`pub(super)`, `pub(crate)` and `pub` visibility is checked for items, construction,
fields, methods and re-exports. Unsupported visibility forms are rejected. Local
declarations shadow prelude items and intrinsic spellings. Aliases preserve
nominal identity; identically spelled sibling declarations remain distinct.
Struct and enum construction follows the resolved declaration through transparent
alias chains and imports, preserving visibility, field privacy and evaluation
order. Constructor parameters have fresh bindings separate from the caller.
Explicit, omitted and inferred alias arguments can constrain fixed, reordered
and nested field types. A type alias does not introduce a tuple constructor in
the value namespace; a use alias can refer to the original tuple constructor.
Nested alias targets participate in the same structural type constraints as their
expanded constructors. An enum variant is a constructor or pattern, never a type;
resolved suffixes cannot reuse the nominal identity of their enclosing enum.

Module and enum glob imports preserve binding visibility. Explicit declarations
and imports shadow globs; unused glob ambiguities are tolerated and a conflicting
use is diagnosed. Public globs retain private binding restrictions instead of
making a private alias public. Function/block imports apply throughout their
lexical block, including nested blocks, and do not leak to neighboring functions.
Qualified module paths keep their module meaning. Anonymous trait imports
(`use Trait as _`) add method evidence without introducing a name.

Generic parameter binding uses its original lexical/file scope. It shadows an
unrelated declaration, while qualified paths keep their declaration identity.
Substitution, inference, coherence and implementation selection do not turn a
resolved nominal type or an intrinsic type such as str into a same-spelled parameter.

Types and instances use structural keys built from declaration/child-type IDs
and substitutions. Printable names are diagnostic labels with compact child IDs;
they are not expanded semantic identity keys.

Struct/enum fields have declaration order. Scalars and shared references are Copy.
Nominal types opt into Copy with `#[derive(Copy, Clone)]`, whose fields must all be
Copy; other values move even if their fields are scalar. Box and Vec own their
contents. Arrays contain their declared fixed number of elements. Partial moves
can be reinitialized. A type with custom Drop cannot be Copy or partially moved.
Custom Drop runs before owned fields, including on early return/break; temporary
receivers are cleaned at the end of their statement. Source cannot call Drop
directly. Traps terminate the program without cleanup or recovery.

References have local, parameter, or immutable static provenance. Mutable
references are exclusive; fields can be borrowed disjointly. A view aggregate may
contain independently rooted reference leaves. Each leaf retains its own source
and parent dependencies. A result declares possible input leaves or an immutable
static origin for every output leaf; exact maps preserve the selected input
referent while bounded maps permit its descendants. The checker requires complete,
unique output coverage and validates every possible source at calls and returns.
Source rigid T input-to-T transport preserves its abstract leaf origins; equal
concrete types alone do not authorize that correspondence. Named lifetimes and
repeated generic input candidates map to explicit alternatives. Exact transport
of a bounded input keeps its conservative footprint; it cannot forge exact storage.
Borrow endings and cleanup are explicit in K.
Whole-array/Vec slice projections, view structs and implicit mutable reborrowing
are implemented. Generic arguments declared as &[T] or &mut [T] can infer T from
an array or vector reference, including through transparent aliases. This is an
argument reference coercion; trait matching retains exact type equations. A bare
generic T still moves a mutable reference instead of creating an implicit reborrow.

Range indexing supports start..end, ..end, start.., .., start..=end and ..=end on
vectors, arrays, slices, str and the canonical source String. Base, start and end
evaluate once in that order, with the source borrow established before the bounds.
String uses its ordinary source as_str implementation. Shared and mutable slice
ranges preserve the original owner's lifetime and parent loans; mutable text
ranges remain unsupported. Nested ranges compose their extents. A range keeps a
conservative owner footprint even when empty; numeric bounds do not establish
that two mutable ranges are disjoint.

Exclusive ranges require start <= end <= length. Inclusive ranges require
end < length before adding one, then start <= end + 1. Text endpoints must also
be UTF-8 boundaries. Invalid ranges and text boundaries trap R0003. An inclusive
range such as 4..=3 at length 4 is empty; a maximum endpoint is checked before
addition and cannot wrap.

Every initialized reference leaf has checked source provenance. Inactive enum
alternatives carry explicit absence evidence. Moves, construction, assignment,
cloning and replacement preserve this inventory; return checks its completeness.
Replacement transfers the old value's provenance to its result and installs the
new value's provenance in the destination, including through local borrows.
Reference-field writes recheck the enclosing view's source and exclusivity.
Assignment to initialized borrowed/indexed owned storage uses checked replacement
and destroys the separate old owner; it does not move a hole into the container.

K does not yet express cross-parameter mutation contracts. Writes through an
external parameter may retain the exact symbolic origin of each reference leaf
or clear it, but changing that origin is rejected with `E_BORROW`. The comparison
includes projections, not just the parameter root. This conservative restriction
prevents a callee from silently invalidating the caller's lifetime assumptions;
explicit write contracts remain required for more general view mutation. A
reference-bearing write through an opaque call result additionally needs an exact
destination contract. Its conservative alias footprint is not a storage identity;
using it to clear reference leaves would erase sibling evidence. Reference-free
writes remain supported and preserve the other leaves and their tag witnesses.

## Inference and coercions

Each function definition has one constraint plan over lexical bindings and source
expression identities. Generic parameters are rigid under the definition's
stated assumptions. Structural constraints propagate through locals, assignments,
returns, known call signatures, tuples, arrays, Vec, Box, nominal fields, aliases,
projections and scalar patterns. A dependency worklist propagates known answers;
union/find represents equal type variables. Concrete instances consume the
recorded definition plan and instantiate its symbolic types.

Expected types flow into composite expressions. Integer suffixes fix their types.
An unsuffixed integer component defaults to i32 only after its surrounding known
constraints have propagated; its value must fit the selected width. Thus
`let mut v = Vec::new(); v.push(42i64)`, an index initialized with `let mut i = 0`
and later compared with `v.len()`, and `fn pair() -> (i64, i64) { (40, 2) }`
are accepted. Conflicting types or out-of-range literals report E_TYPE. There is
no implicit numeric conversion between fixed types; an explicit cast supplies it.
Unresolved generic arguments or receiver obligations still require enough source
context for the ordinary method/type gate and can report E_TYPE or E_UNSUPPORTED.
This contract covers these constraints; it does not promise all Rust inference.

A solved type is not itself a coercion site. Actual annotations, declared
reference parameters and explicit expected result types can authorize the
supported dereference/reborrow coercions. A generic T learned only from an
operand cannot silently change a move into a reborrow: Some(r), Holder { value: r }
and identity(r) move an otherwise unconstrained mutable reference r. Later use
then reports E_MOVE. An actual Option<&mut T>, Holder<&mut T> or &mut T result
context permits the corresponding Rust reborrow. Reference-bearing inference
hints preserve this distinction through transparent aliases and nested types.

## Diagnostics and census

The parser recovers at item, method and block-statement boundaries using balanced
tokens. It reports independent syntax errors and marks skipped regions as
E_BLOCKED. A recovered source graph never authorizes an artifact. A valid shared
declaration graph permits independent definition constraint errors to be
collected. A structurally valid K graph permits independent function-body errors
to be collected. Analysis of a failing body stops at its diagnostic; shared
signature/resolution failures and subsequent mutable K construction are explicitly
blocked instead of supplying invented types or ownership facts. The single-error
library API selects the first error from these same production paths.

`check --census` prints recognized unsupported syntax and blocked regions by kind.
It counts parsed constructs and unsupported parser diagnostics, not text matches
inside strings or comments. Receiver-dependent operations are not guessed from
method spelling. Regions skipped after an error are separate from counted syntax;
a zero in this table is not a claim that blocked source compiles.

## Immutable text

String literals produce shared UTF-8 text views with immutable module-long storage.
The unsized `str` referent is used through `&str`; bare or mutable text is not
implemented. Text provides byte `len`, `is_empty`, `as_bytes` and source-owned
`to_string`. Byte projections preserve the original static or borrowed origin,
including after a local reference carrier is destroyed. No byte mutation or owned
destruction can reach literal storage.

`std::str::from_utf8(&[u8])` validates bytes at the trusted runtime boundary and
returns the source Result with `Ok(&str)` or `Err(Utf8Error)`. The error is an
opaque marker; byte-position/error-length reporting is not implemented. Successful
conversion borrows the original storage. No unchecked source conversion exists.
String keeps its private Vec bytes and implements Clone, clone_from, PartialEq,
Eq, `push`, `push_str`, `to_string` and `as_str` in source. Cloning copies already-valid
bytes directly; clone_from reuses the target buffer when capacity suffices.
Equality compares byte length and contents. `as_str`
currently validates on conversion; retained UTF-8 evidence for avoiding that scan
remains future work.

Text and character helpers live in source. Supported text operations include
prefix/suffix tests, contains/find/rfind, ASCII case conversion/comparison,
repeat and replace. A source Pattern trait shares the implementation for text
and character needles. Character helpers include UTF-8/UTF-16 lengths, ASCII
predicates and case conversion, Unicode whitespace, and to_string. General
Unicode case conversion is not implemented. Range slicing follows the checked
view and UTF-8 endpoint contract above.

Formatting macros resolve argument expressions once in source order and lower
to ordinary calls into source String/Formatter traits. The supported format
language includes Display, integer x/X/b/o, positional/named/captured arguments,
escaped braces, fixed width, Unicode fill, alignment, signs, prefixes and zero
padding. Explicit zero width remains distinct from absent width for custom
Display implementations. write!/writeln! currently target source String and
Formatter; String requires Write in scope. Arbitrary user Write sinks remain
unimplemented. These macros preserve formatter errors and stop
before later formatting operations; format! traps on a failed formatting
implementation. Temporary values retain statement destruction order. Debug,
floating formatting, dynamic widths and precision remain unsupported.

Option and Result have canonical source declarations with standard option/result
module re-exports. Their source methods include shared/exclusive views, owned
and/or, and transpose. Option also supplies xor, zip/unzip, flatten, replace,
insert and get_or_insert. Owned arguments retain source evaluation and destruction
order even when their payload is not selected. Both types provide source
map, map_or, map_or_else, and_then, or_else and unwrap_or_else; Option also has
filter and Result has map_err. Eager defaults are evaluated before the call;
lazy defaults invoke their callable only on the selected alternative. Every
unused callable and payload still cleans up in source order. filter borrows its
payload for the predicate, then returns or destroys the original owned value.
Returned views preserve their owner and exclusivity requirements. Option::expect has the same value and trap
outcome contract as unwrap; its supplied message is evaluated but currently does
not replace the fixed trap text. Some nested generic receiver expressions still
need explicit type annotations before a method call.

Option additionally supplies consuming is_some_and/is_none_or predicates and
ok_or/ok_or_else conversion to Result. Predicates execute only for Some; the
None result is respectively false/true. ok_or evaluates its error argument
before the call, while ok_or_else invokes its factory only for None. Unused
arguments and captures retain ordinary function/branch destruction order.
Returned views retain the actual selected input or factory origin.
Option<&T> and Option<&mut T> additionally provide copied under Copy and cloned
under Clone. copied reads the payload without invoking Clone; cloned invokes the
trait operation exactly once for Some and never for None. The returned owned
value is independent where cloning makes ownership, while any references inside
that value still retain their actual external origins.

Vec shared access and search also live in source. is_empty compares length with
zero; get returns None for every index at or beyond length, including usize::MAX;
first uses the zero-index case. Their returned elements retain the collection's
shared owner footprint. contains requires PartialEq separately, borrows elements
and its query, and visits elements in order until equality succeeds. It invokes
eq rather than an independently overridden ne, and neither copies nor consumes
an element. Receiver/query/index expressions retain left-to-right evaluation.
extend_from_slice requires Clone, visits the shared input in order, and appends
one independent clone per element. Empty input performs no clones. It preserves
existing target elements and the source remains usable. A same-spelled inherent
clone method cannot replace the generic Clone contract.

Repeated Vec literals evaluate their element and count once,
clone all but the final element, and move the original last; a zero count still
evaluates and destroys the supplied value. This expands through a source helper
constrained by Clone. Ordered BTreeMap storage is a sorted Vec, with logarithmic
lookup and linear insertion/removal. BTreeSet reuses that map's ordering and
storage. Their shared iterators preserve sorted key order; maps also expose keys
and values. Map clear destroys entries in key order.

Shared collection iteration is ordinary source. Iterator declares Item and next;
IntoIterator declares an iterator type constrained by Iterator. The iter method on vectors and slices,
borrowed Vec/slice iteration and iterator values use those resolved
declarations. Collection for lowers to an outer match holding iterable temporaries,
a while loop, a next call and an irrefutable let binding for each item. Break,
continue, early return and ordinary completion preserve item and iterator cleanup.
Refutable item patterns and non-unit bodies are rejected. Numeric range loops use
the same binding subsystem. No iteration K operation or nominal-type shortcut is
required. Borrowed map/set iteration uses the same source protocols. enumerate,
skip, take and zip are source adapters; chain currently requires two identical
iterator types so Item equality follows from the type contract. Adapters retain
the underlying owners and release items/iterators on exhaustion or early exit.
map, filter, filter_map and inspect are ordinary source structs containing the
iterator and callable. Their next methods follow the underlying iterator on
every call, including a Some after a preceding None; they do not silently fuse
an iterator. filter and inspect borrow each item for a fresh callback input;
map and filter_map transfer ownership into the callback. Rejected filter items,
unused captures and partially consumed adapters retain ordinary cleanup order.
fold and for_each consume items in order and stop at the first None. The fold
accumulator is an ordinary owned value passed through each call.
any and all borrow the iterator mutably, pass each item by value to FnMut, and
stop at the first decisive predicate result or None. Empty input returns false
for any and true for all. The next call resumes after the consumed decisive item;
a temporary None does not permanently fuse the iterator. The predicate and its
captures are destroyed when the consumer returns; remaining iterator items stay
owned by the iterator. Receiver expressions execute before predicate construction.
Owned value expressions provide writable temporary receiver storage, including
projected fields and indexed elements. This does not change the mutability of
named bindings or permission through shared references.
Owned/exclusive collection iteration, Sum/FromIterator and the broader adapter
library remain separate work.

The explicit static-lifetime subset covers wholly static function outputs,
including view aggregates. K records `ReturnSource::Static` and checks every live
reference leaf. Static constraints on inputs, local annotations, fields, aliases,
bounds, or mixed static/dynamic source result annotations remain named unsupported.
Direct K can already describe mixed static/dynamic per-leaf result maps. Validation includes unused definitions and dead source.
Ordinary dynamic outputs may contain static literals with their lifetime shortened.
A named shared output lifetime with no source input is universal: elaboration
records an explicit static K contract and the body must satisfy it. Inactive enum
reference leaves satisfy that contract too. A call with no active input reference
leaves may still return static data; it does not prove the output leaves absent.

Ancestor-restricted visibility such as `pub(in super::super)` and
`pub(in crate::outer)` resolves once to a module identity in the declaring crate.
Only an actual ancestor can be named; imports and aliases do not redefine that
region. Fields, methods and re-exports use the existing access and widening
checks. External files and the source library retain their own module identities.

## Generic declarations and traits

Traits describe methods and associated types. Implementations must be unique;
overlap is rejected. Full implementation signatures, prerequisite/supertrait
bounds, associated types and instantiated arguments are validated. Send, Sync and
Data predicates are derived structurally; Data excludes capabilities. Generic
parameter-flow strongly connected components reject expanding instantiation.
Aliases participate in these checks. Bounds with extra type arguments remain
unsupported. Method calls resolve to concrete function applications. Callable
bounds such as F: FnMut(I::Item) prepare associated substitutions from I's declared
trait evidence before checking the callable signature. Generic parameter
source order does not hide that evidence; missing, ambiguous and incompatible
associated bounds still reject. Implementation prerequisite solving separates
input/output type equations, ordinary child trait proofs and full callable
signature validation. A pending or cyclic nonstructural goal is never a proof;
associated projections consume the checked child witnesses.

Inference indexes associated equations by the receiver variable, canonical
trait declaration and associated member. Same-spelled members of different
traits cannot become one rigid type. A unique structurally compatible
implementation can contribute conditional type equations, but those equations
provide no trait or lifetime proof. Ordinary selection still checks the
implementation's prerequisites. Unresolved caller types are not specialized to
make a candidate fit. Contextual method headers and primitive result contracts
supply argument shape; final method, callable and provenance checks still apply.

Generic function, impl-method and trait-default bodies are checked even when
never instantiated. Definition checking reuses production elaboration and K
ownership/type checking under rigid parameter identities and declared trait
contracts. It does not sample concrete parameter types. Bounds, associated-type
relationships, method signatures and explicit method type arguments are checked;
unreachable expressions must still type-check. User traits named Copy, Clone or
Drop do not acquire builtin semantics by spelling. Symbolic signature and
supertrait checks retain resolved trait identities, including imports and aliases.
Derived Copy on a generic layout is conditional on its fields.

Generic Clone::clone/clone_from and PartialEq::eq/ne calls have typed contracts
with checked receiver modes, arguments and results. Copy supplies Clone evidence;
Eq supplies PartialEq evidence. Ambiguous builtin and declared methods reject.
After successful definition checking, source expression identities retain their
selected trait or source method and exact receiver dereference count. Concrete
specialization verifies that selection against the actual receiver and compiles
the selected source implementation. A newly applicable inherent method cannot
replace a checked generic trait call. Rigid verification types/functions never
enter the executable program.

Equality operators evaluate operands once, left to right. Scalar values are read
before right-side effects; nominal owners are borrowed for the comparison.
Nominal == and != select the builtin PartialEq implementation, independently of
inherent methods or same-spelled user traits. An explicit ne executes when present;
otherwise the operator negates the selected eq. Reference operands of the same
type compare their referents. Comparison loans end with the Boolean result;
owned temporaries retain their ordinary statement cleanup.

Copy, Clone, PartialEq, Eq, PartialOrd and Ord derive declarations have their
structural field requirements checked even when unused. Generic derive checks use
rigid parameters with the required conditional assumptions, not a sample concrete
type. Supertrait evidence is required; duplicate derives and overlapping explicit
implementations reject. The current overlap rule is conservative about conditional
implementations, like ordinary implementation coherence. Unsupported derives,
including Debug, receive E_UNSUPPORTED instead of being ignored.

Explicit builtin Clone and PartialEq implementations on source nominal structs
and enums are checked against the
clone/clone_from and eq/ne signatures, including required methods, receiver modes,
argument/result types and generic arity. Eq requires PartialEq evidence. User
traits with the same names remain different declarations. Explicit builtin Copy,
Ord, PartialOrd, Data, Send and Sync implementations remain named unsupported;
Copy derives and the existing structural automatic predicates retain their rules.

Derived Clone and PartialEq execute as memoized monomorphic K functions. Source
and generated bodies share a pending-function queue and all bodies reach the
normal K checker, including those generated for definition checking. Headers are
registered before bodies, so recursive layouts reuse their function identities.
Field operations select the exact builtin trait implementation, with explicit
implementations taking precedence over derives and inherent methods excluded.
Structs, enums, tuples, arrays, nested vectors and boxed recursive layouts execute.
Clone visits fields in declaration order; equality checks both enum tags and
short-circuits over active fields. Eq has no executable method.

A nominal Clone derive with Copy and no type parameters copies the whole value,
including lifetime-only declarations. Type-generic derives invoke field Clone
even when a concrete application is Copy, matching the checked Rust subset.
Explicit Clone overrides still execute. Default nominal/tuple/scalar clone_from
uses clone, replacement and old-value cleanup; default ne negates eq, and scalar
eq/ne methods execute. Direct Box/array/Vec clone_from remains named unsupported
until element clone_from forwarding, vector reuse/truncation/append and their order
contracts are implemented. Default view-bearing clone_from requires a mutation
provenance contract. Source trait-default execution remains unfinished.

Vec<T> cloning uses the source helper constrained by T: Clone; each element
executes its selected Clone implementation. Vec equality requires T: PartialEq,
without Ord, and compares lengths followed by element == in order. Vec != and
ne negate that whole-vector result; they do not call element ne. Nested vectors,
source String, floats and explicit affine element implementations follow these
rules. Direct methods, operators and retained generic calls use the same source
helpers. Helper lookup names the actual library declaration, validates its
signature and checks the operation's element constraint independently of the
supplied helper header. Ordinary method lookup diagnoses competing visible
source traits. Vec clone_from default-body execution remains unsupported.

The emitted program remains monomorphic. The generic validation context names
declared evidence, but a complete independently replayable generic/witness K
contract is unfinished. Reference-bearing content hidden inside arbitrary type
parameters is not fully modeled at this generic boundary; concrete specializations
still receive full K provenance checks. This is not complete generic lifetime or
external mutation-contract closure.

Fixed input regions on callable bounds now have declaration-scoped proof
obligations in the discarded definition-verification program. Reference leaves
are followed through aliases, tuples and nominal fields. Each actual source must
outlive the required region using input-region facts and declared or implied
outlives edges; a local owner's lifetime cannot establish a universal input region.
Helper calls transport these requirements through their callable type arguments,
including fixed leaves of a type family. Explicitly fixed closure inputs do not
satisfy a fresh higher-ranked callable input.

A fixed callable input can retain a borrow independently of its result or the
callable's destruction. Retained effects are indexed by source root. Access
through a returned descendant requires the checked result's input-leaf selectors;
source overlap alone grants no permission. Shared access to stored reference
descriptors preserves the exact current holder while keeping its reborrowed
parent suspended. Reusing a call site revokes obsolete overlapping permissions.
Branches retain every possible effect and only permissions justified on each
relevant edge. Loop effects are replayed to a finite monotone fixed point; ordinary
initialization and loan backedges still require their existing equality contract.

These annotations are private diagnostic evidence. Text K cannot declare them,
and their checking API returns no executable authority. Mutable access to stored
reference origins still needs a mutation contract and is explicitly unsupported
at opaque calls. General output/capture origin families remain unfinished.

A named lifetime on a local type is checked at definition time, including unused
lifetime-only functions and rigid generic definitions. Each active reference
stored in the binding must outlive the declared region. This includes delayed
initialization, destructuring, aliases, fields, replacement and a later Some
after None. Retained loans survive moving, dropping or replacing the descriptor;
the current holder and its checked descendants keep their ordinary permissions.
An elided annotation permits the existing shorter reborrow. An undeclared
lifetime is a source error, including in unreachable source.

An annotation on borrowed descriptor storage constrains its earlier and later
contents. The ordinary checker observes exact storage and transfer identities;
a backward worklist propagates region requirements, then the private verifier
checks all reachable stores from entry. These observations only add restrictions.
Fresh reborrows must establish their issuing reference chain as well as their
referent origin. Moving or copying an existing descriptor preserves its original
contract. External reference-bearing writes retain the ordinary K mutation rule.

Typed views separately retain the declared regions of their visible reference
leaves. A shared view of a descriptor can shorten what is obtained through that
view without shortening the original descriptor. Mutable nested views preserve
invariance and constrain the stored descriptor as well. Requirements follow
checked value transfers, borrows, constructor fields and exact call-result
selectors. Named output types, elided outputs tied to their input or borrowed
receiver, and fixed callable inputs apply the same declared-view check. A longer
source origin alone does not override a shorter declared view. Bounded projected
call results and growing recursive projection equations remain explicit
unsupported cases; finite projection cycles use a visited worklist.
Explicit generic type arguments containing named reference regions, including
regions hidden in concrete aliases, currently require a checked type-instantiation
contract and report E_UNSUPPORTED. This boundary covers free and method calls,
qualified paths, constructors and patterns. Inferred arguments, scalar arguments and
explicit elided reference arguments retain their existing behavior.

Annotation metadata belongs to its compiling body and uses actual local,
projection and instruction identities. Discarded type probes publish no caller
metadata. A closure body can request a named region for an exact captured leaf;
every construction must prove that requirement from its actual capture source.
The body is checked under only those per-leaf input guarantees. CapturePlan remapping transforms both
places and metadata. Concrete type equality never establishes a source region.

Owning reference-bearing collections retain their existing unsupported boundary.
Named reference leaves inside borrowed slice elements are also rejected until
a collection reference inventory exists. No private source annotation can alter
the meaning or acceptance of a parsed K program.

## Closures

Closures are anonymous environment structs plus ordinary K functions. Their
capture requirements come from the body operations; source binding identity and
capture order determine fields, including disjoint field captures. Owned captures
move into the environment. Shared and mutable captures use existing loans, and
unique immutable access to a captured mutable-reference binding retains the
exclusive descriptor relationship. Capture destruction follows source capture
order; projections within one captured root follow field order.

Fn, FnMut and FnOnce select shared, mutable and consuming calls statically.
Generic callable bounds are checked under rigid assumptions; specialization
retains that selection. The declared callable input/output equations can infer
otherwise missing type parameters, including a result parameter used only by a
callable bound. These equations grant no new origin authority, and the completed
bound signature is validated separately. Inferred/contextual parameters, irrefutable parameter
patterns, nested closures and borrowed callable adapters are supported. The
environment has ordinary Copy eligibility and cleanup. Last-use cleanup may end
only its reference capture leaves while owned captures remain alive to their
lexical destruction point.

Inferred return types constrain expression shape without creating a reborrow
context. A bare captured mutable reference therefore moves out of an inferred
closure result and requires FnOnce. This distinction is preserved through
constructors, tuples, if/match, loop breaks and explicit return statements.
Declared return annotations keep their ordinary coercion behavior. An inferred
struct-update base supplies type equations, not an annotation for its fields.
Partially inferred return annotations still need per-position coercion evidence;
contextual FnOnce demands are not yet used to permit escaping reborrows of
captured references. These remain explicit reduced gaps.

Unique visible structurally compatible method headers can supply conditional
closure argument equations. Primitive methods use their shared typed inventory
before competing trait headers; ordinary selection still proves visibility,
prerequisites, inherent priority, ambiguity and callable kind. Ambiguous or
incomplete candidate domains supply no contextual evidence. Partial census
recovery does not turn an unresolved trait import into proof of a unique method.
Structural parameter patterns wait for the subject's reference shape rather than
forcing an unresolved associated Item into an owned tuple.

Ordinary shared or mutable closure calls do not lend references to owned
environment storage. Returned references must come from captured reference
origins, explicit arguments or static storage. Every result leaf has its own
contract. Return inference observes the ordinary checker's validated provenance,
narrows the initial allowed input set, then submits the resulting program to
independent complete K validation. These observations carry no execution
authority. Disjoint mutable result leaves remain disjoint across a call; shared
results may outlive a copied reference's environment while their referents remain
live. Generic and associated result families can likewise begin with explicit
per-leaf candidate bounds and refine from fully checked concrete body returns.
Acyclic callees refine before callers; recursive groups retain conservative
contracts. The final complete K check validates every resulting mapping. These
concrete observations do not prove an unused generic definition's hidden effects.
Explicit reference results hidden behind an opaque generic callable still
require a fuller origin inventory and remain a named unsupported boundary.
Returning closures through impl Trait, lending closures, dynamic function
objects and concurrency scope APIs remain unfinished.

An ordinary selected constructor establishes its tag even when it binds no fields.
Guarded matches retain their cached pure tests: a guard may consume an unbound
scrutinee, so only required binding projections reread its storage after a guard.
At a join, a reference template proven inactive on every feasible predecessor
remains absent after individual tag facts disappear. A holder active on any
incoming edge still participates in ownership and borrow checks.

## K and checking

Dropping a proven tree of scalar and reference descriptors can release its old
descriptor storage while descendants continue to refer to external owners.
Eligibility is computed once for the type graph and excludes every owned
destructor. Actual loans into the released storage still conflict. Replacement,
assignment and custom destruction retain their original parent restrictions.
The K machine checks the same programs independently.


K is a monomorphic program of explicit types, function signatures, local slots,
places, instructions and structured control. It distinguishes copies, moves,
borrows and drops. A function declares a parameter or immutable static source for
returned views. Literal instructions carry bytes that the K checker validates as
UTF-8, independently of source parsing. Text K serializes those bytes as hexadecimal
and static result contracts as `from static`.
A parameter result source carries an explicit reference projection path before
selecting its stored reference leaves (text K: `from 0 deref deref`). An empty
path selects the parameter itself. The path contains actual projection entries;
there is no encoded repetition count or numeric depth allowance. Projected
contracts currently require shared leaves;
exclusive reborrows retain their outer lifetime. The checker verifies returned
origins against the exact selected input leaves, then callers transport their
underlying provenance and parent dependencies. A local reference carrier can die
without invalidating a copied shared reference; its actual referent must remain
live. Each live reference leaf retains a nonempty set of possible source bounds
and a conservative footprint for access and lifetime checks. Direct origins are
exact; an opaque call may select descendants of its declared input sources.
Projecting through such a bound does not narrow it. A later return verifies every
candidate against the permitted source leaves, preserving generic forwarding of
multi-leaf views without accepting an unrelated owned field. Parent dependencies
survive calls and reborrows. Exact storage identity remains separate from the
footprint, and no positional input/output leaf correspondence is assumed.
Early source cleanup tracks immediate carrier dependencies through reference
creation, value transport, calls and control flow. A textually dead descriptor
remains live while another live value needs its storage. Dead reference chains
are dropped in dependency order; K independently verifies every emitted drop.
Concrete trait method selection is represented by an explicit function call.
Bounded slice/text views use ViewRange, written in text K as
`view-range DST shared|mut exclusive|inclusive START END PLACE`. Bounds are
separate consumed usize values; element and result reference types must match.
Every source root and parent loan survives with a bounded footprint. The K
machine independently tracks sequence start/count and checks UTF-8 endpoints;
common lowering computes inline data/count using existing scalar operations.
No range-specific runtime function or encoder operation is required.
K has a readable/writable text form accepted by `check` for direct tests.
Instructions preserve source lines and columns (`@line:column`; `@line` reads
as column 1). Ownership errors name the place, conflicting holder and loan
creation site; parameter-supplied authority is identified as a parameter.

Strict destruction (`drop` in text K) requires the named place to be initialized.
An already moved or dropped exact place, or an absent ancestor, produces
`E_MOVE` with the concrete place path. Structural destruction of a remaining
aggregate shell skips its moved children. A custom destructor requires every
field it may inspect to be initialized.

Conditional destruction (`drop-if`) skips an absent root or exact owned place.
Every projected ancestor must still be initialized; the operation cannot recover
a missing container or manufacture an owner. Both destruction operations reject
paths through a reference or an indexed collection under `E_BORROW`. Replacement
of such storage moves the old value through checked `replace` before destroying
that separate owner, keeping the destination initialized.

Shared lowering maintains initialization flags for roots used by conditional
destruction and clears moved owned payload slots. Consumed call arguments do not
remain initialized merely because their old bits remain in a physical slot.
K cannot supply a flag value. The independent machine tests its own cell state.
Conditional cleanup runs at the original destruction or replacement point;
it does not move destruction to a branch join. Initialization joins intersect
definite roots and union possible roots, with corresponding must/may projected
holes. A later read requires definite initialization; overwriting a possible
owner, strict destruction of a maybe-owner, projection through an absent shell
and leaking a possible owner all reject. Branch joins unite possible reference
holders, source bounds, parent dependencies and absence evidence. Loans are indexed
under every possible root. A later access must be valid against every possibility;
possible presence cannot justify a read. Writes that change reference inventory
need one exact destination identity, not merely a conservative alias footprint.
Loop backedges still require equality until an inclusion contract is implemented.

A by-value enum parameter initially carries reference templates for every
alternative. An exact checked tag witness excludes holders below the other
variants from conflicts. A borrowed witness does not prove that every possible
referent has the same tag. Tag invalidation restores conservative activity. At a
join, a holder moved on one edge and proven inactive on the other contributes
absence; losing the tag must not revive it. A holder live on any feasible edge
still participates in the join.

The checker validates all type references, signatures and operation shapes,
including dead code, then checks ownership through each function. Variant payload
access needs a proven tag. Moving, dropping or replacing a payload field preserves
the enclosing discriminant proof; replacing the enum, its tag witness or an
ancestor invalidates it, including predicates saved before a branch. Writing or
moving an index local used by the actual witness also invalidates that proof;
changing an old index does not retarget an already-created borrowed pointer.
A loop back edge must reproduce entry ownership state.
Return and break edges must carry correct live owners and loans; no fixed-point
iteration is used. Local storage or a local reference carrier cannot become the
source of an escaping view. The checker performs no source inference or trait
search. Only CheckedProgram reaches the backend; later stages rely on that gate.

## Memory and execution

Checked K lowers to target-independent typed operations with layout, calls and
copy/drop/equality glue explicit. The native encoder consumes this form without
consulting K. The independent K machine executes the checked representation directly.
Memory scalars use their natural one-, two-, four- or eight-byte widths; signed
loads extend correctly. Registers and pointers retain 64-bit representations.
F64 retains its bit representation and canonical NaNs after arithmetic.
Collection indexing checks bounds. Traps report a code and source line. The
source runtime releases owned fields, arrays, boxes and vector elements. Its
power-of-two size classes reuse released blocks in constant free-list work;
each block rounds payload plus a 16-byte header to a power of two, with a 32-byte minimum. Growth tries
geometric capacity and then the exact required extent. Structural cleanup uses
an explicit growable DFS stack, preserving custom-destructor-before-children and
declaration/element order. Moved fields are skipped. Copy helpers, recursive
generated Clone/equality calls and explicit recursive source calls use the selected
route's call stack. Loop-condition temporaries end before either loop edge,
including index locals and owned receivers.

The host supplies runtime source separately from application source. Runtime
preparation checks ordinary scalar K, validates the trapping primitive signatures,
and seals a small raw load/store/grow/copy/glue-call interface. Ordinary names or
imports cannot request those substitutions. Allocation, release, growth, the
cleanup walker and UTF-8 validation are source functions. The K compiler takes
this runtime as checked K input, without invoking the source front end.
The K runtime model executes those same functions against an independent raw arena;
the normal K machine keeps its separate typed heap as the application model.
The raw-memory invariants of the source allocator and glue still belong to the
trusted implementation; ordinary type checking alone does not establish them.

Native output is a static Linux x86-64 ELF without libc or a dynamic loader.
Build-host resource measurements never enter its bytes. Startup reads physical
memory (excluding swap) and RLIMIT_AS, then reserves stack and heap within the
available execution extent. Prologues check stack exhaustion (R0008); heap
mapping/growth refusal and allocation exhaustion report R0006. Standalone traps
write their code and source line to stderr and exit unsuccessfully. Linux may
still kill a process under physical/cgroup memory pressure; ordinary operator
budgets belong to execution, not source acceptance.

The runner supplies a default 256 MiB address-space budget for native children
and a 512 MiB budget for a fresh K worker, preserving tighter inherited limits.
LKJ_RUN_MEMORY_BYTES selects explicit bytes or unlimited. Some infallible Rust
allocations in the K host can still abort; the parent reports that host failure
without inventing a source location. Fallible K frame and heap paths distinguish
R0008 and R0006 at their instruction lines. The generator subprocess budget is
2 GiB for test executables and 8 GiB for rustc compilation/linking, configurable
with LKJ_TEST_MEMORY_BYTES; Node orchestration is
excluded from this address-space setting because its VM reserves a large cage.

Lowering reuses local storage across nonoverlapping lifetimes. Address-taken
values have stable frame slots; remaining live storage grows with the program.
Local-slot allocation is an optimization and introduces no acceptance limit.

Struct payloads align fields at their natural widths; enum payloads additionally
carry a tag. Vec and array storage has data, length and capacity words, with
element strides from the one layout contract. A Vec<u8> element occupies one byte.
References to slices and text are inline data/count pairs: 16 bytes aligned to
eight. Sized references remain one word; a reference to a stored slice reference
points to that pair. Aggregate fields and copies preserve both words. Whole-storage
projections read data and length directly, with no separate descriptor allocation.

The common lowering flattens view parameters into scalar words. A view result
uses a caller-owned two-word output slot supplied as the last lowered argument;
the callee stores both words before releasing its frame. Scalar-only calls retain
their existing ABI. Raw exports use that lowered ABI. The command-line host accepts
only unit/scalar entry results and either no arguments or one World capability;
internal calls can still return views. UTF-8 validation and byte output receive
data/count directly. Moved heap slots are cleared so partial cleanup skips them.
These layouts are implementation details, not an exposed source API.

Lowered functions retain `fn:<K-name>` entry identities. The independent K
machine executes checked K directly over its own heap, explicit call/control/drop
stacks, initialization states and exact storage/loan identities. It validates
reference tags and active variants on actual access and distinguishes M_* machine
errors from R* program traps. It shares neither the checker implementation nor
shared lowering. This dynamic search is not a proof or a full Rust memory
model. The host prints scalar entry results; floating display formatting is route
specific, while World output compares exact bytes.
No generated engine implements a step counter or per-operation allowance.

## Source library, trusted modules and imports

The source library implements Option/Result with conditional Clone, Copy,
PartialEq and Eq derives, byte operations, UTF-8 validation,
String, vector cloning and equality, ordering for integer/bool elements, and part
of the BTreeMap API. Vector cloning and equality execute ordinary source loops
using the element Clone or PartialEq contract; allocation/storage still use the
existing memory primitives.
BTreeMap uses sorted Vec entries with logarithmic lookup and linear insertion or
removal. String constructors validate bytes and its private representation is
protected across modules. The library loads through ordinary module scopes.
Bootstrap byte provisioning lives in `bridge/library.rs`; a compiler source
bundle supplies its own library files without hidden fallback.
`std::mem::replace` transfers ownership through an exclusive reference. Vec/Box
and memory operations remain trusted intrinsics until their source layer lands.

`compiler::compile(&SourceBundle)` is the bootstrap source-to-native entry.
The pure core additionally receives its declared Runtime; `kc::compile` consumes
program K, runtime K and an entry name without source-front-end dependencies.
`compiler::check_bundle` uses the same production checking pipeline. These entries
perform no host I/O. Their existence and input-mutation tests do not establish
self-compilation.

The production unsigned/signed LEB128 encoders live in `src/encoding.rs` and are
used directly by the emitter. That exact source also compiles and executes under
lkjscript, with byte-level boundary vectors and behavior-changing source mutations.
The same production canonical-name helper in `src/names.rs` is used by resolution,
generic validation and elaboration, and executes with the source String library.
Its tests compare complete output bytes and require valid/invalid source mutations
to change behavior or produce an attributed diagnostic. These are executing
compiler components; the full dependency closure and fixed point remain unfinished.

`src/json.rs::write_string` is the production emitter's JSON string encoder. It
executes through ordinary source compilation using byte loops, vector operations
and integer range patterns. Tests compare every output byte, independently decode
JSON, inspect actual emitted trap metadata and mutate a range boundary. The old
emitter-local quoting loop has been removed; K text keeps its distinct quoting
semantics.

The complete unchanged `src/k.rs` production representation module also executes
through ordinary source loading, with its real Clone/equality operations and
source library. Probes cover all Op and TypeKind variants, nested Program/Function
data, Option<ReturnSource>, projections and access after original owners are
dropped. Rust executes the same module/probe bytes. Changes to primitives and
Place::local alter artifacts and results; an invalid body reports src/k.rs. These
are representation operations on K data, not execution of that data as a checked
program or a compiler-core fixed point.

`src/check.rs` is the K checker; `src/k.rs` defines its inputs and checked result.
`src/lower.rs` owns layout, glue and the explicit lowered operations.
`library/runtime.rs` owns source runtime algorithms; `src/runtime.rs` seals and links
the raw interface. `src/native.rs` encodes the lowered program as static ELF.
`bridge/` supplies bootstrap file/process/command-line access and declared runtime
and library source. `lkjscript trusted` lists these boundaries; `tools/trust.mjs`
reports their physical sizes.

Generated modules have no ambient imports. A module using a checked Stdout value
imports only the byte writer. Language traps record a code and source line in
lowered globals and terminate the selected route; the host translates that
record after the trap. Trapping requires no application I/O authority.
Ordinary source cannot declare host imports or construct capabilities from scalars.
Ambient filesystem, environment and print operations are rejected. World-derived
standard output is specified below. Other capabilities and structured
task scopes remain unimplemented.

## World and byte output

The host entry alone creates `World`. An entry is either a zero-argument function
or a function taking exactly one `World`; ordinary source calls still supply their
own arguments. `World` and `Stdout` are affine intrinsic types with no source
constructor, Copy, Clone, Data, Send or Sync implementation. Structs merely named
World or Stdout do not acquire capability authority. K cannot construct a
capability with an integer, cast, unit or aggregate instruction.

`World::stdout(&mut self) -> Stdout` derives authority to write standard output.
A writer may outlive the World borrow and may be moved or passed by mutable
reference. Repeated derivation is permitted. `Stdout::write(&mut self, &[u8])`
returns unit after the complete byte sequence has been written, in call order.
Only checked `world_stdout` and `stdout_write` K operations grant and use this
interface. Source code cannot name a host import or forge either operation's
required value. Pure programs do not import the output operation.

The native host boundary receives a byte pointer, length and source line. It
checks the range without overflowing, handles partial writes and EINTR, and
retains no bytes or pointer after returning.
Byte storage is already contiguous under the common layout contract; the host
API does not depend on a source aggregate representation. Zero-length writes produce no bytes. An invalid range, zero-byte
write or operating-system write failure traps with `R0007` (host output failure).
Output emitted before any trap remains observable. Traps do not unwind; this is a
specified departure from Rust panic unwinding. Normal destructor order remains
Rust-compatible in the supported intersection.

The rustc conformance adapter supplies the same types and methods, privately
constructs World in its host wrapper, uses write_all and aborts on output failure.
Conformance compares exact bytes and normal exit versus the specified trap kind;
matching bytes followed by a crash is not a passing execution.

## Diagnostic recovery and test oracles

Once signatures are valid, a body that passes definition inference proceeds to K
construction and ownership checking even if a peer fails. Diagnostic-only trap
bodies retain known signatures for dependent checks; the saved error list
prevents such a program from becoming an emitted artifact. Unsupported source
forms are enumerated across parsed regions. The census separates syntax sites,
blocked regions and observed name/type/method/trait dependencies, with phase
coverage and dependency blockers reported separately. It retains real source
declaration identities and uses an indexed dependency worklist to recover
independent bodies. A known field type remains available when a different field
blocks its enclosing layout. Source type observations never fabricate that layout.

Closed structural bodies with complete nongeneric dependencies can reach ordinary
K construction and ownership checking in the diagnostic pass. Unknown nominal,
generic, callable, constant or region contracts remain ownership blockers.
The public recovery result contains observations and locations only; it cannot
supply checked K or authorize emission. Missing modules and excluded implementation
domains prevent method-absence claims. An unresolved import can affect trait
visibility, but it cannot add an implementation to an otherwise complete supplied
source graph. Existing candidates whose visibility or prerequisites are unknown
remain blocked. A method's spelling alone is not evidence that it resolves.

The bootstrap batch command accepts native, K, or both. It checks each source
once. K uses fresh machine state per case inside its bounded worker; native cases
execute separate images. Every case retains its own bytes and termination class.
The batch watchdog measures missing progress, not total duration. Missing,
duplicate or incomplete results fail the suite. Rust reference outputs are cached
by source, seed, compiler version and flags. Semantic cases are source/K data;
source-sensitivity bootstrap tests remain while their exact source identity is
relevant. Detailed measurements remain under target/.

## Consumer-driven surface additions

Non-test compilation evaluates cfg(test) as false, including nested all/any/not
conditions; excluded declarations never reach name resolution. Derived Default
is an ordinary generated implementation using field defaults under declared
trait bounds. Text equality and string-literal patterns follow Rust's reference
matching depth. String references coerce to str at declared argument and receiver
sites. Byte-string literals have static shared references to exact-size byte
arrays; the K constant contract checks their type and extent.

The source library provides String ordering/from conversion, scalar ASCII tests
and min/max, owned Vec iteration, stable sort and retain. Source bodies specify
method behavior and destruction order. General From/Into evidence and exclusive
iter_mut still need their real generic/provenance contracts; they are not claimed
by the implemented conveniences. Arrays and Vec/Box of views still reject with
located alternatives: owned values, indices, or separate local views.

The ordered map uses an owned AVL node arena with index links and free-slot reuse.
Insertion, lookup and removal follow tree height; ordered iteration and clear/drop
preserve key order. This is library source, not a new trusted primitive.
