# Status

The Rust bootstrap checks monomorphic K, applies one layout/lowering contract,
and runs static Linux x86-64 output by default. Native artifacts need neither
Node nor libc. WebAssembly remains a thin optional encoder; the independent K
machine remains the execution oracle. Compiler closure, the fixed point and
independence of the compiler itself from Rust remain unfinished.

## What now runs

Class 0 control and pattern width is installed: equal-error-type question-mark,
if-let, while-let, diverging let-else, nested and rest patterns, binding modes,
or-patterns, guards, struct update and value-producing break. Guarded products
and nonuniform reference choices share their source tree and continuation.
Cached pure tests skip impossible subtrees without enumerating their products.
Effectful guards retry feasible alternatives in source order.

Initialization joins intersect definite ownership and unite possible ownership.
Conditional destruction preserves lexical destruction points. Reference-holder
joins unite possible sources, parents and absence; every later access is checked
against every possibility. Per-leaf call contracts carry exact or bounded input
alternatives, including repeated generic inputs and named lifetimes. Exact
transport preserves bounded incoming provenance. Loop backedges remain equality
checks; reference-bearing writes still need an exact destination identity.
Exact enum tag witnesses exclude inactive reference templates, and joins retain
absence when a holder was moved on one edge and impossible on the other. A live
holder on either feasible edge still conflicts. Tags and saved predicates track their
index dependencies. An already-captured pointer retains its fixed-address proof.
The independent machine rejects the retained forged-K negatives.

Closures now lower to ordinary capture structs and statically selected
Fn/FnMut/FnOnce functions. Shared, mutable, owned, nested, disjoint-field and
unique immutable captures preserve evaluation and destruction order. Contextual
parameter inference and irrefutable parameter patterns work. Callable result
contracts are refined from ordinary checked body provenance, then checked again
by the independent safety gate. No closure-specific K operation or runtime
authority was added. Opaque callable type parameters with explicit reference
results remain unsupported until their hidden captured origins have a contract.
Associated callable bounds such as FnMut(I::Item) prepare associated types
before signature selection, including reversed generic declaration order. Callable
input/output equations now infer a type parameter that appears only in the bound.
The retained generic Map examples run on all routes and Rust, including callable
prerequisites on an iterator implementation and borrowed associated results.
Type equations, child trait proofs and full callable validation are separate
steps; conflicting signatures, missing prerequisites and wrong callable kinds
still reject. Generic and associated result families refine explicit per-leaf
candidate maps from ordinary validated concrete bodies. Acyclic callees refine
before callers; final K rechecks every mapping. Complete definition-site opaque
callable origin/effect contracts remain a separate obligation.

Unicode char is a distinct four-byte checked scalar. Repeated Vec construction,
character/text helpers, Display and integer formatting, String/Formatter
write/writeln, and BTreeSet are source library code. Formatting includes named,
positional and captured arguments, escaped braces, Unicode fill, fixed width and
alignment, signs, prefixes and zero padding. Debug, precision/dynamic width,
floating formatting and arbitrary Write sinks remain unsupported. Shared collection
for loops over borrowed Vec/slice values and iterator values now lower to ordinary
source Iterator/IntoIterator calls, match, while and irrefutable let. The outer match
keeps iterable temporaries alive; items and iterators clean up on every exit. Range
loop binding patterns share that subsystem. Shared map/set iteration and source
enumerate, skip, take, zip and same-type chain adapters now run. Source map/filter/filter_map/inspect adapters and fold/for_each consumers now
compose with contextual Item/callable equations. The source any/all consumers
short-circuit while retaining iterator position; temporary receivers are writable
without changing named binding or shared-reference permissions. General associated Item equality
and owned/exclusive collection iteration remain further work. Option/Result have source
shared/exclusive views and owned combinators, including and/or, transpose,
zip/unzip, flatten and in-place Option replacement/insertion. expect preserves
the value/trap outcome but currently retains unwrap's fixed trap message.

Bounded slice/text views now compose over the inline data/count representation.
All six range forms work on vectors, arrays, slices, str and canonical String;
mutable slice ranges preserve exclusive loans. Inclusive ends are checked before
addition, UTF-8 endpoints are checked before access, and invalid ranges trap
R0003. Every range retains the owner's conservative footprint, including empty
ranges. Numeric disjointness and mutable text are not claimed. Range lowering
uses existing scalar operations; neither encoder nor runtime needs a new body.
Generic declared slice-reference arguments infer their element types from array
or vector references while exact trait matching and bare-T moves remain intact.
Integer wrapping add/sub/mul now have an explicit checked K contract; their
existing machine/lowering implementations are reused. Checked add/sub/mul are
ordinary guarded source functions across all ten integer types.

Imports now retain canonical declaration identity and binding visibility.
Module/enum globs, local use scopes, crate/self/super anchors and anonymous trait
imports work; ambiguities are diagnosed on use, and private aliases cannot widen
visibility. Nominal type lookup rejects enum variants and suffixed value paths
used as types. This is not yet every Rust import form.

The source runtime owns size-class allocation/release, growth, UTF-8 validation
and the iterative cleanup walker. Prepared runtime source is checked normally;
only its sealed raw interface gets primitive bodies. Ordinary raw function names
do not confer authority. Native and Wasm consume the same explicit runtime/glue
calls, natural-width layout and generated functions. Slice/text references now
carry inline data/count metadata: 16 bytes aligned to eight, with sized references
still one word. One scalar-word mapping covers fields, copies, spills and calls;
view returns write a caller-provided slot before the callee frame is released.
Both encoders remain unchanged. UTF-8 validation and byte output consume the pair
directly. K separately exercises the runtime source against a raw-memory model
and keeps its typed application heap.

Native stack/heap extent exhaustion and mapping refusal report R0006; broken
stdout reports R0007 at the source line. The Node reservation worker and Linux
main-stack adjustment are deleted. The remaining Rust compiler thread adapter
serves the bootstrap compiler, not emitted programs. Source type inference now
uses explicit traversal frames after the unchanged small-stack capacity test
exposed recursion through deeply nested nominal types.

Seventeen closure-taking Option/Result combinators now run as ordinary source:
map/map_or/map_or_else/and_then/or_else/unwrap_or_else on both, Option::filter,
is_some_and/is_none_or/ok_or_else, and Result::map_err. Eager Option::ok_or also
converts to Result. Option reference payloads support copied/cloned for both
shared and mutable inputs under their respective Copy/Clone contracts.
Default evaluation, short-circuiting and cleanup agree with
independent expansions. Unique visible structurally compatible method headers can supply conditional
closure argument equations; ordinary selection still proves visibility,
prerequisites, precedence and callable kind. Associated projections retain their
canonical trait identity. Primitive signatures precede competing trait context,
and incomplete census visibility cannot establish a unique method.

Inferred closure results constrain expression shape without creating reborrow
sites. Captured mutable references move through tuples, constructors, branches,
returns and loop breaks; declared annotations retain their coercion behavior.
Callable-produced generic result families include the callable's origins alongside
input alternatives and refine from checked bodies. Final K verifies every mapping.

Fixed callable input regions now have declaration-scoped outlives obligations in
the discarded definition verifier. Requirements follow aliases, tuple/nominal
leaves and callable type arguments through helpers. Retained effects survive
argument/result/callable destruction. Exact checked result selectors and current
stored holders determine permissions. Branches unite effects and loops replay
them to a finite monotone fixed point. Private annotations can only add diagnostics;
parsed K cannot grant itself this authority. Mutable access to stored origins
remains unsupported at opaque calls. General source-region safety is unfinished.

Named local lifetime annotations now constrain every active store, including
late initialization, aliases, fields, replacement and previously absent enum
payloads. Checked storage transfers propagate requirements through earlier and
later contents. Unused lifetime-only and generic definitions receive the same
obligations. Closure construction proves exact captured-region requirements;
body-owned metadata cannot escape discarded type probes. Declared views retain
their own region restrictions across aliases and calls; mutable nested views
preserve invariance. Result checks honor named and elided input/receiver regions.
Ordinary K remains the independent concrete safety gate. Named borrowed slice-element inventories and
external stored-origin mutation remain explicit boundaries.

Vec is_empty/get/first/contains and extend_from_slice are ordinary library source.
Shared views retain their owner; search uses element eq in order; extension clones
in order under the declared Clone contract. Typed let-else bindings now survive
statement temporary cleanup by their actual lexical binding identity.

Destructor-free scalar/reference descriptor trees can be released while descendants
borrow external owners. Actual addressed descriptor storage and custom/owned
destructors retain ordinary protection. Ordinary selected constructors establish
empty-variant evidence; joins discard templates inactive on every feasible edge.
Guarded patterns still use cached pure tests when a guard consumes the scrutinee.

Callback and stored-task programs print through captured Stdout. Data now
excludes both intrinsic World and Stdout through structural containers; ordinary
same-spelled types retain their identity. Retained-borrow errors name the place
and both source locations.

The semantic census recovers independent bodies using one real source graph and
dependency indexes. Known-receiver method absence requires a complete candidate
domain and visibility facts. Missing layouts, signatures, imports and trait
prerequisites remain blockers. Closed structural bodies reach the ordinary K
checker; recovery supplies no executable authority. Orphan nodes in skipped parser
regions cannot establish missing declarations.

## Observations and commands

Final release snapshot: `R=target/collections-verified/lkjscript`, SHA-256
`9057a0dcc5dfeaf96aafba1f9eddb4bee20d7f43f05c147e0dd97d8f20d2eab1`. The source tree and binary were
unchanged during the final census and scale checks. Details remain disposable
under target/. The routine timing below uses existing build/reference caches.

| Observation | Command and seed | Result |
| --- | --- | --- |
| Routine native/K gate | `bash tools/check.sh` | **16.414 s**; **439 Cargo tests**; 607 conformance passes, 4 pending, 514 Rust comparisons per route |
| Gate parts | same command | format 0.442 s; host 0.061 s; release 0.027 s; Rust 15.811 s; lint 0.043 s; overlapping suites 15.861 s |
| Suite parts | same command | conformance 13.192 s; values 3.104 s; verdicts 3.165 s; output 1.593 s; drops 3.523 s; sugars 12.748 s; closures 2.659 s |
| Full all-route gate | `LKJ_GATE_ENGINES=native,k,node bash tools/check.sh` | **37.201 s**, including 14.079 s release rebuild and 21.024 s Rust tests/rebuild; all required checks pass |
| All-route conformance | all-route gate; replay with `LKJ_ENGINES=native,k,node node tools/conformance.mjs "$R"` | **607 passes, 4 pending, 514 Rust comparisons** per route |
| Independent examples, each route | `LKJ_ENGINE=native node tools/corpus.mjs "$R"`; repeat with k/node | idioms **26/30**; ownership **26/30**; rejected **16/16** |
| Values, all routes | all-route gate; `LKJ_ENGINES=native,k,node node tools/generators/value.mjs "$R" 601 100` | 92 equal values; 8 equal traps |
| Verdicts, all routes | all-route gate; `LKJ_ENGINES=native,k,node node tools/generators/verdict.mjs "$R" 601 100` | 73 both reject; 26 equal values; 1 valid E_BORROW rejection |
| Output, all routes | all-route gate; `LKJ_ENGINES=native,k,node node tools/generators/output.mjs "$R" 601 50` | 39 equal normal exits; 11 equal traps |
| Destruction, all routes | commands below: 1101–1104, 400 each; one-sided 1201, 300 | **1,900 equal outputs per route**; zero rejections or invalid cases |
| Sugar/expansion pairs, all routes | all-route gate; `LKJ_ENGINES=native,k,node node tools/generators/sugar.mjs "$R" 1301 216` | **182 equal normal exits; 34 equal traps; 72 families** |
| Closure/environment pairs, all routes | `LKJ_ENGINES=native,k,node node tools/generators/closures.mjs "$R" 1501 176 target/collections-verified/final-closures` | **176 equal outputs per route**; 22 families |
| Routine closures, native/K | gate: 1501, 44 pairs | 44 equal outputs |
| Routine destruction, native/K | gate: ONE_SIDED=1, 1201, 100 programs | 100 equal outputs |
| Outside must-reject programs | included in conformance | all 26 reject, including r01/r02/r13/r17/r18/r19 |

Other generated outcome classes are zero. The gate now runs 216 sugar pairs and
611 conformance cases, an increase of 69 pairs and 118 cases since the preceding
checkpoint.
Its 16.414 s warm time is slightly above the original 16 s reference; the
performance objective has not been met in this observation.

```sh
R=target/collections-verified/lkjscript
LKJ_ENGINES=native,k,node node tools/generators/drops.mjs "$R" 1101 400 target/collections-verified/final-drops
# Repeat for seeds 1102, 1103 and 1104.
LKJ_ENGINES=native,k,node ONE_SIDED=1 node tools/generators/drops.mjs "$R" 1201 300 target/collections-verified/final-drops
```

Rust checks all 216 expansions and 210 sugar forms. Six compact 32-way stress
forms exceed Rust's observed compilation envelope; all lkjscript routes and
expansions execute them, and smaller equivalents remain Rust-checked. Closure
pairs compare independently emitted environments and Rust. Caches include
generator/source/seed and compiler identity. Conformance caches each Rust
rejection source separately, including the host wrapper and flags; adding a
fixture no longer invalidates every unchanged negative reference.

The batch host checks once for requested routes and shares lowering while keeping
execution independent. Optimized suites overlap the full debug Rust tests/lint.
Timeouts require missing progress rather than merely a long batch. Every case
starts with fresh execution state; direct-K traps require exact runtime codes.
The retained named-local, typed-view, elision, capture, temporary-receiver and
selected-result permission cases are required conformance checks. The overlap
permission negative is rejected by the earlier ordinary borrow check; it is not
evidence that the later private grant filter was reached.

### Compiler census

Commands use final R and the same declared source library. The archived original
source bytes were verified against Git:

```sh
mkdir -p target/contracts-next/census-base
git archive 80cd198a src | tar -x -C target/contracts-next/census-base
"$R" check --census target/contracts-next/census-base/src/lib.rs
"$R" check --census src/lib.rs
```

| Unsupported syntax | Original sources | Current sources |
| --- | ---: | ---: |
| Anonymous impl Trait forms | 0 | 8 |
| **Recognized unsupported sites** | **0** | **8** |
| Separate syntax blocked observations | 3 | 19 |

All ten ancestor-restricted visibility sites now parse and resolve. The original
sources still skip two associated Item-equality bounds. Syntax blockers comprise
2 original / 18 current skipped items plus one semantic-phase marker for each.
Zero recognized unsupported forms does not establish complete parsing or compilation.

The second table reaches independent bodies behind incomplete shared
dependencies. Counts are observed diagnostic sites, with blockers kept separate.

| Observed semantic diagnostics | Original sources | Current sources |
| --- | ---: | ---: |
| Receiver-qualified missing methods | 155 | 270 |
| Source-type diagnostics | 104 | 187 |
| Names/imports | 19 | 37 |
| **Total** | **278** | **494** |
| Distinct receiver/method pairs | 54 | 71 |

Source-type findings include inference/coercion limitations in the Rust-valid
compiler; reduce them before assigning library work. The current declared source
graph contains 1,432 bodies: 1,421 attempted, 1,192 returning inference facts,
187 failed, 42 blocked. Only 21 closed structural bodies reached ownership
construction. Returning inference facts is weaker than proving a body.
Overlapping blocked-phase observations are 1,411 layout/ownership, 4,616 method
lookup, 14 resolution and 53 source types; these are not additional feature counts.

| Library family | Original missing-method sites | Current sites | Next work |
| --- | ---: | ---: | --- |
| Vec | 47 | 90 | Extend under checked Item equality; then element mutation algorithms |
| Source iterator types | 40 | 64 | FromIterator/collect, copied, find and further adapters |
| Scalars | 24 | 41 | Source comparisons/classifiers; explicit bit contracts |
| Ordered map/set | 12 | 36 | Entry retention and exclusive iteration contracts |
| String/str | 18 | 22 | UTF-8-aware chars, splitting and trimming |
| Option | 3 | 9 | Default-based extraction and dereference views |
| Result | 5 | 5 | is_ok_and |
| Other receivers | 2 | 3 | Resolve each declaration and its trait requirements |
| Slice/array | 4 | 0 | Original input still exposes to_vec |

Leading current pairs: Vec::extend 53, BTreeMap::entry 30, Map::collect 25 and
Iter::copied 15. Across all receivers, collect has 30 sites and copied 17.
The two original skipped bounds are IntoIterator<Item = bool> and
IntoIterator<Item = (String, usize)>. Rc, Cow, VecDeque, Index, Default,
FromIterator, Range and i128 remain declaration/contract dependencies.
Blocked uses provide no absence claims; an unobserved method is not proved implemented.

### Scaling and native capacity

Quiet final-R command:
`node tools/scale/run.mjs "$R" --families traits,refs,variants,fields
--sizes 2000,4000,8000 --repeat 3 --verb check --engine native
--out target/collections-verified/scale`.
These deterministic families have no random seed. All 36 checks passed.
Median whole-command check milliseconds:

| Family | 2,000 / 4,000 / 8,000 | Ratios per doubling |
| --- | --- | --- |
| Traits | 178.177 / 345.786 / 690.093 | 1.941 / 1.996 |
| Shared references | 70.433 / 118.621 / 225.644 | 1.684 / 1.902 |
| Enum variants | 76.108 / 135.542 / 255.729 | 1.781 / 1.887 |
| Struct fields | 57.095 / 87.920 / 153.055 | 1.540 / 1.741 |

The following capacity observations belong to the 5ee9930d native checkpoint,
not a new maximum run of the expanded front end. They used release snapshot
`8175eefa10a289e01e170333ad5b6bcba5ca1e095759755364132ebbcd57b326`,
saved as `target/owned-snapshot/lkjscript` (`S` below). The native encoder and
source runtime allocation/cleanup algorithms are unchanged; the front end/library,
char/range lowering and view representation have changed.

Command for each deterministic row:
`node tools/scale/owned.mjs S FAMILY SIZE --out target/owned/full`.
Every result assertion and phase marker completed, with no signal or trap.

| Family / size | Compile | Standalone run | Peak run RSS |
| --- | ---: | ---: | ---: |
| `chain 1000000` | 17.835 s | 0.443 s | 1.017 GB |
| `depth 100000000` non-tail calls | 0.010 s | 5.561 s | 15.999 GB |
| `list 50000000` nodes | 0.011 s | 39.876 s | 26.189 GB |
| `push 3000000` and sum | 0.009 s | 0.079 s | 0.068 GB |

The million-function compilation peaked at 6.848 GB RSS and emitted 873,108,149
bytes. List phases were 13.199 s construction, 2.446 s sum and 23.573 s destruction;
whole-process time also includes final memory release. Sizes and available host
memory are observations, not source limits.

Comparable whole-command Fibonacci command, three interleaved samples per route:
`S run /home/coder/lkjscript-claude-20261113/perf/fib.rs --engine native --entry probe`,
then `--engine node`. Medians: native **76.358 ms**, thin Node **32.042 ms**;
ratio **2.383**. Every run returned 2,178,309.

No new maximum-capacity run was made on K or Node. Their earlier million-function
K chain and 35-million-node complete Node list remain historical observations in
limits, not evidence for the changed compiler or the now-thin Node host.

### Trust ledger

Command: `node tools/trust.mjs`. Physical source lines include comments, blanks
and any colocated unit tests; line counts are not weighted proof obligations.

| Responsibility | Lines |
| --- | ---: |
| K contract and safety gate | 7,792 |
| Lowering, layout and lowered contract | 2,838 |
| Source runtime and raw-memory binding | 597 |
| WebAssembly encoder and metadata | 413 |
| Native encoder and Linux entry | 1,193 |
| Production host boundary | 1,393 |
| Independent K and raw-memory models | 2,414 |

The front end is trusted for source meaning; K independently checks concrete
safety. Callable return inference is advisory and the final contract is rechecked.
The source runtime is checked, but its raw-memory invariants and bindings remain
trusted. The models are oracles, not production prerequisites. Rust/Cargo still
build the compiler. Linux runs native artifacts. Node remains in optional Wasm
execution, backend-specific Rust tests and development tools.

## Old paths, limits and next action

Decision 3 steps 1–4 are installed and the requested capacity observations passed
at the recorded native snapshot. The gate uses native/K; Wasm remains thin.
No new maximum run is claimed for this expanded front end. The old hand-built
runtime, Node worker/retries and Linux main-stack adjustment are gone.
Rust compiler thread reservation still supports recursive bootstrap paths.

Named local annotations and shortened typed views now have definition-site
checks, including unused bodies, storage history, closure construction, fixed
calls and named/elided results. Explicit generic type arguments containing named
reference regions still need a type-instantiation contract and reject explicitly.
Opaque callable output/capture families, external stored-origin mutation, named
slice-element inventories and recursive projected view equations remain open.
Concrete specializations still receive the independent K provenance checks.

Other retained reductions cover bounded Option mutation precision, absent-variant
reference choice, Copy-reference Option cleanup, generic iterator stored-origin
forwarding, same-region mutable tuple results, partial closure result annotations,
contextual FnOnce reborrows, array-to-slice method receiver coercion and the
pre-existing direct as_mut_slice promotion gap through a nested range projection.
Four conformance cases remain pending: three concurrency imports and one opaque
impl return. Owned reference-bearing containers, loop inclusion and general
exclusive iteration still need contracts. One valid generated verdict remains
rejected. The reduced named-local and inferred-Stdout gaps are now required tests.

Definition plans cover inference and rigid method evidence. Borrow/copy/coercion
construction, control, cleanup and part of instance construction still use the
existing elaborator. The recovered semantic table is partial. Compiler closure,
declared file capabilities, the three-build hand-over test and removal of
Rust/Node from compiler development remain future work.

**Next action:** check associated-type equalities in trait bounds and the explicit
IntoIterator/Iterator Item relationship using canonical declaration identities
and the existing region obligations. Then implement source Extend, FromIterator,
Copied and Sum over that contract; do not infer equality from coincident concrete
types. The compiler's skipped Item-equality bounds and the observed extend,
collect and copied sites are the immediate probes. Continue explicit generic
region contexts, opaque output/capture contracts, loop inclusion and
reference-bearing collection ownership as separate stated obligations.

`run file.rs --engine native|k|node` selects execution; `batch cases.tsv --engine
native|k|node` selects one route; `batch cases.tsv --engines native,k,node`
shares checking/lowering across routes. `build -o file` emits native ELF;
`build --engine node -o file.wasm` emits Wasm. `k` prints checked K;
`check file.k` reaches the independent safety gate. `run file.k --engine k
--unchecked-k` exposes forged K to the model deliberately.
`LKJSCRIPT_TIMINGS=1` reports the stages that actually ran.
