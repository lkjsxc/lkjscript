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
then compound assignment reads the old value. Each operand executes once.
Traps terminate without cleanup or recovery; there is no unwinding.
Cases: [scalar bits](../tests/reference/scalar_bits.rs), [evaluation order](../tests/reference/evaluation_order.rs).

## Statements, control and patterns

Functions, structs, enums, constants, aliases, generics, trait implementations,
match, if, while, loop and for are implemented. Assertions, panic and vec forms
are fixed syntax. Unsupported forms receive named diagnostics.

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

The owned loader reads inline/file modules, child.rs or child/mod.rs, and nested
directories through the source Directory. Missing, ambiguous and duplicate inputs
have distinct diagnostics. File identities survive into IR and diagnostics.
include_str!("path") produces static UTF-8 text; include_bytes!("path") produces
a static reference to an exact-size byte array. Paths are literal, relative to the
containing file and beneath the granted source root. Missing/escaping paths reject;
invalid UTF-8 needs include_bytes. [Include case](../tests/reference/include.rs).

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
| Holder lifetime | A reference stored in a live view aggregate may retain its loan despite its last apparent source use. | End the carrier's lexical scope before borrowing the owner exclusively; this conservative rejection is intentional. |

Cases: [ranges](../tests/reference/range_views.rs), [UTF-8 trap](../tests/reference/text_boundary.rs),
[holder rejection](../tests/reference/holder_liveness.rs), [scope rewrite](../tests/reference/holder_scope.rs).

Wholly static outputs, including aggregates, lower to Static result contracts
checked on each live leaf. A named shared output lifetime without a source input
is universal and also requires static origin. Inactive leaves satisfy that contract;
absence of active input leaves does not prove output absence. Ordinary dynamic
outputs may shorten static literals. Explicit static input/local/field/alias/bound
constraints or mixed static/dynamic source output annotations remain unsupported;
direct IR can express mixed per-leaf maps. [Static view](../tests/reference/static_view.rs).

## Inference, generics and traits

Definition checking solves structural constraints over lexical bindings/expression
identities using rigid generic parameters and declared assumptions. Constraints
cover assignment/return, signatures, composite literals, fields, aliases, projections
and scalar patterns; expected types flow inward. Unsuffixed integers default only
after known constraints propagate. Concrete specialization uses that checked plan.
Insufficient generic/receiver context may still require annotations; there is no
promise of complete Rust inference. [Inference case](../tests/reference/inference.rs).

A solved type is not a coercion site. Some(r), an inferred generic field and
identity(r) move an unconstrained mutable r; later use is E_MOVE. Explicit
Option<&mut T>, view-field or result expectations can authorize a reborrow.
Aliases and nesting preserve this distinction.

Traits specify methods/associated types; implementations must be unique.
Signatures, prerequisites, supertraits, associated equations and instantiated
arguments check. Send/Sync/Data derive structurally; Data excludes capabilities.
Expanding generic-instantiation SCCs reject, including alias paths.
Bounds with extra type arguments remain unsupported. Method calls become
concrete function calls; trait spelling alone never grants builtin semantics.

Associated equations key on receiver variable, resolved trait and member, so
same-spelled members of different traits remain distinct. A unique structurally
compatible implementation may contribute conditional equations, never proof.
Selection still validates prerequisites and does not specialize unresolved caller
types to fit. Associated callable inputs can use declared trait evidence regardless
of generic parameter order. Missing/ambiguous/incompatible bounds reject.
Pending/cyclic nonstructural goals are not proofs; associated projections require
checked child witnesses.

Unused generic function/impl/default bodies and unreachable expressions check under
rigid parameters through production elaboration and ownership checking, without
sampling concrete types. Bounds, associated relationships, method signatures and
explicit arguments check. Selected trait/source methods and receiver dereference
counts survive specialization; newly applicable inherent methods cannot replace
them. Rigid verification types/functions never reach execution.
[Generic selection](../tests/reference/generic_selection.rs).

| Trait operation | Meaning and lowering | Rejection or rewrite |
| --- | --- | --- |
| Clone / PartialEq methods | Typed receiver/argument/result contracts; Copy provides Clone evidence and Eq provides PartialEq. Explicit builtin nominal implementations validate required methods, modes and arity. | Ambiguous builtin/source methods reject. Same-spelled user traits are distinct. |
| == / != | Evaluate operands once left-to-right. Read scalar left before RHS effects; borrow nominal owners. Same-type references compare referents. Explicit ne executes, otherwise negate eq. | Inherent methods/user PartialEq spellings do not replace the selected builtin contract. |
| Copy/Clone/PartialEq/Eq/PartialOrd/Ord derives | Validate structural fields, supertraits, conditional generic assumptions and coherence even when unused. Generated monomorphic functions use ordinary IR checking and memoized identities. | Duplicate/overlapping implementations and invalid fields reject. Debug and other unsupported derives report E_UNSUPPORTED. |
| Clone derive | Visit fields in order, selecting builtin traits rather than inherent methods. Nongeneric Copy nominal derives copy wholesale, including lifetime-only declarations; type-generic derives call field Clone even if a specialization is Copy. Explicit Clone still wins. | Default view-bearing clone_from needs a mutation contract. |
| Equality derive | Compare tags and short-circuit active fields in declaration order, including tuples/arrays/nested Vec/boxed recursion. Eq has no method. | Fields need the proper selected evidence. |
| Ordering derive | Nominal fields and enum declaration order; arrays/slices/Vec lexicographic. A decisive unordered float stops comparison and makes all four relations false. str/String compare UTF-8 bytes. | Ordering does not use custom PartialEq as an equality oracle. |
| Default derive | Ordinary generated implementation using field defaults under declared bounds. | Unsupported field evidence rejects. |
| Explicit Copy/Ord/PartialOrd/Data/Send/Sync | Not implemented as source impls. | Use supported derives/structural predicates, or ordinary methods. |

Default nominal/tuple/scalar clone_from uses clone, replacement and old-owner
cleanup; default ne negates eq and scalar eq/ne execute. Direct Box/array/Vec
clone_from is unsupported pending forwarding/reuse/truncation/order contracts.
General source trait-default execution is unfinished.

The executable remains monomorphic. Generic definition evidence is not yet a
complete independently replayable generic/witness IR contract; arbitrary hidden
reference content and external mutation effects are not fully modeled there.
Concrete specializations still receive complete IR provenance checking.

## Declared regions and closures

Fixed callable input regions create private definition-verification obligations.
Reference leaves through aliases/tuples/nominal fields must outlive required regions
using input facts and declared/implied outlives edges; local storage cannot prove a
universal input region. Helper calls transport fixed type-family leaf requirements.
A fixed closure input cannot satisfy a fresh higher-ranked input.

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
Opaque generic reference outputs, impl Trait closure returns, lending closures,
dynamic callables and scoped concurrency remain unfinished. Pending escape/three
concurrency cases specify intended ownership rules, with their missing features.

## Source library

| Facility | Source behavior and lowering | Limit or rewrite |
| --- | --- | --- |
| Text/String | Literals are immutable module-long UTF-8 views; str is unsized and shared. len counts bytes; is_empty/as_bytes/to_string preserve origins. String privately owns Vec bytes; Clone/clone_from copy valid bytes and reuse target capacity; equality compares length/bytes. | Bare/mutable text rejects. as_str currently revalidates UTF-8. |
| str::from_utf8 | Checked runtime validation returns source Result<&str, Utf8Error>, borrowing original bytes. Utf8Error is an opaque marker. | No unchecked conversion or error-position/length API. |
| Text methods | Prefix/suffix, contains/find/rfind, ASCII case/comparison, repeat/replace, bytes/chars, LF/CRLF lines, strip_prefix, split_once and split (including empty patterns). Pattern shares text/char needles; integer decimal parse covers ten types with validity/range checks. | General Unicode case conversion is unsupported. Borrowed iterators retain owners. |
| Character helpers | UTF-8/UTF-16 lengths, ASCII predicates/case, Unicode whitespace/control, to_digit and to_string. | Operate on Unicode scalars, never arbitrary code points. |
| Formatting | Arguments evaluate once in order; macros call source String/Formatter traits. Display, x/X/b/o, positional/named/captured arguments, escaped braces, fixed width, Unicode fill, alignment, sign/prefix/zero padding. Explicit zero width remains distinguishable. | write!/writeln! target String (Write in scope) or Formatter; arbitrary Write sinks, Debug, float formatting, dynamic width/precision are unsupported. Errors stop later formatting; format! traps on failure. |
| Option/Result | Canonical source declarations and re-exports; conditional derives, views, owned and/or/transpose and selected map/default combinators. Eager arguments evaluate before call, lazy callbacks only on selected variants; unused captures/payloads still clean up normally. | Some nested receiver expressions need annotations. Option::expect evaluates its message but retains fixed trap text. |
| Option extras | xor, zip/unzip, flatten, replace/insert/get_or_insert, is_some_and/is_none_or, ok_or/ok_or_else. None predicates yield false/true respectively. filter borrows the payload then keeps/drops the owner. | Returned views preserve selected input/factory origins. |
| Option<&T>/Option<&mut T> | copied uses Copy without Clone; cloned invokes Clone once for Some, never None. | Owned clones may be independent, but nested views retain external origins. |
| Vec/slice access | is_empty/get/first/last and mutable slice counterparts; invalid get returns None, including usize::MAX. contains borrows query/elements and uses eq in order without copying/consuming. | Returned references retain collection footprints. |
| Collection copying/equality | extend_from_slice/to_vec clone in order; empty input does no clones. Vec Clone requires Clone; equality requires only PartialEq, checks length then element ==, and Vec ne negates the whole result. Nested types/floats/custom implementations follow selected traits. | Inherent clone/ne cannot replace generic contracts. Helper signatures and element constraints check independently. |
| Slice/Vec helpers | starts_with borrows; copy_from_slice needs Copy and equal lengths; predicate split retains empty groups at delimiters. clear/truncate destroy removed elements forward; stable sort/retain are source operations. | General From/Into, Sum/FromIterator and exclusive iter_mut need their own contracts. |
| Ordered maps/sets | BTreeMap uses an indexed AVL arena/free slots with logarithmic lookup/insertion/removal; BTreeSet reuses it. Sorted shared/owned iteration, map keys/values/first_key_value/pop_first/retain and set first/pop_first. Clear/drop and partially consumed iterators destroy in key order; Clone/Default/equality ignore arena shape. | Entirely source algorithms over existing storage, not new trusted primitives. |

Option/Result also provide map, map_or/map_or_else, and_then/or_else/unwrap_or_else;
Option has filter and Result map_err. String ordering/from conversion, scalar
ASCII helpers and min/max are ordinary source conveniences, not general trait evidence.
[Formatting case](../tests/reference/formatting.rs).

Iteration resolves source Iterator::Item/next and IntoIterator evidence.
Collection for holds iterable temporaries in an outer match, loops over next and
binds each item with an irrefutable let. Non-unit bodies/refutable patterns reject;
break/continue/return/exhaustion preserve item/iterator cleanup. Numeric ranges
share binding machinery. No iteration IR operation or nominal-name shortcut exists.
Shared Vec/slice/map/set and owned Vec/Option/map/set iteration are supported.

enumerate/skip/take/zip are source adapters; chain currently needs identical
iterator types to prove Item equality. map/filter/filter_map/inspect store the
iterator/callable. filter/inspect borrow fresh items; map/filter_map transfer them.
Adapters follow next even after a previous None and do not silently fuse.
fold/for_each stop at first None and consume in order; fold owns its accumulator.
any/all borrow the iterator mutably, pass items by value to FnMut and stop at a
decisive result/None; empty answers are false/true. Iteration resumes after that
consumed item; callbacks/captures drop on return and remaining items stay owned.
find borrows predicate inputs and returns the matching owned item; position
consumes and returns an index. Receiver evaluation precedes callback construction.
Owned temporary receivers, including projected/indexed values, are writable without
making named bindings or shared references mutable.
[Iterator/Option case](../tests/reference/iterator_options.rs).

## Checked IR and execution

IR has explicit types, signatures, locals, places, copy/move/borrow/drop and
structured control. Every type/signature/operation shape checks, including dead
code; source inference/trait search never occurs below admission.
Only CheckedProgram can reach emission; optimizer output must be rechecked.
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
| lkjscript build PATH [-o FILE] | Static executable, default PATH.elf; temporary publication plus rename preserves an existing destination on rejection. |
| lkjscript run PATH [--entry NAME] | Execute main, else probe, with native default or --engine interpreter. |
| lkjscript test PATH | Discover expectation cases or execute #[test]/IR test functions across native/interpreter routes; details on failure. |

Program arguments after -- remain exact bytes; compiler arguments require UTF-8.
Scalar runs print integer decimal, bool text, UTF-8 char (including NUL), or
f64:0x plus 16 lowercase hex digits and newline; unit adds nothing.
Native scalar results use a driver-owned raw eight-byte descriptor. Trap diagnostics
use that channel separately from program stderr; standalone artifacts report traps
on stderr and exit 1, preserving prior output. Both routes format scalars identically.

--stats reports phase milliseconds, IR/code counts and peak RSS. --source-dir grants
the source root; --library-dir/--runtime-source replace embedded components explicitly.
--help gives options. Test expectation headers declare value, output bytes, diagnostic
or trap; pending headers name the missing intended feature. The gate compares exact
values/bytes/status/file effects and trap kinds; exhaustion locations may differ.
Terminating cases use generous watchdogs, mutants a separate short allowance.

Parser recovery synchronizes balanced item/method/statement boundaries, marking
skipped regions E_BLOCKED. Recovered source never authorizes emission. With valid
shared signatures, independent inference/IR body errors accumulate; each failing
body stops at its diagnostic. Diagnostic-only placeholders preserve known signatures
for dependent checks, never executable authority. Shared resolution/signature errors
block dependent work; no fabricated types or ownership facts continue analysis.
Single-error APIs select the first diagnostic from these same production paths.

## World and capabilities

Entries currently take no argument or World; ordinary calls supply their arguments.
World, Stdout/Stdin/Stderr, Directory, File, Process, Clock and Child are affine
intrinsic types with no source constructor, Copy/Clone/Data/Send/Sync implementation.
Names, integers or casts cannot forge authority. Every capability operation checks
independently in IR; methods below exclusively borrow the receiver. Derived owners
can outlive that borrow and move normally.

| Form | Result and authority |
| --- | --- |
| World.args() | Vec<Vec<u8>> excluding argv0. |
| World.stdout()/stdin()/stderr() | Corresponding inherited stream capability. |
| World.directory(path) | Result<Directory,i64> acquiring a root. |
| World.process()/clock() | Process or monotonic-clock capability. |
| World.temporary_file() | Result<File,i64>, anonymous read/write storage. |
| World.stdin_file()/stdout_file()/stderr_file() | Owned inherited-stream duplicate, preserving closed streams. |
| World.executable() | Read-only running executable at fresh offset; checked interpretation prepares a native image lazily, unchecked IR returns ENOSYS. |
| World.set_exit_status(u8) | Set normal status and continue cleanup/execution; last value wins. |
| Directory.open/create(path) | Read or create/truncate a writable descendant File. |
| Directory.open_directory(path), list() | Descendant Directory; Vec<Vec<u8>> names excluding dot/dot-dot, unspecified order. |
| Directory.create_directory/remove/remove_directory(path) | Create directory, unlink entry or remove empty directory. |
| Directory.rename(old,new) | Rename within root, replacing destination as Linux allows. |
| Stdin/File.read(&mut [u8]), read_to_end() | Result with short-read count or collected bytes; zero is EOF for nonempty buffers. |
| File.write/write_all(&[u8]) | One-write count or complete-write Result. |
| File.duplicate()/readonly() | Duplicate sharing offset/state; or read-only reopen with independent zero offset. Closed remains closed. |
| File.rewind()/set_executable() | Seek zero or set mode 0755. |
| Stdout/Stderr.write(&[u8]) | Complete write or R0007 at the call. |
| Clock.nanoseconds() | Result<usize,i64> monotonic differences/deadlines. |
| Process.spawn_file(executable,&args,stdin,stdout,stderr,result) | Result<Child,i64>, explicit executable File, argument byte vectors and four File streams; result maps to descriptor3. |
| Child.wait()/try_wait() | Exit code or 128+signal, with Option for polling; reaped children later fail ECHILD. |
| Child.terminate() | SIGKILL then reap; dropping a live Child also kills/reaps its direct child. |

Convenience methods are source in library/io.rs; Result failures carry positive Linux
errno, raw read/write leaves negative errno. EINTR retries except close, which
consumes once. Paths reject NUL; raw paths need exactly one trailing NUL.
Descendants use openat2 RESOLVE_BENEATH|RESOLVE_NO_MAGICLINKS without weaker fallback;
absolute/escaping paths fail. Mutations derive a parent then validate the basename.
New files/directories use 0666/0777 subject to inherited umask.

Captured process files share offsets; rewind before reading. Closed File returns
EBADF, drops harmlessly and closes the assigned child stream. Spawn validates
NUL-free arguments, chooses argv0 lkjscript, clears environment and executes the
File with execveat, without PATH/shell fallback; exec failure returns errno.
Direct children receive parent-death SIGKILL. No caller slice pointer survives I/O.
File/Directory drops close once; standard-stream capability drops leave inherited
streams open. Acquiring another capability preserves inherited closed streams.

Generated programs have no ambient filesystem/environment/process imports or
source-declared host imports. Normal destructor order matches the supported Rust
intersection; outputs and normal statuses stay distinct from traps. The optional
witness/bridge/output.rs adapter observes bytes/status/trap kind separately.
