# Language

This file describes the implemented generation-two subset. Status names the
unimplemented parts of the intended language.

## Source and values

The surface is Rust-compatible. Integer literal fallback is i32; annotations and
suffixes retain distinct types, including usize/u64 and isize/i64. Arithmetic is
checked. Signed minimum literals are accepted; out-of-range integer and non-finite
floating literals are rejected. Explicit casts follow Rust conversion behavior.
Named wrapping operations remain source-library work. Booleans are distinct from integers. Expressions evaluate
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

Integer and byte literal patterns support inclusive, half-open and open ranges:
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
source order. Unguarded alternatives with uniform binding paths, and independent
value-binding choices, share their source tree, including irrefutable let bindings.
Guards cannot assign or mutably borrow a tested place or its ancestor; unrelated
untested fields remain writable. Copy bindings are frozen too. A wildcard does
not inspect storage. Guarded alternative products and nonuniform reference
binding joins still need shared continuations; status records that code-growth
defect. Targeted K loop transfers preserve enclosing source loops when lowering
needs an internal loop; invalid depths reject even in unreachable K.

Enums retain distinct unit, tuple and named-field variant shapes. Named constructors
evaluate initializers in written order and store payloads in declaration order.
Named patterns support reordered fields, renamed/shorthand bindings, wildcards and
rest patterns. Constructor and pattern fields are checked for names, completeness,
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
expanded constructors.
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
and parent dependencies. A result can declare a separate input leaf or immutable
static origin for every output leaf; exact maps preserve the input referent while
bounded maps permit its descendants. The checker requires complete, unique leaf
coverage and validates calls and returns independently. Source rigid T input-to-T
transport preserves its abstract leaf origins; equal concrete types alone do not
authorize that correspondence. Named lifetimes map when each source is unique.
Alternative input choices and repeated generic input candidates remain conservative.
Borrow endings and cleanup are explicit in K.
Whole-array/Vec slice projections, view structs and implicit mutable reborrowing
are implemented.

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
Eq, `push_str`, `to_string` and `as_str` in source. Cloning copies already-valid
bytes directly; clone_from reuses the target buffer when capacity suffices.
Equality compares byte length and contents. `as_str`
currently validates on conversion; retained UTF-8 evidence for avoiding that scan
remains future work.

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

## Generic declarations and traits

Traits describe methods and associated types. Implementations must be unique;
overlap is rejected. Full implementation signatures, prerequisite/supertrait
bounds, associated types and instantiated arguments are validated. Send, Sync and
Data predicates are derived structurally; Data excludes capabilities. Generic
parameter-flow strongly connected components reject expanding instantiation.
Aliases participate in these checks. Bounds with extra type arguments remain
unsupported. Method calls resolve to concrete function applications.

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

## K and checking

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

Wasm lowering maintains initialization flags for roots used by conditional
destruction and clears moved owned payload slots. Consumed call arguments do not
remain initialized merely because their old bits remain in a physical slot.
K cannot supply a flag value. The independent machine tests its own cell state.
Conditional cleanup runs at the original destruction or replacement point;
it does not move destruction to a branch join. Initialization joins intersect
definite roots and union possible roots, with corresponding must/may projected
holes. A later read requires definite initialization; overwriting a possible
owner, strict destruction of a maybe-owner, projection through an absent shell
and leaking a possible owner all reject. Reference inventories and loop
backedges still require equality until their own contracts are implemented.

The checker validates all type references, signatures and operation shapes,
including dead code, then checks ownership through each function. Variant payload
access needs a proven tag. Moving, dropping or replacing a payload field preserves
the enclosing discriminant proof; replacing the enum, its tag witness or an
ancestor invalidates it, including predicates saved before a branch. A loop back
edge must reproduce entry ownership state.
Return and break edges must carry correct live owners and loans; no fixed-point
iteration is used. Local storage or a local reference carrier cannot become the
source of an escaping view. The checker performs no source inference or trait
search. Only CheckedProgram reaches the backend; later stages rely on that gate.

## Memory and execution

Checked K lowers to target-independent typed operations with layout, calls and
copy/drop glue explicit. The Wasm encoder consumes this form without consulting K.
Memory scalars use their natural one-, two-, four- or eight-byte widths; signed
loads extend correctly. Registers and pointers retain 64-bit representations.
F64 retains its bit representation and canonical NaNs after arithmetic.
Collection indexing checks bounds. Traps report a code and source line. The
reusable first-fit allocator and structural/custom drop release owned fields,
arrays, boxes and vector elements. Structural cleanup uses an explicit growable
DFS stack, preserving custom-destructor-before-children and declaration/element
order. Moved fields are skipped. Copy helpers, recursive generated Clone/equality calls
and explicit recursive source calls still use the engine stack. Loop-condition
temporaries end before either loop edge, including index locals and owned receivers.

Lowering reuses physical Wasm locals across nonoverlapping lifetimes. Additional
live values spill into a growing memory64 frame; address-taken values have stable
frame slots. The physical register pool is an optimization, not an acceptance
limit. Source-local count does not determine the permanent Wasm-local count.

Struct payloads align fields at their natural widths; enum payloads additionally
carry a tag. Vec and array storage has data, length and capacity words, with
element strides from the one layout contract. A Vec<u8> element occupies one byte. Whole-storage slice projections borrow that header without a
separate allocation. Moved heap slots are cleared so partial cleanup skips them.
These layouts are implementation details, not an exposed source API.

Generated functions export as `fn:<K-name>`, avoiding a collision with the
`memory` export. The Node route executes this encoding. The independent K
machine executes checked K directly over its own heap, explicit call/control/drop
stacks, initialization states and exact storage/loan identities. It validates
reference tags and active variants on actual access and distinguishes M_* machine
errors from R* program traps. It shares neither the checker implementation nor
WebAssembly lowering. This dynamic search is not a proof or a full Rust memory
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

`compiler::compile(&SourceBundle)` is the source-to-Wasm compiler-core entry.
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
`src/wasm.rs` contains lowering/emission plus allocation and collection internals.
`bridge/main.rs` supplies bootstrap file/process/command-line access and the trusted Linux main-stack shim;
`bridge/run.mjs` instantiates on Node. `lkjscript trusted` lists these boundaries.

Generated modules have no ambient imports. A module using a checked Stdout value
imports only the byte writer. Language traps record a code and source line in
module globals and execute WebAssembly unreachable; the host translates that
record after the trap. Trapping requires no host authority.
Ordinary source cannot declare host imports or construct capabilities from scalars.
Ambient filesystem, environment and print operations are rejected. World-derived
standard output is specified below. Other capabilities, closures and structured
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

The WebAssembly host boundary receives a packed byte pointer, length and source
line. It checks the entire range without overflowing, copies chunks before
writing, loops on partial writes and retains no bytes or pointer after returning.
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
blocked regions and observed name/type/trait dependencies; blocked method bodies
are not counted as successfully resolved. Missing imports can explain dependent
unresolved type names.

The bootstrap batch command compiles each case once. K uses a fresh machine per
case; Node loads fresh modules in one process through the same engine and byte
writer implementation. Cases retain their own output bytes and termination
classes. Rust reference output is cached by generated source, generator/seed,
compiler version and flags. Generators and reduced failures are source records;
detailed measurements remain under target/.
