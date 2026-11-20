# Language

lkjscript has a Rust-shaped surface over independently checked monomorphic IR.
Programs both compilers accept behave the same. [Status](status.md) records gaps;
[IR](ir.md) defines the admission boundary. Executable reference examples live in
[tests/reference](../tests/reference/), with expectations checked by the native gate.

## Scalars and evaluation

| Form | Meaning and lowering | Rejection or rewrite |
| --- | --- | --- |
| Integer literals/types | Unsuffixed literals default to i32 after constraints propagate. Signed/unsigned 8/16/32/64-bit types, usize and isize remain distinct. Signed minima, radix and suffix rules apply to patterns too. | An out-of-range literal or conflicting fixed type is E_TYPE; choose a wider type or explicit cast. |
| Integer operators | Checked arithmetic; wrapping_add/sub/mul operate modulo the receiver width. Source checked_add/sub/mul return Option for all ten types. Operands execute once, receiver before argument. | Ordinary overflow traps R0001. Wrapping/checked methods require matching operand types. |
| Shifts | Counts may have another integer width; validate sign and width before narrowing. | Negative or excessive counts trap; choose a valid count explicitly. |
| Primitive references | Arithmetic, bitwise and unary operators forward one shared reference to a scalar. | Further indirection requires explicit dereferencing. |
| bool and char | Distinct types; char is a four-byte Unicode scalar, ordered by scalar value. Cast char to integers or u8 to char; char::from_u32 returns Option. | Surrogates/values above U+10FFFF reject; other integer-to-char casts and character arithmetic reject. Use from_u32. |
| f64 | Finite literals; explicit numeric casts follow Rust. Binary arithmetic canonicalizes NaNs; negation flips only the sign bit. | Nonfinite literals reject; construct special representations with from_bits. |
| f64::from_bits(u64), value.to_bits() | Preserve every bit, including signed zero, signaling state and NaN payloads, through dedicated checked operations. | Wrong receiver, argument/type-argument count or type rejects. Use casts for numeric conversion. |

Expressions execute left to right; logical operators short circuit. Assignment
and primitive compound assignment evaluate RHS before destination/index expressions,
then compound assignment reads the old value. Each operand executes once. Empty [] needs an element type: use a [T; 0] binding, or &[0u8; 0] for &[u8].
Traps terminate without cleanup or recovery; there is no unwinding.
Cases: [scalar bits](../tests/reference/scalar_bits.lkj), [evaluation order](../tests/reference/evaluation_order.lkj).

`std::convert::{From<T>, Into<T>}` are ordinary source traits. Identity `From<T>`
and the blanket `Into<U> for T where U: From<T>` retain the exact applied input
type. Lossless `u8` conversions cover the wider integer types and `f64`.
A unique trait implementation whose receiver is exactly its own type parameter
can supply a conditional method header for a rigid generic receiver, after
written bound methods have been considered. Argument and result context can
then infer the complete applied implementation, as in `value.into()` under
`U: From<T>`, while ordinary selection must still prove every prerequisite.
Concrete receiver implementations cannot supply an undeclared generic
interface, and competing blanket candidates remain unresolved. Trait function
items written without their receiver type, such as `Into::into` in a higher-order
argument, remain a separate located inference boundary.

`std::ops::{Shl<Rhs>, Shr<Rhs>, Div<Rhs>, AddAssign}` provide source contracts for
`<<`, `>>`, `/`, and `+=`. Generic bodies must establish the canonical library
trait and its exact RHS and output types even when unused. A same-spelled local
trait or inherent method cannot grant the operator. Primitive operations keep
their checked scalar lowering; admitted source operators become ordinary checked
calls. A generic definition's trait recipe survives specialization even if
unrelated type arguments happen to be equal in one instance.

Binary operator calls evaluate their left operand before the right operand.
Source-trait compound assignment evaluates the destination expression before
the RHS and retains its ordinary exclusive receiver borrow, including when a
generic body later specializes to a primitive type. The independent checker
still verifies the call, moves, borrows, and destruction. Operator query edges
name the canonical trait method at the punctuation token; that token is not an
editable method name. Checked body edits must prove the replacement contract.
Macro-generated operator dependencies retain their expansion origin and
invocation anchor, with the ordinary generated-source edit refusal.

The source primitive implementations cover all ten integer types for
`Shl<usize>`, `Shr<usize>`, same-type `Div`, and same-type `AddAssign`, plus `f64`
division and addition assignment. This slice admits bare `AddAssign` with its
standard `Self` RHS. General trait parameter defaults, bare `Div`/`Shl`/`Shr`,
explicit heterogeneous `AddAssign<Rhs>`, and additional primitive conversion or
operator trait applications remain located source boundaries. Reference-bearing
operator trait arguments also require a source region recipe and are explicitly
refused until that proof is available; their identity cannot be reconstructed
from erased reference types. Primitive shared-reference forwarding and the
existing primitive shift-count widths retain their ordinary scalar semantics.
Constant blocks retain the existing primitive operand checks even when the RHS
has no definition-wide inferred type. A known nominal RHS still requires its
exact source operator application; generic recipes never become primitive
operations merely because a later instance uses scalar types. Source operator
calls remain outside the supported const-function contract.
[Bit-accumulation case](../tests/conformance/pass/numeric_traits_bits.lkj),
[evaluation-order case](../tests/conformance/pass/numeric_traits_compound_order.lkj).

## Statements, control and patterns

Functions, structs, enums, module constants, aliases, generics, trait implementations,
match, if, while, loop and for are implemented. Assertions, panic and vec forms
are fixed syntax. Literal panic messages and plain named captures of unique immutable outer-block literal bindings become static trap text. Other panic payloads are not evaluated: write format!(...) bytes through an already granted output capability before panic!(). Local const declarations reject; move the declaration to its containing module.

An inherent implementation on a nongeneric struct or enum may declare `const`
members. A value path such as `Ready::EMPTY` selects the member of that exact
nominal declaration and checks its visibility. An import can rename the owning
type, and `Self::EMPTY` works within its inherent implementations. `Self` in the
constant's annotation or constructor denotes the owning type. Initializers admit
scalar literals, tuples, data constructors and paths to other inherent constants.
The initializer checks in its declaring module even when unused. Cycles are
rejected. Each use constructs the value with ordinary IR, including inside a
const function. The catalog records value identities such as `value@Ready::EMPTY`.
Trait constants, generic owners, type-alias owners, constant patterns and calls
that require compile-time evaluation remain source boundaries.

Free functions and inherent methods may be declared `const fn`. Ordinary runtime
calls use the same concrete functions and checked IR as other calls. Every
const-qualified body is verified, including unused definitions and unreachable
expressions, under its declared generic assumptions. Data constructors, moved
inputs, field access, shared views, matches, scalar operations and local control
flow are supported. Calls must select another declared const function or a data
constructor; a pure implementation of an ordinary method does not make it const.
Implicit destruction must be structurally trivial. Moving a non-Copy or generic
input into the result is allowed; discarding an unconstrained generic input,
heap owner or value with a user destructor is refused. References do not destroy
their referents, and a Copy bound can establish trivial generic destruction.
Trait const methods, generic trait calls, callable/closure invocation, try,
intrinsic calls, allocation, mutable borrowing and mutation through references
remain located limits. Source queries retain const qualification, and checked
renames/body edits preserve it; changing a callee to non-const is rejected when
a const caller depends on it. Function calls in module constant initializers
remain a located compile-time-evaluation limit, including scalar calls; general
constant aggregate/reference evaluation is not claimed.
[Const-function cases](../tests/conformance/pass/const_function_construct.lkj),
[queries and edits](../tools/const-functions-test.sh).

Unparenthesized block forms in statement position end before a binary operator,
call or index that can start another expression; dot and ? continue the expression.
Parentheses, initializers, arguments, returns and operator RHS use value parsing.
Match arms follow this boundary and need commas after grouped/postfix expressions.
Implicit block statements must return unit or diverge; a semicolon discards any
value with normal cleanup. Unreachable statements, tails and definitions still check.

if let/while let lower to match/loop. let else requires a diverging failure branch
and retains borrowed initializer temporaries through the binding scope. Option/Result
? lowers to early return; Result errors keep their type or convert &str through
the canonical String constructor. General From evidence is unsupported.
Loop break values unify across exits. continue advances a for iterator; an inclusive
maximum endpoint does not overflow. Struct update evaluates explicit fields in
written order, then its base once, transferring only the remaining fields.

Integer/byte/character patterns accept inclusive, half-open and open literal ranges.
Empty/reversed ranges reject. Exhaustiveness unions intervals over the concrete
domain, including signed minima, u64::MAX and the Unicode surrogate gap; it never
enumerates the domain. Runtime/named endpoints need ordinary guards instead.
Every arm checks, including arms after a default and unused generic bodies.
Scalar reference matches read the referent and preserve mutable reborrows.

Nested tuple/struct/enum patterns, reference binding modes, or-patterns and guards
run in source order. Failed tests/guards do not move payloads. Guard bindings,
including Copy bindings, retain shared loans: guards cannot move affine bindings
or mutate a tested place/ancestor; unrelated untested fields remain writable.
A wildcard does not inspect storage. Pure tests are cached; overlapping alternatives
retry guards in order. Shared pattern trees/continuations avoid static Cartesian
expansion, though effectful guards may visit many feasible choices.
A guard may consume an unbound owner: later discriminant-only tests use cached facts,
but later payload access still needs ownership. Internal lowering loops use checked
transfer depths; invalid depths reject even in unreachable IR.

Enums retain unit, tuple and named-field shapes. Named initializers execute in
written order and store in declaration order. Tuple/rest patterns select typed
prefix/suffix fields; unannotated lets preserve initializer binding mode.
Named patterns accept reordered/renamed/shorthand fields, wildcards and rest.
Names, completeness, duplicates, types and shapes all check. Unit/empty tuple
variants accept empty braces; nonempty tuple numeric-brace syntax is unsupported.
Enum fields inherit enum visibility; explicit field visibility is invalid.

ref/ref mut binds original storage; nonmoving matches preserve owner, mutability
and destruction timing. Distinct fields of one proven variant permit exclusive
borrows; other tags/same fields do not. Existing-place matches may move affine
fields and borrow disjoint fields together. Projected tag tests retain owner cleanup.
A selected constructor establishes its tag even if the pattern binds no fields.

## Source graph and names

The owned loader reads inline/file modules, child.lkj or child/mod.lkj, and nested
directories through the source Directory. Missing, ambiguous and duplicate inputs
have distinct diagnostics. File identities survive into IR and diagnostics.
A reserved keyword in a module declaration produces `E_PARSE` at the name token
before the loader looks for a module file.
include_str!("path") produces static UTF-8 text; include_bytes!("path") produces
a static reference to an exact-size byte array. Paths are literal, relative to the
containing file and beneath the granted source root. Missing/escaping paths reject;
invalid UTF-8 needs include_bytes. [Include case](../tests/reference/include.lkj).

Resolution uses declaration/module identities, lexical scopes, aliases, qualified
crate/self/super paths and prelude priority. Local declarations shadow prelude and
intrinsic spellings. Private, pub(self), pub(super), pub(crate), pub and ancestor
pub(in PATH) visibility applies to items, construction, fields, methods and re-exports.
Ancestor restrictions resolve once to actual ancestor modules; aliases/imports
cannot redefine the region. Unsupported/widening visibility rejects.

Aliases preserve nominal identity; sibling declarations with equal spelling remain
distinct. Struct/enum constructors follow transparent alias/import chains, preserving
visibility and written evaluation order. Constructor parameters bind separately
from caller parameters. Fixed/reordered/nested alias arguments participate in normal
structural constraints. Type aliases create no tuple constructor in the value
namespace: use the original constructor or a use alias. Variants are constructors
or patterns, never types; suffixes cannot reuse an enclosing enum's type identity.

Explicit declarations/imports shadow module/enum globs. Unused glob ambiguity is
allowed; conflicting use is diagnosed. Public globs preserve private binding
restrictions. Block imports cover their entire lexical block and nested blocks,
not neighboring functions. Qualified paths retain module meaning. use Trait as _
adds method evidence without a name. Generic parameters retain original lexical/file
scope and shadow unrelated declarations, not resolved nominal/intrinsic identities.

Types/instances use structural declaration/child IDs and substitutions. Printable
names, including compact child IDs, are labels rather than semantic identity.
Non-test compilation evaluates cfg(test) as false, including all/any/not; excluded
declarations never enter resolution.

## Ownership, storage and views

Scalars/shared references are Copy. Nominal types require a valid Copy derive;
otherwise even scalar-only aggregates move. Box/Vec own contents. Arrays have fixed
length: repeats evaluate the seed once, copy it without Clone for counts >=2,
move it for one, and evaluate/destroy it for zero. Repetition is compact IR/native
loop work; interpreted elements are independent. Repeated Vec literals instead
evaluate element/count once, Clone all but the final element and move it last;
zero still evaluates/destroys the seed.

Partial moves can be reinitialized. Custom Drop excludes Copy/partial moves,
runs before owned fields and executes on normal early return/break. Source cannot
call Drop directly. Fields/elements destroy in declaration order; moved fields skip.
Owned temporaries last through their statement; loop temporaries end before edges,
including indexed locals and owned receivers. Traps bypass all cleanup.

References have local, parameter or immutable-static origins. Mutable references
are exclusive; proven distinct fields may be borrowed together. View structs/enums
may live in locals, parameters and results. Box, Vec and owned arrays of views,
including nested view aggregates, reject: use owned values, indices or separate views.
Each reference leaf retains its own possible sources and parent dependencies.
Moves, construction, assignment, clone and replacement preserve the complete leaf
inventory; inactive enum alternatives carry checked absence evidence.

Whole-array/Vec slices, view aggregates and implicit mutable reborrows are supported.
Declared &[T]/&mut [T] arguments can infer T from array/Vec references, including
aliases; trait matching still uses exact equations. A bare generic T moves a
mutable reference. Actual reference annotations/parameters/results can authorize
coercion; inferred type equality alone never converts a move to a reborrow.

| View operation | Meaning and checked lowering | Rejection or rewrite |
| --- | --- | --- |
| Range indexing | start..end, ..end, start.., .., start..=end and ..=end on arrays, Vec, slices, str and canonical String. Base/start/end execute once in order; establish the source loan before bounds. Nested ranges compose extents. | Exclusive requires start <= end <= length. Inclusive checks end < length before addition, then start <= end+1. Invalid bounds or non-UTF-8 text endpoints trap R0003. |
| Mutable ranges | Preserve owner/parent loans and conservative footprints, even when empty. | Numeric bounds do not prove two mutable ranges disjoint. Mutable text remains unsupported. |
| Returned views | Every leaf declares possible input leaves or static origin; exact maps retain selected referents, bounded maps permit descendants. Complete unique coverage and every candidate check at calls/returns. | Concrete type equality does not establish generic T-to-T origin transport. Bounded footprints do not create exact storage identities. |
| Reference-bearing replacement | Returns old provenance and installs new provenance; initialized borrowed/indexed storage stays initialized while the separate old owner is destroyed. | Reference-field writes must preserve enclosing source/exclusivity contracts. |
| External parameter writes | May preserve each leaf's exact symbolic origin, including projections, or clear it. | Changing origins is E_BORROW until cross-parameter mutation contracts exist. Use explicit local views/owned values. |
| Writes through opaque results | Reference-free writes preserve sibling leaves/tag facts. Reference-bearing writes require an exact destination identity. | A conservative alias footprint cannot authorize clearing sibling reference evidence; mutate through the original exact place. |
| Holder lifetime | A reference stored in a live view aggregate may retain its loan despite its last apparent source use. Temporary source views also last through match arms or a self-slice assignment. | End the carrier's lexical scope before exclusive access. Evaluate an owned match scrutinee or to_vec copy as { let saved = expression; saved } to end its temporary source view first. Mutable method receivers borrow before arguments: read a scalar self.field argument into a local before calling the method. Before returning a sliced owner, compute a scalar result inside { let view = &owner.bytes[range]; view.len() }, then assign it outside that scope. |

Cases: [ranges](../tests/reference/range_views.lkj), [UTF-8 trap](../tests/reference/text_boundary.lkj),
[holder rejection](../tests/reference/holder_liveness.lkj), [scope rewrite](../tests/reference/holder_scope.lkj).

Wholly static outputs, including aggregates, lower to Static result contracts
checked on each live leaf. A named shared output lifetime without a source input
is universal and also requires static origin. Inactive leaves satisfy that contract;
absence of active input leaves does not prove output absence. Ordinary dynamic
outputs may shorten static literals. Explicit static input/local/field/alias/bound
constraints or mixed static/dynamic source output annotations remain unsupported; own a text label as String instead of a &'static str field, take the constructor input as &str, and construct the owned field with label.to_string().
Direct IR can express mixed per-leaf maps. [Static view](../tests/reference/static_view.lkj).

## Inference, generics and traits

**Default type arguments.** Named structs, enums, and type aliases may give
trailing type parameters defaults, including earlier parameter references:
`struct Pair<T = i64, U = T> { left: T, right: U }`. Omitted arguments expand in
the declaration's scope before type interning, alias normalization, inference,
coherence, and implementation selection. `Pair`, `Pair<i64>`, and
`Pair<i64, i64>` therefore have one application identity. A caller's unrelated
parameter with the same spelling cannot replace a declared default.
Concrete nominal defaults check their well-formedness and concrete input bounds
even on unused declarations. A default or bound depending on another parameter
retains its obligations at each actual application; `struct Pair<T, U: Copy = T>`
does not imply `T: Copy`, and omitting `U` still requires that concrete argument
to satisfy `Copy`. Self/forward parameter references, nontrailing defaults, and recursive expansion through
nominal defaults or aliases are located errors; explicit arguments that break
an omission cycle remain finite. The source AST retains each written default
for signatures, references, and checked edits. Generated implementation binders
clear those defaults, and strict Default/Debug origin validation rejects copied
default metadata on those binders. Nested declarative macros retain captured
default types and the distinction between caller names and `$crate` paths;
authenticated replay includes the exact default TypeIds. Generated declarations
keep generated query origins and refuse source-projection edits. Authored
defaults remain editable, and changing a macro's captured default invalidates
its derived epoch. Struct/enum/alias parameter renames use exact declaration
scope references and refuse capture. This is frontend substitution into the
existing checked types and calls; the independent IR checker is unchanged.
Default syntax on functions, implementations, traits, or associated families,
const/lifetime defaults, and named defaults combined with const/lifetime binders or
nonparameter where predicates remain explicit `E_UNSUPPORTED` boundaries.
Dependent defaults that require deferred nested well-formedness and bounded or
unsized alias defaults also retain located `E_UNSUPPORTED` boundaries.
[Default argument cases](../tests/conformance/pass/type_defaults_nominal.lkj).

Definition checking solves structural constraints over lexical binding/expression identities using rigid generic parameters and declared assumptions; specialization uses the checked plan.
Assignments/returns, signatures, composite literals, fields, aliases, projections and scalar patterns check. Expected types flow inward; unsuffixed integers default after known constraints propagate.
Unequal byte-string branches passed to &[u8] need &b"..."[..] views. Generic/receiver context may require annotations: use BTreeSet::<String>::new() for owned text before later insert calls.
Complete Rust inference is not promised. [Inference case](../tests/reference/inference.lkj).

A solved type is not a coercion site: Some(r), inferred generic fields and identity(r) move unconstrained mutable r; reuse is E_MOVE.
Explicit Option<&mut T>, view-field or result expectations can authorize a reborrow; pass &mut *argument for repeated generic mutable arguments. Aliases/nesting preserve this distinction.

Traits specify methods/associated types; implementations must be unique. Signatures, prerequisites, supertraits and instantiated arguments check.

**Positional trait type arguments.** Declared source traits can bind named type
parameters, as in `trait Parser<Input> { type Output; }`. Bounds, implementations,
qualified members, inherited contracts, associated equalities, and opaque
`impl Trait` results preserve the complete application. `Parser<u8>` and
`Parser<bool>` are distinct witnesses even for the same receiver. A blanket
implementation can bind a parameter through the trait argument as well as through
its receiver. Trait type parameters check their declared bounds and implicit
`Sized` requirement; `?Sized` permits an explicitly unsized argument.

Generic bodies are checked with rigid argument identities, including unused
bodies. Sibling associated equalities refine applied arguments independently of
parameter order; the corresponding proof identity is updated before use. A recorded method keeps a source substitution recipe for its applied
trait, so specialization cannot replace that evidence with a different argument
or a same-spelled inherent method. Provided bodies substitute trait parameters
independently from implementation parameters. Supertrait cycles and recursively
expanding proof arguments are refused without a search-depth threshold.
An implementation parameter can also be inferred from an exact prerequisite's
associated equality, as in `P: Parser<I, Output = O, Error = E>`. The receiver and
trait arguments must be known before that prerequisite is proved and its
associated values constrain `O` or `E`. Callable and equality constraints reach
a fixed point; every completed implementation still checks all prerequisites,
equalities and where predicates. These equations neither choose between
ambiguous implementations nor invert an associated family to guess its inputs.
Declared method signatures on rigid parameters also propagate their result
types into subsequent closure captures. A concrete specialization cannot supply
a missing declaration-time method contract.
Unsized reference coercions wait for unresolved producers through match/block result
constraints; a borrowed-slice parameter does not choose an unsized owned local.
This also retains the declared `Option<String>` result through `as_ref().is_some_and()`
on a generic trait receiver, and the owned `String` element of a Vec loop before
its later `&str` argument coercion. The original diagnostic bodies now have
positive [closure-context](../tests/conformance/pass/generic_option_closure_context.lkj)
and [loop-element](../tests/conformance/pass/vec_loop_element_inference.lkj) witnesses.
[Prerequisite witness case](../tests/conformance/pass/trait_equality_witness.lkj).
Trait lifetime/const parameters remain separate located source boundaries. Written reference arguments, arguments through
lifetime-parameterized nominal types/aliases, and projections that normalize to
views require a region-aware witness identity and are refused. The existing
region-erased IR type identity cannot prove that two written lifetimes denote the
same trait argument. Ordinary type parameters can still carry borrowed values
through uniformly generic implementations; no lifetime-specific witness is inferred. [Positional trait cases](../tests/conformance/pass/positional_trait_identity.lkj).
Concrete method arguments and result context can also determine implementation
parameters absent from the receiver, such as `Input` in a tuple parser's
`Parser<Input>` prerequisites. Definition inference retains those equations with
the exact expression, implementation, and method identities; selection proves
every completed prerequisite before using the candidate. Unresolved or competing
candidates do not become witnesses. The completed application supplies no new
argument coercion site: an inferred mutable-reference argument still moves unless
the source establishes a reborrow. Checked specialization, references, and method
renames retain the same application. [Tuple dispatch](../tests/conformance/pass/positional_method_tuple.lkj).
Recorded concrete applications also retain method-local type arguments fixed by
result or chained-call context, such as `parse`'s `T` in `text.parse().ok()`.
Final instantiation checks the method's written bounds. An unresolved method-local
argument may still be inferred from its value expression without discarding an
already complete implementation application. [Result context](../tests/conformance/pass/method_result_arguments_chain.lkj).

**Generic associated types.** A declared family such as `Mode::Output<T>`
keeps its receiver, complete trait application, member, and ordered family
arguments through definition checking, inference and method replay. Qualified
`<M as Mode>::Output<T>` and unambiguous `M::Output<T>` resolve the same family.
A concrete implementation normalizes its RHS under separate implementation and
family binders; no family operation or assumed witness enters checked IR.
The type-parameter form supports methods, aliases, nested associated owners,
parameter bounds and result bounds, including nom-style `OutputMode`/`Mode`
selection. Family results are implicitly Sized; a named parameter may use
`?Sized`. Equal family outputs never infer equal receivers or arguments.

Every implementation family is checked with rigid parameters even when unused.
Its written bounds must establish the declared parameter contract and may not
strengthen it. Its RHS and result bounds must follow from those written bounds;
omitting a declared input bound is rejected even when the RHS does not need it. Missing
members, different arities, contradictory result bounds, transparent cycles and
expanding family queries are refused with source locations. Checked queries
retain family applications and binder identities; member/binder renames and
body transactions recheck those obligations. Unproven family reference sites
remain explicit and block member renames; family binder renames reject capture
inside their associated declaration.

Family lifetime/const binders, type-outlives bounds, family defaults, predicates on nonparameter
where targets, written reference/lifetime-bearing arguments and lifetime-bearing
RHS definitions remain located source boundaries. A named type parameter can
carry a borrowed actual uniformly, as for positional trait applications; written
region-specific family identity needs a separate region contract.
Family input bounds currently require well-formed trait arguments at declaration
time. A declaration whose input contracts rely on Rust's deferred well-formedness
is refused with `E_UNSUPPORTED`; its conditions are never silently discarded.
[Mode family case](../tests/conformance/pass/gat_mode.lkj) and
[Parser mode case](../tests/conformance/pass/gat_parser_modes.lkj).
The [delegated parser case](../tests/conformance/pass/gat_delegated_parser.lkj)
combines exact `OutputM`/`PResult` projections with a handwritten tuple parser,
including emitting, discarding and error propagation through `?`.

Send/Sync/Data derive structurally; Data excludes capabilities. Expanding generic-instantiation SCCs reject, including alias paths.
Method calls lower to concrete function calls; trait spelling alone grants no builtin semantics.

**Sized bounds and borrowed unsized values.** Named type parameters require the
builtin `Sized` bound by default. `T: ?Sized` removes that default; the equivalent
`where T: ?Sized` form and additional ordinary bounds are supported. A parameter
can still establish sizedness through an explicit `Sized`, `Clone` or `Copy`
bound. Trait `Self` does not have an implicit `Sized` requirement; associated
types retain theirs.

Borrowed `str`, slices and generic `&T`/`&mut T` values can pass through relaxed
parameters. Nominal view aggregates may contain references to such parameters:
`struct View<'a, T: ?Sized> { value: &'a T }` has sized storage. Definition
checking keeps the relaxed source contract even if a later caller chooses a
sized concrete type. The verifier's private stand-in layout supplies no
sizedness evidence. Ordinary calls, implementation prerequisites and method
signature comparison check implicit and explicit requirements; an implementation
method cannot strengthen a declared relaxed parameter to require `Sized`.

Executable parameters, results and locals require sized storage. Array and slice
elements, tuple fields and nominal fields also require a proven `Sized` type,
including in unused nominal declarations, abstract signature components and
implementation definitions. An abstract trait signature may name `Self` by
value; an executable body must establish the storage requirement. Associated
equalities retain their ordinary contract meaning: they do not fabricate a
concrete implementation or grant a named relaxed right-hand parameter an
additional `Sized` assumption. Concrete associated definitions must satisfy the
associated declaration's implicit bound.

A declaration may combine `T: ?Sized` with `const N: usize`. Const values carry
no `Sized` obligation; `[T; N]` still requires a sized element even when a later
caller chooses zero. Associated array equalities retain the distinction between
independent named counts.

This support uses existing borrowed representations. Unsized tuple and nominal tails,
owned unsized parameters/results/locals, and `Box<T>`/`Vec<T>` with an unproven
sized element remain located refusals. Existing restrictions on heap-stored
views still apply. Relaxation is limited to named type parameters; associated
`type Item: ?Sized`, explicit `Self` or concrete-target relaxation, and relaxed
anonymous `impl Trait` bounds remain unsupported. Only the resolved builtin
`Sized` may be relaxed: a same-spelled user trait supplies no marker authority.
Duplicate relaxations and explicit implementations of builtin `Sized` reject.

Queries preserve written relaxed bounds and their ordinary trait references.
Checked renames and bound replacements revalidate existing callers; rejected
changes leave the admitted source intact. Header changes invalidate frontend
checkpoints, while unchanged authenticated images remain reusable.
[Borrowed unsized cases](../tests/conformance/pass/maybe_sized_borrowed_str.lkj),
[view aggregates](../tests/conformance/pass/maybe_sized_nominal_view.lkj),
[mixed const and relaxed parameters](../tests/conformance/pass/maybe_sized_const_borrowed.lkj),
[meaning, edits and cache checks](../tools/maybe-sized-test.sh).

Associated equations key on receiver variable, resolved declaring trait, its positional type arguments and member, including inherited members; different trait applications remain distinct.
Equality bindings such as `Source<Item = i64>` work in generic parameter bounds, parameter where clauses, supertraits and associated-type bounds.
Nested binding values, repeated equal constraints and sibling generic parameters are supported.
Cycles consisting only of unresolved projection dependencies are refused; finite relationships such as `A::Next = B, B::Next = A` are supported.
Definitions retain equality before generic-body checking; every concrete instance checks the implementation's actual associated type.
Implementation prerequisites and trait-method signatures retain that constraint.
Generic parameter bounds and projected-target `where` bounds establish their
ordinary trait identities and collect sibling equality equations before resolving
right-hand sides. Reordering those written constraints does not change their
meaning. Targets that normalize to concrete types must satisfy their full bounds.
A unique structurally compatible implementation may contribute conditional equations, never proof. Selection checks prerequisites without specializing unresolved caller types to fit.
Associated callable inputs can use declared trait evidence regardless of generic parameter order. Missing/ambiguous/incompatible bounds reject.
Projections require checked child witnesses, and pending/cyclic nonstructural goals are not proofs.
Conflicting, unknown, ambiguous or incompatible member bindings reject at the binding. Equalities on ordinary types or implementation trait headers reject.
Positional arguments can precede equality bindings, as in `Parser<u8, Output = i64>`; the equality belongs to that exact applied trait. Generic associated types retain their existing limits.
Equalities lower to ordinary concrete types/calls with no new checked-IR operation or checker authority.
Queries retain binding syntax and resolved member references; checked function changes preserve constraints. [Equality cases](../tests/conformance/pass/associated_equality.lkj).

Fully qualified associated types and immediate calls retain the receiver type,
declaring trait and member independently: `<T as Source>::Item` and
`<T as Source>::value()` cannot select an inherent member or another trait's
same-spelled item. Aliased traits, nested projections, explicit receiver arguments
and method type arguments use the same selection. Projection results can occur
inside other types and in `where` predicates; conditional implementation selection
proves these predicates before exposing a witness. Qualified and unambiguous
shorthand projections denote the same source type. Unused nominal declarations
and trait method signatures retain their obligations.
Forwarding `<F as Parser<I>>::Error` as an associated type carries its declared
bounds from the exact `F: Parser<I>` assumption, so `Error: ParseError<I>` uses
that same `I` during generic implementation checking.

A chained call such as `<T as Source>::Item::make()` selects an associated
function on the projected receiver. Its declared trait must be in scope; inherent
functions on a concrete receiver use ordinary precedence. The checked generic
selection survives specialization, including when the concrete type has another
same-spelled inherent function. Nested qualified receivers and method type
arguments keep the same identity. Provided associated function bodies use the
selected trait's parameter scope; an implementation's generic names cannot
capture the provided method's own parameters. Associated equality constraints belong on
bounds; bindings in an `as` clause, such as `<T as Source<Item = i64>>::Item`,
are located refusals.

Associated definitions resolve one requested member at a time. Finite sibling
chains, decreasing generic families and nominal recursion through indirection
are supported. Transparent direct/mutual cycles and expanding associated-type
queries reject with located diagnostics, including in unused implementations.
Only completed concrete queries are cached; definition checking does not reuse
results across changing rigid assumptions.

Implementation signature proofs normalize qualified equality values for the
current receiver. A qualified equality value requiring a separate concrete
implementation may remain a located refusal.

Associated and concrete type predicates apply to functions, nominal types, aliases,
traits and implementations. A method implementation cannot strengthen its declared
predicate. Invalid concrete predicates and missing trait/member evidence reject
at the source form. Trait lifetime/const arguments, generic associated families,
standalone qualified function values, builtin-trait qualification and recursive
where assumptions requiring a lazy contract remain located refusals. These forms add no checked-IR
operation or trusted trait proof. Queries preserve the written qualified type and
exact receiver/trait/member references; checked renames update the selected trait
family, and checked body changes run through ordinary admission.
[Qualified path cases](../tests/conformance/pass/qualified_associated_paths.lkj),
[predicate cases](../tests/conformance/pass/qualified_where_blanket.lkj),
[chained calls](../tests/conformance/pass/qualified_chain_where.lkj),
[qualified equality bounds](../tests/conformance/pass/qualified_equality_where.lkj).

Const `usize` parameters are supported on free functions, inherent functions and
implementation blocks whose receiver contains each const parameter. They can
appear as standalone array lengths, repeat counts, ordinary `usize` expressions
and explicit function/method arguments. Calls infer const values from array
arguments or use decimal `usize` literals and in-scope const parameters. Mixed
type/const parameters, borrowed array receivers, associated array results,
recursive calls and opaque-result captures retain the same parameter identity.
Static methods on non-generic receivers accept explicit method arguments.

Definition checking keeps each symbolic count distinct from every literal and
from sibling parameters, including unused definitions and conditional branches.
An implementation for `[T; 0]` supplies no evidence for `[T; N]`; comparing `N`
with zero does not establish type equality. A symbolic array repeat requires
`Copy` even when a later call chooses zero or one. Overlap checking treats const
variables independently from type variables. Specialization emits existing
concrete arrays, repeats and integer values; there is no new checked-IR type,
operation or proof rule. The private storage used during discarded definition
checking never establishes an array length equality.

Const parameter types must resolve to the builtin `usize`. Other const types,
const defaults, arithmetic/braced const arguments and array lengths, const
parameters on nominal types/aliases/trait declarations or trait methods remain
located source boundaries.
Where predicates whose written target contains a symbolic array length also
remain a located boundary: the current predicate engine introduces assumptions
for direct rigid types, while compound targets require concrete trait evidence.
Queries retain written const parameters and symbolic array lengths, report const
parameters as `usize`, and identify references in both type and value positions.
Checked const-parameter renames update those references when the replacement
spelling is fresh in the complete enclosing item and all use sites are exact.
Ambiguous or capturing renames retain a located refusal. Checked body edits
revalidate symbolic identities before committing source changes.

A name in an array type length can refer to a visible module constant
declared with the builtin `usize` type and initialized by an integer literal.
The name may use a module path or a lexical import. For a bare name, a const
parameter of the same name takes precedence. Resolution uses the literal's value
as an ordinary concrete length and retains the written reference for queries
and checked rename.
Module constants in repeat counts or const-generic arguments and calls or
arithmetic in named length initializers remain located source boundaries.
[Const array cases](../tests/conformance/pass/const_generic_arrays.lkj),
[const receiver cases](../tests/conformance/pass/const_generic_borrowed.lkj),
[distinct opaque captures](../tests/conformance/fail/const_generic_opaque_identity.lkj).

Argument-position `impl Trait` introduces a distinct anonymous generic parameter

for each occurrence, including through references, tuples and ordinary type
arguments. Multiple bounds, callable bounds and inherent/trait method parameters
use the same definition-site and concrete bound checks as named parameters.
Explicit call type arguments select only written named parameters; anonymous
parameters are inferred independently and cannot be chosen through a turbofish.
Trait implementations preserve whether a method parameter was anonymous.
Source queries retain the written `impl Trait` bounds and checked changes reparse
that ordinary form. Anonymous parameters are not separate editable declarations.
Nested `impl Trait` inside another bound, function-pointer signatures, local/type
aliases remain located unsupported forms.
[Argument cases](../tests/conformance/pass/argument_impl_trait.lkj).

Direct return-position `impl Trait` on free and inherent functions exposes the
written bounds while retaining one hidden concrete result type per definition
and captured type arguments. Different producers and different captured type
arguments retain distinct source identities even when their concrete layouts
match. Every producer is checked under rigid parameters, including unused
definitions; every caller is checked against the opaque contract, including
nongeneric callers. Hidden fields, inherent methods and unadvertised `Copy` are
unavailable. Automatic `Send` and `Sync` facts follow the checked hidden layout.
Concrete execution uses ordinary layouts and functions through the unchanged IR
checker. Returned `Fn`, `FnMut` and `FnOnce` closures preserve source-checked
capture and method choices across specialization.

When the opaque result has no written lifetime bound and its hidden value is
view-free, an advertised callable may return views supplied by its arguments.
A result leaf from a type parameter selects only the corresponding leaves of
arguments written using that same parameter. Equal concrete types leave
distinct parameters independent. An elided result requires a single elided
argument region. Named output regions require separate caller-region support.
The returned loan follows the selected argument independently of later calls
or destruction of the callable. Concrete closure origins must fit the
advertised contract. Captured views and trait-method positional loans remain
separate source rules.
[Shared](../tests/conformance/pass/opaque_callable_shared.lkj) and
[exclusive](../tests/conformance/pass/opaque_callable_mutable.lkj) cases exercise
both execution engines.

Associated-type equalities in argument and result bounds use the ordinary
declared constraint rules. In an inherent result such as `impl Source<Item =
Self>`, the equality captures the producer's `Self`; a later method call binds
its own receiver separately. Qualified associated paths may use advertised
equalities and associated bounds. Unwritten associated types and additional
traits of the hidden result remain abstract.

View-bearing opaque results require one explicit result lifetime bound such as
`+ 'a` or `+ 'static`; reference leaves preserve that contract through calls and
unused-body checking. Lifetimes inside a callable argument signature do not
become lifetime bounds on the returned value. Source queries retain `impl Trait`
for declarations and local bindings. Checked body replacement may change the
hidden layout; such programs use full source-bound frontend construction after
an edit, while unchanged authenticated images remain reusable.
Nested result positions, trait method results, recursive hidden-result inference,
multiple independent result lifetime bounds, and implicit lifetime capture of
view-bearing generic results remain located unsupported forms.
[Result cases](../tests/conformance/pass/return_impl_trait.lkj),
[returned closures](../tests/conformance/pass/return_impl_trait_callable.lkj),
[opaque queries and edits](../tools/return-impl-trait-test.sh).

Unused generic function/impl/default bodies and unreachable expressions check under
rigid parameters through production elaboration and ownership checking, without
sampling concrete types. Bounds, associated relationships, method signatures and
explicit arguments check. Selected trait/source methods and receiver dereference
counts survive specialization; newly applicable inherent methods cannot replace
them. Rigid verification types/functions never reach execution.
[Generic selection](../tests/reference/generic_selection.lkj).

| Trait operation | Meaning and lowering | Rejection or rewrite |
| --- | --- | --- |
| Clone / PartialEq methods | Typed receiver/argument/result contracts; Copy provides Clone evidence and Eq provides PartialEq. Explicit builtin nominal implementations validate required methods, modes and arity. | Ambiguous builtin/source methods reject. Same-spelled user traits are distinct. |
| == / != | Evaluate operands once left-to-right. Read scalar left before RHS effects; borrow nominal owners. Same-type references compare referents. Explicit ne executes, otherwise negate eq. | Inherent methods/user PartialEq spellings do not replace the selected builtin contract. |
| Copy/Clone/PartialEq/Eq/PartialOrd/Ord derives | Validate structural fields, supertraits, conditional generic assumptions and coherence even when unused. Generated monomorphic functions use ordinary IR checking and memoized identities. | Duplicate/overlapping implementations and invalid fields reject. Other unsupported derives report E_UNSUPPORTED. |
| Clone derive | Visit fields in order, selecting builtin traits rather than inherent methods. Nongeneric Copy nominal derives copy wholesale, including lifetime-only declarations; type-generic derives call field Clone even if a specialization is Copy. Explicit Clone still wins. | Default view-bearing clone_from needs a mutation contract. |
| Equality derive | Compare tags and short-circuit active fields in declaration order, including tuples/arrays/nested Vec/boxed recursion. Eq has no method. | Fields need the proper selected evidence. |
| Ordering derive | Nominal fields and enum declaration order; arrays/slices/Vec lexicographic. A decisive unordered float stops comparison and makes all four relations false. str/String compare UTF-8 bytes. | Ordering does not use custom PartialEq as an equality oracle. |
| Default derive | Ordinary generated implementation using field defaults under declared bounds. | Unsupported field evidence rejects. |
| Explicit Copy/Ord/PartialOrd/Data/Send/Sync | Not implemented as source impls. | Use supported derives/structural predicates, or ordinary methods. |

Default nominal/tuple/scalar clone_from uses clone, replacement and old-owner cleanup; default ne negates eq and scalar eq/ne execute.
Direct Box/array/Vec clone_from is unsupported pending forwarding/reuse/truncation/order contracts; general source trait-default execution is unfinished.

Executables remain monomorphic. Generic definition evidence is not a complete independently replayable generic/witness IR contract and does not fully model arbitrary hidden reference content or external mutation effects.
Concrete specializations still receive complete IR provenance checking.

## Declared regions and closures

Fixed callable input regions create private definition-verification obligations.
Reference leaves through aliases/tuples/nominal fields must outlive required regions
using input facts and declared/implied outlives edges; local storage cannot prove a
universal input region. Helper calls transport fixed type-family leaf requirements.
A fixed closure input cannot satisfy a fresh higher-ranked input.

Named lifetime predicates such as `where 'long: 'short + 'other` on free
functions and inherent methods use the same region edges as inline lifetime
bounds. A method may name a lifetime declared by its enclosing impl. Both sides
must name declarations in that scope; the relationship retains its direction
and applies during unused-body checking and at calls. Queries show the written
relationship and exact lifetime declaration references; checked body edits and
function renames retain it. Inline bounds use the same declared names, with
enclosing impl bounds checked in their own scope. Directed return-origin
selection is limited to free functions and inherent methods; trait methods
retain exact lifetime-label agreement. Paired trait methods with inline
lifetime bounds must declare identical directed edges after method-local names
are paired by generic position; adding or dropping an edge rejects. Bounds
involving an enclosing lifetime remain located refusals until owner substitution
is implemented.
Ordinary results preserve invariance through mutable storage. An opaque result
lifetime instead requires its hidden references to outlive that lifetime,
without equating their stored lifetimes to it. Trait-method and implementation-header lifetime
predicates, anonymous or static lifetime predicates and higher-ranked lifetime
predicates remain located refusals. No IR, independent
checker or runtime rule is added. These edges also apply to const-qualified
functions with mixed lifetime, type and builtin-usize constant parameters, and
to borrowed `T: ?Sized` referents. They preserve distinct opaque constant
captures and mutable-storage invariance.
[Lifetime where cases](../tests/conformance/pass/outlives_where_outer_impl.lkj)
and [const-array opaque case](../tests/conformance/pass/outlives_const_array_opaque.lkj).

### Named type-outlives bounds

Free functions and inherent methods admit named bounds such as
`where T: Clone + 'a` and inline `T: 'a`, including several bounds on one
parameter. The subject is the function's own type parameter; the lifetime may
belong to that function or its enclosing inherent implementation. Declaration
scope and directed lifetime edges are retained. For example, `T: 'long` and
`'long: 'short` establish `T: 'short`, without establishing the reverse edge.

An abstract type may contain borrowed data even when its temporary definition
layout has no fields. An opaque result with a lifetime bound therefore checks
all captured type dependencies, including dependencies inside owned structures,
enum alternatives, arrays and containers. It checks unused generic definitions
as well as concrete calls. Forwarding another opaque result retains its verified
hidden storage and declared type dependencies. Direct well-formed
`&'a T` and `&'a mut T` inputs imply `T: 'a`; an output annotation creates
no such assumption.

At calls, fixed callable input regions constrain the callee's type-outlives
obligations. Actual reference leaves must satisfy those regions, while an
abstract caller type needs an established bound. A fresh, unconstrained callee
lifetime may be chosen for that call. Existing reference-origin and mutable
storage checks continue to apply independently of the whole-type proof.
The [capture case](../tests/conformance/pass/type_outlives_capture.lkj) and
[parser buffer case](../tests/conformance/pass/type_outlives_parser_fill.lkj)
exercise these contracts on both engines; the latter uses explicit input type
arguments because inference from a later opaque-callable use remains separate.

This subset does not admit type-outlives bounds on nominal, alias, trait or
implementation headers, trait methods, associated family parameters or
nonparameter where targets. Static and anonymous type-outlives bounds remain
located refusals. Written regions in nominal arguments, function-pointer inputs,
invariant nested input storage and ordinary result lifetimes also remain
located boundaries for functions with type-outlives predicates: those forms
need caller-region instantiation. Direct outer reference inputs and direct
opaque result lifetime bounds use the proven contracts above. Opaque callable
outputs supplied by stored views still require a source contract for those
views.

Fixed callable inputs may retain loans independently of their result or callable
destruction. Effects track source roots; descendant access requires checked
result selectors, not mere source overlap. Shared stored-descriptor access retains
the exact holder and suspends its parent reborrow. Repeated calls revoke obsolete
overlapping permissions; branches union possible effects and retain only justified
permissions. Effects use finite monotone loop replay; initialization/loan backedges
still require equality.

Named local lifetimes check even in unused/lifetime-only definitions, including
delayed initialization, patterns, aliases, fields, replacement and later Some after
None. Loans persist across moving/dropping/replacing descriptors; checked current
holders/descendants retain permissions. Elided annotations allow shorter reborrows;
undeclared names reject even in dead source. Storage annotations constrain earlier
and later contents through exact transfer identities and backward requirements,
then a private verifier checks reachable stores. These observations only restrict.

Typed views retain their visible leaf regions separately from referent origins.
Shared descriptor views may shorten access without shortening the original;
nested mutable views preserve invariance and constrain stored descriptors too.
Requirements follow checked transfers, constructors, exact result selectors, named/
input-tied outputs and fixed callable inputs. Fresh reborrows prove their issuing
chain; moving/copying a descriptor retains its original contract.
Finite projection cycles use a visited worklist; bounded result projections,
growing recursive equations and explicit generic arguments with named reference
regions (including concrete aliases, calls, constructors and patterns) remain
E_UNSUPPORTED pending type-instantiation contracts. Infer the arguments or use
elided reference arguments where sufficient. Borrowed slice elements with named
reference leaves need a collection inventory and remain unsupported.

Annotation metadata uses the compiling body's actual local/projection/instruction
identities. Probes publish none. Closure bodies may require regions for exact
capture leaves; each construction proves them from actual sources, and capture
remapping transforms metadata too. Concrete type equality grants no region.
Private annotations cannot be declared in text IR or change its acceptance/meaning.

Closures lower to environment structs plus ordinary functions. Body operations
select captures by source binding identity/order, including disjoint fields.
Owned captures move; shared/mutable captures use loans; unique immutable access to
a captured mutable-reference descriptor retains exclusivity. Capture destruction
uses capture order, with projections of one root in field order. Last-use cleanup
may end reference captures while owned captures retain lexical destruction.

Fn/FnMut/FnOnce choose shared/mutable/consuming calls statically. Rigid bounds and
specialization retain that selection. Callable equations can infer inputs/results,
including result-only generic parameters, but grant no origins; signatures still
validate. Contextual/inferred parameters, irrefutable patterns, nested closures
and borrowed adapters work. Environments have ordinary Copy/cleanup rules.

Inferred return types constrain shape without granting reborrows: returning a
captured mutable reference consumes it and requires FnOnce, through constructors,
tuples, branches, loop breaks and explicit returns. Declared outputs retain
coercion behavior; inferred struct-update bases only supply equations.
Partial return annotations need per-position coercion evidence, and contextual
FnOnce demands do not yet authorize captured escaping reborrows.
Unique visible compatible method headers can provide conditional closure equations;
primitive inventory precedes competing traits, but full selection still checks.
Ambiguous/incomplete domains supply none; structural patterns wait for reference shape.

Shared/mutable calls never lend owned environment storage. Results come from
captured reference origins, arguments or static storage, with per-leaf contracts.
Checked body observations may narrow candidate origins, followed by full independent
IR checking; they grant no authority. Disjoint mutable outputs stay disjoint, and
shared results may outlive reference-only environments while owners live.
Generic/associated result families can refine similarly: acyclic callees first,
recursive groups conservatively. This does not prove unused generic hidden effects.
Implicit capture of stored views in opaque generic results, lending closures and dynamic callables remain unfinished.
Only borrowed_closure_escape remains pending; concurrency uses isolated processes and owned messages, not shared-address-space threads.

## Source library

| Facility | Source behavior and lowering | Limit or rewrite |
| --- | --- | --- |
| Text/String | Literals are immutable module-long UTF-8 views; str is unsized and shared. len counts bytes; is_empty/as_bytes/to_string preserve origins. String privately owns Vec bytes; Clone/clone_from copy valid bytes and reuse target capacity; equality compares length/bytes. | Bare/mutable text rejects. String.parse requires text.as_str().parse::<T>(); String.split uses text.as_str().split(separator). as_str currently revalidates UTF-8. |
| str::from_utf8 | Checked runtime validation returns source Result<&str, Utf8Error>, borrowing original bytes. Utf8Error is an opaque marker. | No unchecked conversion or error-position/length API. |
| Text methods | Prefix/suffix, contains/find/rfind, ASCII case/comparison, repeat/replace, bytes/chars, LF/CRLF lines, strip_prefix, split_once and split (including empty patterns). Pattern shares text/char needles; integer decimal parse covers ten types with validity/range checks. | General Unicode case conversion and str.trim are unsupported; std::str::trim_ascii(value) strips only bytes 9..13 and 32, preserving Unicode spaces. Only proved ASCII literals receive an automatic trim edit. Borrowed iterators retain owners. |
| Character helpers | UTF-8/UTF-16 lengths, ASCII predicates/case, Unicode whitespace/control, to_digit and to_string. | Operate on Unicode scalars, never arbitrary code points. |
| Formatting | Arguments evaluate once in order; macros call source String/Formatter traits. Display, Debug ?/#?, x/X/b/o, positional/named/captured arguments, escaped braces, fixed width, Unicode fill, alignment, sign/prefix/zero padding. Explicit zero width remains distinguishable. | write!/writeln! target String (Write in scope) or Formatter; arbitrary Write sinks, float formatting, x?/X? debug modes and dynamic width/precision remain unsupported. Errors stop later formatting; format! traps on failure. |
| Debug | The declared std::fmt::Debug trait formats integers, bool, char, str/String, references, Box, slices/arrays/Vec, BTreeMap/BTreeSet, Option/Result, unit and tuples of one through twelve fields. Source DebugStruct/DebugTuple/DebugList/DebugMap/DebugSet builders preserve field order, child options, lazy alternate indentation and partial output on errors; all support finish_non_exhaustive. Map key/value sequencing is checked, and an error suppresses later child formatting. Collection Debug requires Debug bounds without Ord. derive(Debug) generates ordinary checked struct/enum implementations, including generic field obligations before use. | Unicode escaping follows the library's Unicode 17.0.0 table. Builders retain the formatter's exclusive loan, not their entry arguments. No byte inspection, reflection intrinsic or opaque formatting authority is added. Iterator-based entries, borrowed format_args! and float Debug remain absent. |
| Option/Result | Canonical source declarations and re-exports; conditional derives, views, owned and/or/transpose and selected map/default combinators. Eager arguments evaluate before call, lazy callbacks only on selected variants; unused captures/payloads still clean up normally. | Some nested receiver expressions need annotations. Option::expect evaluates its message but retains fixed trap text. Result::unwrap omits its dynamic error payload and reports the callee site; all sites retain their source path and line. To retain errno and the operation site, use an already granted output writer: `match operation() { Ok(value) => value, Err(errno) => { output.write(format!("operation: errno {errno}\n").as_bytes()); panic!("operation failed") } }`. |
| Option extras | xor, zip/unzip, flatten, replace/insert/get_or_insert, is_some_and/is_none_or, ok_or/ok_or_else. None predicates yield false/true respectively. filter borrows the payload then keeps/drops the owner. | Returned views preserve selected input/factory origins. |
| Option<&T>/Option<&mut T> | copied uses Copy without Clone; cloned invokes Clone once for Some, never None. | Owned clones may be independent, but nested views retain external origins. |
| Vec/slice access | is_empty/get/first/last and mutable slice counterparts; invalid get returns None, including usize::MAX. contains borrows query/elements and uses eq in order without copying/consuming. | Returned references retain collection footprints. |
| Collection copying/equality | extend_from_slice/to_vec clone in order; empty input does no clones. Vec Clone requires Clone; equality requires only PartialEq, checks length then element ==, and Vec ne negates the whole result. Nested types/floats/custom implementations follow selected traits. | Inherent clone/ne cannot replace generic contracts. Arrays use array[..].to_vec(); helper signatures and element constraints check independently. |
| Slice/Vec helpers | starts_with borrows; copy_from_slice needs Copy and equal lengths, with array receivers written as array[..].copy_from_slice(source); predicate split retains empty groups at delimiters. clear/truncate destroy removed elements forward; stable sort/retain are source operations. | Further primitive conversions, Sum/FromIterator and exclusive iter_mut need their own contracts. |
| Ordered maps/sets | BTreeMap uses an indexed AVL arena/free slots with logarithmic lookup/insertion/removal; BTreeSet reuses it. Sorted shared/owned iteration, map keys/values/first_key_value/pop_first/retain and set first/pop_first. Clear/drop and partially consumed iterators destroy in key order; Clone/Default/equality ignore arena shape. | Entirely source algorithms over existing storage, not new trusted primitives. |
| Generational arenas | std::arena::Arena<T,Kind> owns graph nodes; Handle<Kind> stores owner namespace/index/generation. get/get_mut lend temporary checked views; remove moves the value and invalidates prior handles. Exhausted generations retire. | Caller assigns distinct same-Kind owner IDs while any handle survives. Handles are values, not authority; no heap views or new IR. [Million-node specimen](../examples/object_graph/README.md). |

Option/Result also provide map, map_or/map_or_else, and_then/or_else/unwrap_or_else;
Option has filter and Result map_err. A map_err closure needs known input/result types: annotate the missing type. For a literal.to_string() result use |_| -> std::String { literal.to_string() }; this supplies the result type while preserving receiver evaluation and the closure's capture/return/? scope. String ordering/from conversion, scalar
ASCII helpers and min/max are ordinary source conveniences, not general trait evidence.
[Formatting case](../tests/reference/formatting.lkj).

Iteration resolves source Iterator::Item/next and IntoIterator evidence.
Collection for holds iterable temporaries in an outer match, loops over next and
binds each item with an irrefutable let. Non-unit bodies/refutable patterns reject;
break/continue/return/exhaustion preserve item/iterator cleanup. Numeric ranges
share binding machinery. No iteration IR operation or nominal-name shortcut exists.
Shared Vec/slice/map/set and owned Vec/Option/map/set iteration are supported. Returning an element view can retain the loop iterator temporary; for an immutable self field of canonical Vec type, index self.FIELD directly and advance before the original body, preserving continue and order. Custom iteration/index/deref or iterator cleanup needs its own proof. IntoIterator for &mut Vec<T> is absent: evaluate the collection once, then use an index loop that advances before borrowing each element, so continue still progresses. Borrowed arrays use &values[..] for shared slice iteration. Owned arrays need an IntoIterator implementation; index them, or use vec![elements] for owned literal iteration.

enumerate/skip/take/zip are source adapters; chain currently needs identical
iterator types to prove Item equality. map/filter/filter_map/inspect store the
iterator/callable. filter/inspect borrow fresh items; map/filter_map transfer them.
Adapters follow next even after a previous None and do not silently fuse.
fold/for_each/count stop at first None and consume in order; fold owns its accumulator. collect returns Vec<I::Item>, preserving ownership and order; general FromIterator destinations remain unsupported. Range values call ordinary source constructors in std::range, with each written endpoint evaluated once. Range, RangeFrom and RangeInclusive implement source Iterator/IntoIterator for all ten integer types; each advances with checked arithmetic, and an inclusive maximum is returned once before exhaustion. Range indexing keeps its existing borrowed-view rules, and directly bounded numeric for ranges keep their endpoint semantics. [Consumer cases](../tests/reference/iterator_consumers.lkj).
any/all borrow the iterator mutably, pass items by value to FnMut and stop at a
decisive result/None; empty answers are false/true. Iteration resumes after that
consumed item; callbacks/captures drop on return and remaining items stay owned.
find borrows predicate inputs and returns the matching owned item; position
consumes and returns an index. Receiver evaluation precedes callback construction.
Owned temporary receivers, including projected/indexed values, are writable without
making named bindings or shared references mutable.
[Iterator/Option case](../tests/reference/iterator_options.lkj).

### Core facade and integer contracts

An unshadowed external `core` root selects the source core facade. Lexically
declared modules, aliases and generic parameters keep their own identities.
The facade reexports `borrow`, `cmp`, `fmt`, `iter`, `marker`, `mem`,
`num`, `option`, `result`, `slice`, `str` and its prelude; its `ops`
module exposes the admitted range contracts. This namespace is a source graph
over the same library declarations used by `std`.

`Borrow<T>` and `BorrowMut<T>` provide same-value borrowed projections.
`cmp::min/max` use the ordinary `Ord` contract. `mem::drop/replace/take`
compose the existing ownership operations, with `take` requiring `Default`.
The facade does not introduce a public layout-inspection or raw-memory API.
`NonZeroUsize::new` rejects zero and `get` returns its stored value; its field
is private. Copy, Clone, comparisons, Display and Debug use that checked value.
Its source representation carries no Rust niche or external ABI claim.

Primitive `MIN/MAX` paths for the ten integer types select typed constants
from the source integer module. The primitive must be written directly to use
this library mapping. A local
module or generic parameter named `usize` prevents the primitive shortcut.
An absent integer library refuses the spelling rather than supplying a hidden
value. The source `SaturatingArithmetic::saturating_add` implementations use
checked addition and the corresponding typed limit on overflow.

`Bound`, `Range`, `RangeFrom`, `RangeTo`, `RangeToInclusive`,
`RangeFull` and `RangeInclusive` are ordinary source values. All six range
forms implement `RangeBounds<T>`. This initial `contains` contract compares
the same `T: PartialOrd`; heterogeneous comparisons need a separate contract.
Inclusive iteration retains its exhaustion state, so bounds and containment
reflect an exhausted range. Integer iterators remain checked at their numeric
limits.

`core::slice::Iter<'a, T>` names the existing shared slice iterator.
`Copied<Iter<'a, T>>` requires `T: Copy`; `Cloned<Iter<'a, T>>` invokes
`Clone` for each yielded element. Both adapters retain the source borrow.
The current adapters cover this concrete iterator family; arbitrary borrowed-item
iterator families need their own proof. `str::CharIndices` reports UTF-8 byte
offsets paired with Unicode scalar values. `str::FromStr` has an associated
error type and admits user implementations; the ten integer implementations
share checked decimal decoding.

### Phantom ownership markers

The frontend authenticates the canonical zero-field `PhantomData<T: ?Sized>`
declaration and its unconditional derived value traits. Its nominal application
retains `T` in source inference, generic obligations and type-outlives analysis.
Copy, Clone and comparisons do not require those traits on `T`; Send and Sync
require the corresponding proof for `T`. A marker does not contain or destroy
a value of `T`.

Storage uses a distinctly named instance of the existing zero-length
`[(); 0]` representation. The existing native and independent unit layouts
therefore give it zero bytes and alignment one, independently of `T`.
No trusted IR type, operation, layout rule or runtime field is added. Source
construction, pattern coverage, matching and coercion preserve the nominal
application: an array literal, array method, array trait implementation, index
or slice coercion cannot acquire a marker or its source contracts.

Reference-bearing marker arguments currently receive located `E_UNSUPPORTED`
diagnostics, including inferred arguments and completed recursive nominal
arguments. Written lifetime-bearing applications and phantom-only covariance
proofs remain explicit boundaries. The frontend does not erase those regions
or claim a stored reference which is absent at runtime. Abstract `T: 'a`
dependencies still apply when a marker is captured into an opaque callable.

The [core facade proof](../tools/core-facade-test.sh) covers exact source queries,
checked edits, declaration-shape invalidation, nominal unit boundaries and the
actual emitted marker types through both production layout calculations.
The conformance cases distinguish non-Copy element types, absent Send/Sync
obligations, whole-value outlives requirements, destruction, region boundaries
and attempts to use the physical array representation as source authority.

## Checked IR and execution

IR has explicit types, signatures, locals, places, copy/move/borrow/drop and
structured control. Every type/signature/operation shape checks, including dead
code; source inference/trait search never occurs below admission.
Native emission accepts only private CheckedProgram or CheckedUnit proofs;
optimizer output must be rechecked. [Explicit source units](units.md) check one
unit against authenticated dependency interfaces and stream their native linkage.
Their manifest names define source namespaces; ordinary builds retain their
existing single-root behavior.
See [IR rules](ir.md) for exact provenance, tag, join, destruction and loop contracts,
and [executable scalar IR](../tests/reference/scalar.lkjir) for the primitive prefix.

Strict drop requires initialized storage; drop-if skips an absent exact owned place
but still requires initialized ancestors. Neither moves/drops through a reference
or indexed collection; use checked replace and destroy its separate result.
Conditional cleanup stays at lexical destruction/replacement, never branch joins.
Live owners/loans must agree at loop backedges. Definite initialization intersects
at branches while possible owners/holes/loans union; reads need definite presence
and every possible holder participates in borrow checks. Safe descriptor-only
storage may end before descendants referring to external owners, never before
actual loans into that storage. Early cleanup honors carrier dependencies.
Exact storage identity remains separate from a conservative lifetime/alias footprint.

The backend selects x86-64 directly and emits static Linux ELF; encoding is separate.
Calls/branches use rel32 and an unrepresentable displacement gives a located error.
Scalars use registers or natural-width frame storage; structs/tuples/arrays/enum
payloads and collection elements are inline. Vec is address/length/capacity; Box
and sized references are addresses; slice/text views are address/count pairs.
Aggregate calls/results use frame storage and caller-provided result storage.
Conditional cleanup uses compiler-owned initialization flags, never IR-supplied bits.

Stack addresses remain stable. Reservation retries smaller extents; heap pressure
returns unused reservation and calls regrow pages. Small/medium allocations reuse
slabs/size classes from grown regions; large allocations use mapping/remapping.
Neither allocator nor stack reads RAM/rlimits to plan capacity. Exhaustion is
R0006 (allocation) or R0008 (stack), with located diagnostics.
Bounds, arithmetic, division/shift, UTF-8 and active-tag checks remain.
Locations materialize on cold failure paths; checked-operation lines stay exact,
while allocation/stack failures may name runtime/function locations.
Sealed checked-source runtime alone gets raw load/store/copy/map/unmap/remap.

The independent interpreter uses its own typed heap, initialization/loan identities
and call/control/drop stacks, checking actual references/tags independently.
M_* indicates interpreter safety failure; R* indicates a program trap.

## Commands and diagnostics

One static executable embeds frontend, checker, backend, interpreter, CLI and source
library/runtime. sh tools/bootstrap.sh uses the stored seed and verifies byte-for-byte
reproduction without Rust or Node. The optional witness lives under witness/.

| Command | Result |
| --- | --- |
| lkjscript check PATH | Source/.lkjir check; silent success or located diagnostics. |
| lkjscript ir PATH [-o FILE] | Checked textual IR, to stdout unless an output file is supplied. |
| lkjscript build-child SOURCE --entry NAME --image-key KEY -o IMAGE; lkjscript image-authority IMAGE --image-key KEY | Checked transfer-only image and authenticated canonical authority; verification retains a sealed executable File. See [child image ABI](child-image.md). |
| lkjscript build PATH [-o FILE]; lkjscript build-entries SOURCE -o DIR --artifact NAME=ENTRY ... | Static executable(s); build defaults to PATH.elf and preserves an existing destination on rejection. Multi-entry checks one source/runtime snapshot and every entry/grant before publishing each NAME and its matched NAME.authority report. |
| lkjscript run PATH [--entry NAME] | Execute main, else probe, with native default or --engine interpreter. |
| lkjscript test PATH | Discover expectation cases or execute #[test]/IR test functions across native/interpreter routes; details on failure. |
| lkjscript authority PATH | Entry grants and conservatively reachable operations/calls/owned cleanup, from checked IR alone. |
| lkjscript delta | Frequent accepted rewrites for a writer who knows Rust; ordered from retained diagnostic cases. |
| lkjscript build PATH --profile [--profile-ir IR]; lkjscript profile FILE [--copy-sites --profile-ir IR --limit N --bytes B] | Sample CPU instruction/frame addresses to OUTPUT.samples.PID; optionally retain exact selected native IR. The default function/kind summary remains at most 40 lines. Explicit copy-site detail requires the matching companion and positive row/byte bounds; see [profile contract](ir.md#native-cpu-profiles-and-selected-ir). |

Program arguments after -- remain exact bytes; compiler arguments require UTF-8.
Scalar runs print integer decimal, bool text, UTF-8 char (including NUL), or
f64:0x plus 16 lowercase hex digits and newline; unit adds nothing.
Native scalar results use a driver-owned raw eight-byte descriptor. Private traps carry exact source/message identities; machine observations retain complete text for comparison and authenticated reuse.
User trap records are one bounded `PATH:LINE: RCODE message` line (at most 2048 bytes), with controls/backslashes escaped and overlong paths/messages marked `...`; native failure reporting uses preallocated storage and one complete write, retrying EINTR/short writes.
Standalone failures exit 1 and use only granted Stderr (or World), then granted Stdout if stderr fails; without either grant they are silent. Run/test supervisors keep a private result channel and their own output authority. Both routes format scalars identically.

--stats reports phase milliseconds, IR/code counts and peak RSS; stored builds append [authenticated-store byte/timing attribution](../bench/compiler-cache-io.protocol.md). --source-dir grants
the source root; --library-dir/--runtime-source replace embedded components explicitly.
--help gives options. Test expectation headers declare value, output bytes, diagnostic
or trap; pending headers name the missing intended feature. The gate compares exact
values/bytes/status/file effects and trap kinds; R0004 expectations classify custom traps, while same-source native/interpreter comparisons preserve their full code, message and location. Exhaustion locations may differ.
Terminating cases use generous watchdogs, mutants a separate short allowance.

Parser recovery synchronizes balanced item/method/statement boundaries, marking
skipped regions E_BLOCKED. Recovered source never authorizes emission. With valid
shared signatures, independent inference/IR body errors accumulate; each failing
body stops at its diagnostic. Diagnostic-only placeholders preserve known signatures
for dependent checks, never executable authority. Shared resolution/signature errors
block dependent work; no fabricated types or ownership facts continue analysis.
Single-error APIs select the first diagnostic from these same production paths. When a rewrite is available, stderr adds LKJSCRIPT_EDITS followed by one JSON object {version:1,path,code,edits:[{start,end,replacement}]}; edits use half-open UTF-8 byte offsets in the unchanged input named by path. Apply each nonoverlapping set atomically, then check the rewritten source again; display line/column values are not edit offsets.

## World and capabilities

Entries take owned Arguments, Environment, ExitStatus, Stdout/Stdin/Stderr, Directory/ReadDirectory, Unix-connected File, Image, ForeignCommand, NullDevice, Listener, Connector, TcpPort, TemporaryFiles, Readiness, Process, Clock, Entropy or Stop; no parameters; or a sole World declaring implicit full authority.
build/run require --grant NAME for each parameter: Directory/ReadDirectory take NAME=PATH or NAME=env:VARIABLE, ForeignCommand NAME=/absolute/executable, Image NAME=CHILD_ELF authenticated with --image-key, Environment NAME=VARIABLE, Listener/Connector NAME=IPv4:PORT, and File NAME=unix:/absolute/socket or NAME=unix:env:VARIABLE; TcpPort NAME=PORT (1..65535); others have no payload. Only Listener permits port zero. build embeds descriptors; handles are acquired at startup.
Directory/ReadDirectory's env:VARIABLE selector resolves only that named variable at invocation/startup; absent or empty fails with errno 2, without a default or directory creation. Reading its path bytes separately requires an Environment grant.
Duplicate, missing, unknown or malformed grants reject before publication/execution; acquisition failure traps R0007 at entry with the capability kind, granted value, actual operation and symbolic/numeric errno, e.g. `Listener 127.0.0.1:8798 bind: EADDRINUSE (98)`. Listener can rebind after TIME_WAIT; an active listener still excludes it.
Ordinary build/run rejects ReadFile/Child/Channel/Worker/ReadRegion/ForeignSession/borrowed/aggregate entry parameters. A File entry grants exactly a Unix socket connection; ordinary file-path grants remain refused. Acquire other derived owners from explicit grants. Transfer-only child images follow the child image ABI; Image.spawn currently admits exactly one owned Channel slot.
All capability types, including World, File and Child, are affine, without constructors or Copy/Clone/Data/Send/Sync implementations. Names, integers or casts cannot forge authority. IR checks each operation independently; methods below exclusively borrow receivers, and derived owners can outlive those borrows.

| Form | Result and authority |
| --- | --- |
| World.args(), Arguments.args() | Vec<Vec<u8>> excluding argv0; preserve empty arguments and arbitrary bytes. |
| Environment.get() | Option<Vec<u8>> containing only the granted variable's invocation/startup value: None for absent, Some(empty) for empty. No name argument, ambient lookup or enumeration. |
| World.stdout()/stdin()/stderr() | Corresponding inherited stream capability. |
| World.directory(path) | Result<Directory,i64> acquiring a root. |
| World.listen(address,port) | Result<Listener,i64> binding a numeric IPv4 TCP endpoint. |
| Listener.accept()/accept_nodelay(), port() | Result<File,i64> owning a connected stream (the second disables Nagle); Result<u16,i64> with the bound port. |
| World.connector(address,port), Connector.connect() | Result<Connector,i64> authorizing one numeric IPv4 TCP destination; Result<File,i64> connecting only there. |
| World.tcp_port(port:u16), TcpPort.connector(address:&str) | Result<TcpPort,i64> for a nonzero outbound TCP port; Result<Connector,i64> selecting a run-time numeric IPv4 address on exactly that port. Multiple TcpPort grants express an allowed port set without World; no listening, arbitrary port or name-resolution authority is implied. |
| Connector.connect_deadline(deadline:usize) | Result<File,i64>; connect only to the same destination before an absolute monotonic nanosecond deadline, using nonblocking connection and kernel readiness. The returned File is blocking; callers may explicitly set_nonblocking(true). |
| World.unix_connect(path), Directory.unix_connect(path) | Result<File,i64> for a nonblocking Unix stream; World requires an absolute pathname, Directory confines resolution to its held root and pins the socket inode before connecting. |
| File.send_message(bytes,files,deadline) | Result<usize,i64>; send raw bytes and an owned Vec<File>, consuming every supplied owner on every outcome. Positive partial success transfers rights once. |
| File.receive_message(bytes,descriptor_limit,deadline) | Result<(usize,Vec<File>),i64>; receive raw bytes into &mut [u8] and fresh close-on-exec owners. Truncation closes all delivered rights and leaves the buffer unchanged. |
| World.readiness() | Result<Readiness,i64> owning a wait set. |
| File/Listener.set_nonblocking(bool) | Result<(),i64>; would-block reads, writes and accepts return errno 11. |
| Readiness.watch_file(&mut File,token,interest) | Result<(),i64>; add/update a watch, usize token, u8 interests 1 read and 2 write; zero watches full close/error only. |
| Readiness.watch_listener(&mut Listener,token)/watch_stop(&mut Stop,token) | Result<(),i64>; add/update acceptance or unread-SIGTERM readiness. |
| Readiness.unwatch_file/unwatch_listener/unwatch_stop(&mut handle) | Result<(),i64>; remove a watch without consuming the handle or retaining a language view. |
| Readiness.wait(&mut Clock,deadline) | Result<Vec<(usize,u8)>,i64>; absolute monotonic nanoseconds, events 1 read, 2 write, 4 closed, 8 error. |
| World.process()/clock()/entropy() | Process, Clock or Entropy capability. |
| Entropy.fill(&mut [u8]) | Result<(),i64>; fill from kernel getrandom, retrying EINTR and short reads. |
| World.stop() | Result<Stop,i64>; claim SIGTERM and own a nonblocking signalfd. |
| Stop.requested()/target() | Result<bool,i64> with sticky observation; Result<u32,i64> identifying this execution's signal endpoint. |
| World.temporary_file(), TemporaryFiles.temporary_file() | Result<File,i64>, fresh anonymous read/write storage. TemporaryFiles is an entry-only factory granted by `--grant NAME` with no payload; it has no named-path, inherited-stream or executable authority. |
| World.stdin_file()/stdout_file()/stderr_file() | Owned inherited-stream duplicate, preserving closed streams. |
| World.executable() | Read-only running executable at fresh offset; checked interpretation prepares a native image lazily, unchecked IR returns ENOSYS. |
| World.set_exit_status(u8), ExitStatus.set_status(u8) | Set normal status and continue cleanup/execution; last value wins. |
| Directory.open/create(path), open_regular(name) | Open for reading or create/truncate a File. open_regular accepts one basename, rejects symlinks/nonregular leaves before data access, and reopens the retained inode read-only. |
| Directory.create_new/open_append(path), sync() | Result<File,i64> for exclusive create or create/atomic append (final symlink rejected); Result<(),i64> for directory fsync. |
| Directory.create_private(path), File.set_private() | Result<File,i64> for exclusive creation with initial mode 0600; Result<(),i64> for fchmod to exactly 0600. |
| File.sync()/truncate(length)/lock_exclusive()/lock_wait() | Result<(),i64>: fsync, set byte length, or advisory exclusive flock (nonblocking lock_exclusive; blocking lock_wait). Locks last until the final duplicate closes; lock_wait retries EINTR. |
| Directory/ReadDirectory.open_directory(path), list() | Corresponding descendant directory capability; Result<Vec<Vec<u8>>,i64> names excluding dot/dot-dot, unspecified order. |
| ReadDirectory.open(path) | Result<ReadFile,i64> for a regular file only, without first data-opening a FIFO/device. ReadFile has read/read_to_end/modified_nanoseconds and no write, mode, lock or writable-conversion operation. |
| File.seek(offset:i64,origin:u8) | Result<usize,i64> position; origin 0 start, 1 current, 2 end; other values fail errno 22. O_APPEND still places every write at the then-current end. |
| File/ReadFile.modified_nanoseconds() | Result<usize,i64> Unix mtime from the same held inode; nonregular descriptors fail 22, pre-epoch/overflow timestamps fail 75. Atomic pathname replacement does not retarget the handle. |
| Directory.create_directory/remove/remove_directory(path), rename(old,new), rename_new(old,new) | Create directory, unlink entry, remove empty directory, or rename within root. rename may replace; rename_new accepts basenames in one held directory, never replaces an occupied destination, and has no replacing fallback. |
| std::io::directory_identity(&mut Directory) | Result<[u8;16],i64> with little-endian device/inode of the currently held kernel directory. |
| Stdin/File/ReadFile.read(&mut [u8]), read_to_end() | Result with short-read count or collected bytes; zero is EOF for nonempty buffers. |
| File.write/write_all(&[u8]) | One-write count or complete-write Result. |
| File.duplicate()/readonly() | Duplicate sharing offset/state; or read-only reopen with independent zero offset. Closed remains closed. |
| File.rewind()/set_executable()/seal() | Seek zero, set mode 0755, or permanently seal writes/growth/shrink/seal changes on anonymous storage, including every alias. All return Result<(),i64>; ordinary files cannot seal. Neither mode nor seals grant executable admission. |
| Stdout/Stderr.write(&[u8]) | Complete write or R0007 at the call with stream kind, inherited descriptor, write operation and symbolic/numeric errno. |
| Clock.nanoseconds()/unix_seconds()/unix_nanoseconds() | Result<usize,i64> monotonic nanoseconds for deadlines; Result<i64,i64> CLOCK_REALTIME seconds; Result<usize,i64> CLOCK_REALTIME nanoseconds. Realtime may jump; unix_nanoseconds returns errno 75 for negative or unrepresentable time. |
| ForeignCommand.run(&mut ReadDirectory,&mut NullDevice,&[Vec<u8>],duration_ns:usize) | Result<ReadFile,i64> framed exit/deadline/stdout/stderr; std::foreign::decode consumes it. Synchronous, nonzero duration; absolute deadline begins before setup; every child is killed/reaped before return. |
| ForeignCommand.image() | Result<ReadFile,i64> fresh read view of immutable transformed initial ELF for checked SHA256; no execution or raw writable image access. |
| std::foreign_session::{start,exchange,finish,idempotent_request} | Start derives affine ForeignSession from ForeignCommand, ReadDirectory and NullDevice; exact response framing, one request deadline and transactional output; finish/drop reap, bounded restart is explicit library policy. [Exact contract](programs-foreign-session-contract.md). |
| Image.spawn(deadline)/buffer(bytes); Worker/Channel.send/receive/send_file/receive_file; Worker.wait/terminate/result; File.map_readonly(), ReadRegion.bytes()/len() | Authenticated attached workers, deadline-bound byte packets and moved File values; sealed immutable shared bytes with checked owner loans. ReadRegion methods share-borrow; other receivers borrow exclusively. [Types, results, lifecycle and remaining message-identity gap](ir.md#isolated-workers-and-immutable-regions). |
| Process.spawn_file(executable,&args,stdin,stdout,stderr,result) | Result<Child,i64>, explicit executable File, argument byte vectors and four File streams; result maps to descriptor 3. |
| Child.wait()/try_wait()/terminate()/request_stop() | Wait gives exit code or 128+signal (Option when polling); reaped children later fail ECHILD. Signals give Result<(),i64>, affect only the owned child and do not wait. Dropping a live Child kills and reaps it. |

Convenience methods live in library/io.lkj, command.lkj, read_io.lkj and foreign.lkj. The loader mounts new typed helper modules beside canonical $std/lib.lkj so stored seeds can still build the compiler. Result errors carry positive Linux errno; `std::io::errno_name(errno)` returns its static Linux name (EUNKNOWN otherwise). Explicit matches retain operation/grant context; Result::unwrap never formats generic E. Raw read/write leaves return negative errno. EINTR retries except close, which consumes once.
Paths reject NUL; raw paths need exactly one trailing NUL. Descendants use openat2 RESOLVE_BENEATH|RESOLVE_NO_MAGICLINKS. Only ENOSYS selects anchored openat component traversal: absolute paths, parent/empty components and every symlink reject; held directories retain their inode across rename. The fallback allocates the actual path length, with no guessed path/component limit. Removal, rename and directory creation derive a parent then validate the basename. Ordinary new files/directories use 0666/0777 subject to umask; create_private uses 0600 subject to umask atomically, without an initially broader permission window. set_private sets exactly 0600 regardless of umask, permitting owner read/write after restrictive creation and restricting future opens; it cannot revoke already-open descriptors.

Unix stream grants resolve only the specified socket pathname, or the specified environment selector at invocation/startup. Missing/empty selectors fail with errno 2; a pathname must be absolute, NUL-free and shorter than 108 bytes. There is no inherited-descriptor integer grant or ambient display selection. Directory.unix_connect accepts a descendant path, rejects escapes, opens the target with O_PATH|O_NOFOLLOW under the held root, checks that it is a socket, then connects through that pinned descriptor. The final socket cannot be retargeted by replacing its pathname; a missing /proc/self/fd path returns its OS error without an ambient fallback. Connections and received descriptors use CLOEXEC, preserve closed standard streams, and become ordinary affine File values compatible with Readiness. ReadDirectory has no connect operation. A full Unix listen backlog returns EAGAIN without blocking the event loop.

Message deadlines are absolute monotonic nanoseconds. Each operation attempts once immediately, then retries EINTR/EAGAIN under that deadline; a ready operation may therefore succeed with an already-expired deadline. A required wait after expiration returns ETIMEDOUT (110). Empty byte buffers return EINVAL (22), including an attempted rights-only send: no marker bytes or private framing are inserted. SOCK_STREAM may split/coalesce application messages. On a positive partial send, only the returned prefix has been written and the rights were sent once; send the remaining bytes with an empty File vector. The local input owners are consumed even on errors, including timeout, so callers that require a retained owner must explicitly duplicate it beforehand. Receive returns owned Files without assuming their kernel kind. Message operations validate AF_UNIX and SOCK_STREAM before transfer: TCP returns EAFNOSUPPORT (97), Unix datagrams return EPROTOTYPE (91), and ordinary files return ENOTSOCK (88). All input owners are still consumed on rejection; ordinary File methods retain their existing kind checks. MSG_CTRUNC, MSG_TRUNC or more delivered owners than the explicit limit return EMSGSIZE (90), close every delivered descriptor, and preserve the caller's byte buffer. A successful receive copies only its returned prefix; zero with no owners is stream EOF. This transport supplies no Wayland parser, object identifiers, protocol lifecycle or browser authority policy; those remain source programs.

Entropy.fill blocks until kernel randomness is ready; empty slices succeed. A later error preserves the already-filled prefix and untouched tail; no fallback generator is used.
A successful Stop acquisition blocks SIGTERM for the execution lifetime; failure restores the previous mask and does not claim it. Dropping Stop closes its descriptor without restoring the mask, even after the last Stop drops. Multiple Stops share the pending signal queue; delivery is not broadcast and standard signals may coalesce.
Until its first true, requested() consumes one pending SIGTERM or returns false without blocking; later calls return cached true for that Stop. Its readiness watch reports unread SIGTERM and need not remain readable after consumption.
Native Stop.target is the program PID; interpreted Stop.target is the owned syscall broker PID. The interpreter CLI driver/wrapper PID is not this endpoint: graceful requests must use the published target.
After a successful Stop claim, spawned children restore only the SIGTERM mask bit saved before the first successful acquisition, preserving every other bit. Without a claim the mask stays unchanged; an originally blocked SIGTERM remains blocked.

Captured process files share offsets; rewind before reading. Closed File returns EBADF, drops harmlessly and closes the assigned child stream. Spawn validates NUL-free arguments, chooses argv0 lkjscript, clears environment and executes the File with execveat, without PATH/shell fallback; exec failure returns errno. Direct children receive parent-death SIGKILL. No caller slice pointer survives I/O.
File/ReadFile/Directory/ReadDirectory/ForeignCommand/Image/Channel/NullDevice/Listener/Readiness/Stop drops close once; ReadRegion unmaps/closes; Worker and ForeignSession kill/reap/close. Standard-stream drops leave inherited streams open; acquisition preserves inherited closed streams.
TemporaryFiles grants only creation of fresh, initially empty anonymous read/write Files through the same operation as World.temporary_file(). Grant acquisition and destruction perform no host I/O; each temporary_file call exclusively borrows the affine grant, returns an independent owned File, and propagates existing host errors. The File can outlive the grant and retains ordinary File operations, including sealing and typed descriptor transfer. This grant has no path or environment selector, no byte/file-count policy, no implicit stream or executable access, no source constructor or World factory, and no child-image transfer slot. `Image.buffer(bytes)` instead returns sealed immutable storage; a Weston wl_shm pool refuses its writable shared mapping. [Narrow grant and real compositor proof](../bench/anonymous-files.md).

TcpPort is an affine port grant, not a socket. Its port never changes, and each derived Connector snapshots a numeric address with that same port. Grant parsing and World.tcp_port reject zero; construction and destruction perform no network I/O. A browser can take separate --grant http=80, --grant https=443 and --grant dns=53 capabilities, using the last for an explicit DNS-over-TCP client. TcpPort has no child-image transfer slot in this slice.

Connector.connect_deadline checks expiry before opening its destination socket and returns ETIMEDOUT (110) when its deadline expires. One absolute deadline covers setup and every interrupted readiness wait; readiness is followed by SO_ERROR, so refusal returns ECONNREFUSED (111), never a falsely successful File. Every unsuccessful attempt closes its private socket once. It performs no name resolution and retains the exact Connector destination; a separate Clock grant is needed to observe monotonic time when constructing a deadline.

Readiness drops only its wait set. Unwatch before closing a watched descriptor if aliases remain. Wait batches at most 64 events without limiting registrations; expired deadlines poll once.
Read interest reports half-close; drain readable final bytes. Write-only interest does not report peer write-half closure repeatedly.

ForeignCommand admits one immutable initial image, then borrows a read/execute subtree and explicit null device under kernel confinement: no sockets/descendants, credential changes or filesystem metadata mutation syscalls; parent death and deadline kill the child; reexec keeps confinement. Private read/write/seekable output memfds can supply reexecuted anonymous code under the same confinement. Preexisting special files and mutable runtime code inside the root remain available, and outside metadata/clock/entropy remain observable. This is not an authenticated lkjscript child. The exact fixed environment and limits are in docs/ir.md.
Generated programs have no ambient filesystem/environment/process imports or source-declared host imports. std::time::http_date formats supplied time; it grants no ambient clock. std::text supplies source-only utf8_lossy (maximal ill-formed subparts), form_decode, escape_html, Unicode whitespace trim and Unicode 17 scalar lowercase_chars (expansions, no contextual final sigma); it grants no effects.
Normal destructor order matches the supported Rust intersection; outputs/statuses stay distinct from traps. witness/bridge/output.rs observes bytes/status/trap kind separately.

### Declarative source macros

Module-scoped `macro_rules!` definitions produce ordinary source expressions or
items. Matchers accept literal token trees and `expr`, `ty`, `ident`, and `tt`
fragments. `*`, `+`, and `?` repetitions preserve nesting and require matching
capture lengths when several repeated names are emitted together. Separators,
optional trailing commas, recursive item producers, and nested macro calls are
supported. Expression and type captures remain distinct opaque fragments when
forwarded. Substitution preserves precedence; later literal-token or identifier
arms cannot inspect them. A `tt` capture forwards either as one opaque tree. Literal forms follow the ordinary source lexer; unsupported literal
prefixes fail before matching. Literal matcher tokens retain their exact spelling, including raw
string delimiters and byte escapes. Unused expression captures do not expand
nested macros or read includes. Unused type captures do not resolve their names.
Each emitted occurrence gets fresh syntax and ordinary checking at its use site,
including positional `impl Trait` obligations and constructor inference.
Empty nested repetitions retain their outer iteration counts.

A fragment that cannot begin at the next token lets matching try another arm.
Once its parse starts, a parse error is fatal. A type fragment must end at the
matcher boundary or precede `{`, `[`, `,`, `=>`, `:`, `=`, `>`, `>>`,
`;`, `|`, `as`, `where`, or a repetition with allowed first tokens.
Separated repetitions use the same follow set. Nested generic closers can split
`>>`, `>=` and `>>=`: the type owns its consumed prefix, the matcher retains
the suffix, and failed arms or repetition iterations restore the exact input.
Splitting preserves source origins and never joins separate tokens.

Captured identifiers, expressions and types retain their source syntax contexts.
Ordinary names inside expression and type captures resolve in the destination
module and `Self` environment. Nested macro calls inside captures retain their
original textual macro scope. Introduced local bindings have a separate syntax
context for each expansion; repeating a binding preserves ordinary lexical
shadowing. Block imports follow explicit block ownership and parent links, so
generated siblings remain separate even when their source anchors coincide.
Constant-initializer checks likewise follow expression ownership.
Unqualified template item names use the destination namespace, while `$crate`
names the defining source crate. Module `#[macro_use]` and `#[macro_export]` expose declarative
macros through source-module scope; exported qualified calls and
`macro_export(local_inner_macros)` retain the defining crate. All resulting
ownership, generic, capability and effect obligations use the normal frontend
and independent IR checker. Expansion adds no runtime operation or checker
privilege.

The current fragment set is deliberate. Other fragment kinds, procedural
macros, block-local or generated macro definitions, selective `macro_use`
imports, attributes on invocations, and expansion that discovers external
source modules have located refusals. Built-in macro names remain reserved.
Macro invocations in type, pattern, and multi-statement positions are not
admitted by this slice. Captured types work in ordinary type positions,
qualified receivers and generic arguments; they cannot become expression paths,
struct literals or trait bounds. `impl $t for Target` with a captured trait type
has a located unsupported diagnostic. Existing ordinary type and constant
expression boundaries still apply. Repetition heads must determine continuation without overlapping the
following matcher. Fragment ambiguity has a located diagnostic; pure literal
overlaps that Rust can disambiguate with its matcher automaton remain explicitly
unsupported. A repeated active invocation has a located recursion diagnostic;
ordinary allocation and representation rules govern expansion storage.

When expansion fails, source reports continue at independent invocation boundaries.
Each failed attempt rolls back its temporary syntax, generated items and capture
materializations, while the report retains its located error and blocked region.
Malformed definitions do not manufacture additional matcher errors at their calls.
Unused captures remain unexpanded during recovery, including their nested includes.
The partial tree retains the original refusal and is marked incomplete; it cannot
enter name resolution, checked lowering or an accepted semantic index.

The frontend delta-checkpoint format does not yet encode expansion dependency
and hygiene identities. Programs with actual macro expansions therefore receive
fresh frontend admission. Authenticated checked artifacts and source-unit
linkage retain their normal checks. Exact source replay binds destination and
textual scopes, structural block owners, split-token origins and every fragment
materialization. Captured types belong to their expansion's generated syntax.
Queries expose those references; renaming a declaration with generated capture
references refuses rather than publishing an incomplete rewrite. An authored
body may replace its type arguments through the checked edit path.
A macro definition or capture edit must reexpand the affected source before
its checked result can be published.
