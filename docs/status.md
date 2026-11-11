# Status

Generation two is a runnable Rust-bootstrap compiler with a memory64 WebAssembly
backend. Whole-compiler source acceptance, a compiler fixed point, an owned CLI,
and independence from Rust/Node remain unestablished. Generation one is frozen on
`generation-one` at `05a9326c` and in the read-only `.generation-one` worktree.
Running deployments are independent and unchanged.

## Works

Functions, checked fixed-width integers and f64, structs/enums, matches, loops,
generic instances, associated types and prerequisite bounds execute on Node.
Ownership includes moves, partial initialization, custom Drop, disjoint mutable
fields, one-source views, array/slice projections and reborrowing.
Ambient I/O and direct calls to actual destructor functions are rejected.

Generic free functions, impl methods and trait defaults are checked under rigid
parameters and declared contracts, independently of specialization or reachability.
The production elaborator and K checker are reused. Bounds, affine ownership,
associated identities and explicit method type arguments are enforced; inherent
methods and user traits do not become builtin Clone/Drop/Copy by spelling.
Concrete associated-result selection executes successfully. Checked method choices
and receiver dereference counts survive specialization; newly applicable inherent
methods cannot replace generic trait calls. Builtin Clone/clone_from/eq/ne have
typed definition contracts, and concrete source implementations execute. Symbolic
bounds preserve builtin versus declared trait identity. Derive declarations
now check all structural field and supertrait obligations, including unused
generic layouts. Explicit builtin Clone/PartialEq signatures and Eq evidence are
validated; unsupported derives/overrides are named. Explicit source PartialEq
implementations execute through ==/!=, with exact trait selection and operand
order. Derived Clone/equality execute as ordinary checked K functions, with exact
field trait evidence, recursive header interning and a shared source/generated
body queue. Struct/enum/tuple/array/Box shapes and nested source Vecs execute.
Default nominal/tuple/scalar clone_from, default ne and scalar comparison methods
execute; container and view mutation boundaries remain explicit.

Integer range patterns support typed inclusive/half-open/open bounds and exact
interval-based exhaustiveness. Borrowed scalar matches read through references.
All arms are checked, even after an unconditional default; affine default bindings
retain first-arm execution and exactly-once cleanup. Named-field enum constructors
and patterns preserve declaration identity, written evaluation order, typed field
layout and actual storage for ref/ref mut bindings. Distinct fields of the same
proven variant are disjoint. Nonmoving matches preserve owner/destructor timing;
mixed affine moves and explicit refs on an existing place remain named unsupported.
Payload moves and field writes preserve the enclosing enum tag; whole-enum or
witness mutation invalidates saved predicates and proven tags.

Statement and match-arm parsing distinguishes unparenthesized block forms from
ordinary value expressions. Implicit block statements require unit; explicit
semicolons discard values with cleanup. Ordinary code after return is checked
just like unused generic bodies.

Every initialized reference leaf has provenance or explicit inactive-variant
absence evidence. Moves, construction, cloning, replacement, field assignment,
call results and return preserve/check it. Replacement transports both the old
and installed values. External writes cannot silently change symbolic reference
leaf origins; explicit mutation contracts remain required for general updates.
The original replacement repair is committed and CI-verified at `35a5d862`.
K now also declares projected parameter sources for shared references stored behind
borrowed carriers. Callees must return the selected source leaves, and callers
preserve those underlying origins. Shared reference cloning no longer acquires a
false dependency on a temporary descriptor. Early cleanup retains immediate
carrier dependencies and drops dead chains in dependency order. Explicit source
projection paths avoid numeric repetition expansion, including cyclic K types.
Exclusive projected return sources still need explicit reborrow/transfer contracts.
Each leaf now retains exact origins or conservative source bounds. Reborrowing an
opaque call result preserves its full footprint; return checks validate every
candidate rather than granting its common ancestor. An opaque reference-bearing
write requires an exact destination contract. Scalar writes preserve sibling
reference evidence and use their actual access-place tag witness. These changes
repair two reproduced source/direct-K alias/provenance errors.

Files, modules and declarations have explicit identities. The loader owns file
discovery; resolution handles lexical/qualified names, aliases, imports, visibility
and prelude shadowing. The library uses ordinary module scopes. Private String
bytes cannot be constructed or accessed by outside clients. Type/instance keys
are structural, with compact diagnostic names. Generic parameters are resolved
by their original lexical/file scopes rather than global name guesses. An
unrelated enum E no longer changes the source Result implementation, qualified
crate::T is not substituted by parameter T, and str cannot become an inferred
parameter in a call. Alias constructors follow the resolved struct or enum, with
fresh parameters, lexical inference and underlying privacy. Alias-aware type
inference uses explicit work frames. Partially known nested alias targets still
need explicit arguments or field annotations in some Rust-valid programs; the
unchanged boundary fixture and both working annotations remain in the suite.

`compiler::compile(&SourceBundle)` takes declared source/library bytes and returns
Wasm bytes or diagnostics without host I/O. The CLI uses the same production
checking pipeline. Bootstrap byte packing is in `bridge/library.rs`. Supplied
library bytes affect emitted behavior; missing inputs are errors. This is a
working compiler-core boundary, not a self-hosting claim.

The source library implements Option/Result, bytes/UTF-8, String, vector cloning,
equality and ordering, and a sorted-vector BTreeMap (logarithmic lookup, linear
insertion/removal). Vec cloning invokes element Clone; equality uses element ==
and whole-vector inequality negates that result. Equality no longer requires Ord.
Direct operations and retained generic calls resolve the same source functions;
missing library inputs, altered signatures or missing element bounds reject. String
Clone, clone_from, PartialEq/Eq, appending and text projection use source functions.
Source Option and Result derive conditional Clone/Copy/PartialEq/Eq through the
same general implementation. Clone copies known-valid bytes and clone_from reuses
available target capacity.
Immutable &str values
support literals, byte length/projection and checked UTF-8 conversion; no second
owned String representation was introduced. Integration tests load actual source
bundles under lkjscript and use the same components under rustc.

K text literals carry checked UTF-8 bytes and immutable static provenance. Function
results explicitly name a parameter or static source. Static/dynamic leaves can
share a shorter dynamic lifetime; static requirements on individual source leaves,
inputs, fields, locals and bounds remain named unsupported. Trait implementations
cannot weaken a declared static result. Checked runtime conversion returns an
opaque Utf8Error marker on invalid bytes; String::as_str currently revalidates.

Lowering reuses physical locals and spills excess live values into memory64 frames.
Address-taken values have stable frame slots. Structural destruction uses a growing
explicit stack, including custom Drop, partial moves and recursive Box/Vec values.
Copy helpers, recursive generated Clone/equality and explicit recursive source
calls still use the engine stack. Assignment through borrowed/indexed storage
uses checked replacement, with RHS-before-place evaluation and exactly-once
cleanup. Loop-condition index and owner temporaries end before either loop edge.

The exact production unsigned/signed LEB128 encoders in `src/encoding.rs` now
compile and execute through the ordinary source-bundle pipeline. Tests compare
every encoded byte for 34 unsigned and 63 signed boundary inputs, include fixed
known vectors, and require a behavior-changing source mutation to alter the
artifact and result. An invalid mutation fails with the original source filename.
The exact production canonical-name helper in src/names.rs is now shared by
resolution, generic validation and elaboration and executes with the source String
library. Complete byte comparisons include empty, ASCII/control, NUL, multibyte,
maximum Unicode and roughly 5.6 KiB inputs. Separator and ill-typed mutations
change generated behavior or reject with names.rs diagnostics. These executing
components still use emitter-provided memory primitives and Node; they do not
establish a compiler-core fixed point.

The actual trap-metadata encoder now calls shared src/json.rs::write_string. That
exact source executes under lkjscript with range patterns and byte loops. Five
tests cover every emitted byte, independent JSON decoding, the actual Wasm custom
section, a behavior-changing boundary mutation and an invalid range with its
original filename. The former emitter-local quoting loop is removed.

The complete unchanged src/k.rs representation module now compiles and executes
with its actual Clone/equality consumers and source-library inputs. Three component
test groups cover 172 same-source Rust/Node observations over all Op/TypeKind
variants, nested Program/Function records, Option<ReturnSource>, tuple/vector
fields, projections and surviving original-owner cleanup. Mutating primitives
and Place::local changes artifacts and behavior; an ill-typed body reports src/k.rs.
This closes the representation component, not the parser/checker/emitter closure;
the probe treats Op records as data and does not interpret them as K instructions.

## Current verification

The integrated 2026-11-11 production-K/source-bound milestone gate passes **361 Cargo tests**
and reports **103 conformance passes, eight pending, zero failures; 96 rustc comparisons**.
Formatting, Clippy, build and conformance are included. Elapsed: **18.670 s**.

The suite includes 19 provenance tests, 28 generic-definition tests, 20 module
tests, source-bundle and executing-component mutations, parser/embedding capacity
cases, two 30,000-local shapes, deep destruction/order and allocation reuse.
Direct K fixtures reach the checker without frontend filtering. Rejections use
the intended semantic errors; source-level replacement/clone escapes and generic
definition/type-argument failures are also compared with rustc. Text adds 27
checker/source tests, two direct-K backend tests and three production component
tests. The backend validates 72,458 byte inputs against Rust, including every
one/two-byte sequence and longer UTF-8 boundaries, through K checking, emission,
Node instantiation and execution. Static-region bytes remain unchanged across
1,000 allocation/drop cycles; a literal pool exceeding one page loads correctly.
These are regression observations without peak RSS measurement. A numeric lexer
regression also prevents usize/isize suffixes or hexadecimal e digits from being
mistaken for decimal exponents or absorbing adjacent signs.

Eleven range-test groups compare 2,351 observations from identical source compiled
with rustc and lkjscript. They cover every u8/i8 value, selected i64/u64 extrema,
borrows, mutable-reference reuse, inference, overlap/order and affine cleanup.
Invalid bounds, coverage gaps and invalid unreachable generic arms reject for the
intended reason. Wide-domain boundary samples are not exhaustive 64-bit testing.

Twelve named-enum test groups include 14 same-source rustc/Node execution probes
over three files, plus direct K alias/tag checks and intended semantic rejections.
Nine generic-scope groups exercise library shadowing, original-file scopes,
qualified nominal identity, disjoint implementation coherence and actual trait
witness selection. Fifteen derive groups check unused field obligations,
conditional generic validation, supertraits, explicit builtin method contracts,
coherence, original-file diagnostics and named unsupported boundaries.

Eight String trait groups compare complete bytes over eight empty/control/NUL/
Unicode inputs and 64 equality pairs, preserve independent owners, reject invalid
borrows and require source clone/eq/ne mutations to change artifacts and behavior.
Six equality groups include 15 same-source Rust/Node probes for trait identity,
operand order, references, destruction and ownership. Ten builtin-contract groups
verify retained method choices, generic ambiguity, executable-body boundaries,
absence of abstract verification artifacts and explicit cyclic-reference paths.
Four symbolic trait-identity groups compare declared versus builtin constraints
with Rust. Twelve projected-return groups include 16 same-source Rust/Node probes,
direct K forged-source rejection, carrier destruction, reference-parent retention,
branch reassignment and loop exits. Paths of lengths 1, 2 and 64 through a cyclic
K reference type check and round-trip; those paths were not executed or scaled.

Seven block-statement groups include 34 same-source rustc/Node observations,
unit obligations, explicit-semicolon destructor behavior and invalid dead code.
Seven alias-constructor groups include 27 same-source runtime observations,
14 Rust-matched rejections and one explicitly retained Rust-valid inference
boundary. Nine Vec-trait groups cover complete String bytes after original-owner
cleanup, nested independent clones, affine method selection/destructor order,
floats, trait scope and source-helper mutations that change artifacts and results.
Lawful element implementations use Rust Vec as an oracle; deliberately inconsistent
eq/ne implementations compare the exact source helper and the specified lkjscript
rule. Two direct K groups distinguish payload operations from invalidating whole
enum mutations, including mutation before and after a tag branch.

Five derived-operation groups cover field trait selection, conditional Copy
behavior, recursive containers, defaults and loop-condition cleanup. Seven exact
source-prelude groups include 11 Rust/Node probes plus payload/prelude mutations.
Five projected-assignment groups include 15 Rust/Node observations for ownership,
RHS/place order, contextual typing and drop behavior. Five derived-provenance and
six source-bound groups cover multi-leaf forwarding, all-candidate return checks,
opaque reborrows, exact storage identity, source/direct-K escape rejection, static
fallback and destructor inputs. The formerly unsupported derived vector-holder
case is now executing coverage. Conditional present/absent view joins remain an
explicit Rust-valid E_MOVE boundary, independent of Clone.

The eight closure/callback/task conformance cases remain explicitly pending.
Structural Send/Sync checks do not establish a task runtime. No World-backed
I/O or concurrency execution pass is claimed.

## Capacity observations

These 2026-11-11 debug CLI observations were taken after the backend repair,
before the following generic/type-frame changes. They include parse,
resolve/elaborate, check, emit, Node load/compile, execution and cleanup.
Each is one end-to-end observation; peak RSS was not measured.

| Input | Result | Elapsed |
| --- | ---: | ---: |
| 30,000 successive local declarations | 29,999 | 0.253 s |
| 20,000-node Box/enum list | 20,000 | 0.023 s |
| 20,000-node custom-destructor chain | 20,000 | 0.024 s |
| 20,000-node Vec/enum chain | 20,000 | 0.027 s |

The repeated custom-destructor probe used 5,963,776 linear-memory bytes after its
first call and 9,109,504 after its second, then stayed unchanged through calls
three to five. The allocator still does not split/coalesce blocks; stable reuse
after initial placement does not establish absence of fragmentation.

Nested Box measurements after structural interning at `368cbb37` cover parse,
resolve/elaborate, check and K serialization, not emission/execution. Structural keys and compact names remove the earlier
approximately quadratic expanded-name output for this input shape:

| Depth | Source bytes | K-text bytes |
| ---: | ---: | ---: |
| 128 | 678 | 16,933 |
| 256 | 1,318 | 20,133 |
| 512 | 2,598 | 26,533 |
| 1,024 | 5,158 | 39,433 |

The subsequent parser/type-elaborator repairs replace the independently observed
depth-256 stack failures with explicit work frames. On an explicit 2 MiB thread:

- Parsing and AST destruction pass at depth 16,384; routine regressions cover
  eight simple/mixed shapes at depths 512 and 8,192, including malformed input.
- The ordinary compiler-core entry compiles builtin Box and transparent-alias
  graphs at depths 512 and 2,048 through emission; Node loads them and executes
  the scalar probe. An uncalled `consume` function carries the deep parameter type.
- User-defined Box types and reverse-ordered nominal fields pass at depth 512.
- A recursive alias/nominal/Box program builds, executes and cleans 1,000 nodes.
- Alias-only cycles reject semantically; no depth/fuel threshold was introduced.

Alias-aware structural type inference now also uses explicit work frames; no
new depth measurement is claimed for it. Other expression/pattern parsing,
elaboration and trait/signature traversals remain outside this stack repair.
Complete stack independence is not established.

Reference containment in K is now one reverse-edge graph propagation rather than
a fresh transitive scan for each container. Directly constructed nested-Box K
type tables gave the following single debug-library observations (K check only;
no source parsing, elaboration, emission or execution):

| Types in Box chain | Before | After |
| ---: | ---: | ---: |
| 1,024 | 5.583 ms | 0.274 ms |
| 4,096 | 85.892 ms | 0.942 ms |
| 8,192 | 345.529 ms | 1.848 ms |

### Earlier evidence and its original scope

At `aff27e54`, Rust 1.97.0/Node 24.18.1 measurements reported incremental/fresh-target
gates of 1.724/3.994 s for the then-94-test suite. Release million-parentheses,
million-local and million-function probes measured checking through K, not emission
or execution: 0.144 s/218 MiB, 1.178 s/833 MiB and 2.322 s/1,396 MiB sampled peak
RSS respectively. These were not repeated for the current changes.

The earlier 2.125 GiB Vec execution reported 5.075 s/6,327 MiB sampled peak RSS,
retaining values across addresses above 4 GiB and growing linear memory to 8 GiB
plus one page. The earlier 50,000-unary-operator check under an external 1 GiB
address-space limit reported 0.276 s and a 766 MiB main stack. Neither was repeated.
Earlier `fib(35)=9227465` single observations after startup were 25.702 ms for Wasm
and 28.781 ms for rustc checked-arithmetic native code. These are not averages,
complete compiler benchmarks or an owned-engine result.

## Missing and next work

Executable K is still monomorphic. Generic validation retains checked method
selection through specialization, but complete witness-bearing generic K and
serialized contract closure are unfinished. Source trait-default execution remains
named unsupported. Direct container clone_from needs element-update forwarding
and source Vec reuse/truncation/append semantics; default view clone_from needs
provenance-preserving mutation contracts. Explicit source bodies and the
implemented builtin defaults execute. Reference content hidden inside arbitrary type
parameters still requires concrete K provenance validation. Deeply dependent
associated-witness chains are not comprehensively normalized; unresolved cases
reject. External view writes need explicit mutation contracts before their
symbolic source relationships can change.

The normal CLI starts from `src/lib.rs` and follows its real modules. The current
whole-crate check stops at `src/source.rs:38:32` with `E_PARSE` on the tuple closure
parameter in `map(|(p, _)| p)`. Range patterns no longer cause the earlier stop. No complete compiler-core closure is accepted, and no W1/W2,
N1/N2 or owned bootstrap seed exists. Text/collection operations beyond the implemented subset, formatting, map/set
APIs, derives, closures, patterns and ownership layouts remain dependency work.

The ordinary whole-file check of src/k.rs now succeeds, and the unchanged module
executes with its required cloning/equality consumers. K and private checker-state
Debug derives had no production formatting consumers; their earlier removal
preserved structural comparisons and text-K failure output. The next whole-core
work is the actual src/source.rs loading dependency graph: iterator/closure uses,
source map/set operations and their parser/AST dependencies. Keep the production
K component in that closure; do not substitute a constructor-only copy or bypass
semantic passes to increase acceptance counts.

A concrete ownership follow-through is place-based tag witnesses and branch
partial-move/cleanup contracts for mixed affine and explicit-reference matches.
The current supported cases bind real storage and retain the established
consuming path; the unsupported combination is not simulated with moved copies.

Other observations include missing borrowed collection iteration, `while let`,
and repeated Vec initialization. Add these when they close the next actual
component dependency. Keep the successful encoding component and extend its
production closure rather than counting parsed files.

Vec/Box/allocation internals remain in the emitter. The one-source view model,
views-in-owned-containers restriction, closures, explicit World capabilities,
tasks, CLI ownership and an owned/native execution route remain unfinished.
Choose generally useful semantics over repeated compiler-only source contortions.

## Bootstrap

`cargo build --release` builds C0; `bash tools/check.sh` is the routine gate.
`lkjscript run file.rs --entry probe` executes on Node. `build -o file.wasm` emits,
`k` serializes checked concrete meaning, and `check file.k` reaches the K checker.

The first compiler-core fixed point must run C0's language pipeline on the exact
production source/library closure to emit W1, then execute W1 with the same inputs
to emit equal W2. Conformance and behavior-changing compiler-source mutations are
also required. Rust directly compiling to Wasm is not this route. Node and a small
host adapter can remain at that scoped milestone. Native closure and independence
from Rust/Node require separate evidence, including an isolated rebuild from a
named owned seed. Shared-host tools and deployments stay intact.
