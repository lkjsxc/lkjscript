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
holder on either feasible edge still conflicts. A new direct-K probe exposed a
stale tag after changing an indexed witness; tags and saved predicates now track
that index dependency. An already-captured pointer retains its fixed-address
proof. The independent machine rejects all eight new forged-K negatives.

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
enumerate, skip, take, zip and same-type chain adapters now run. General iterator
Item equality, owned/exclusive collection iteration and closure-taking adapters
remain further work. Option/Result have source
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

Fourteen closure-taking Option/Result combinators now run as ordinary source:
map/map_or/map_or_else/and_then/or_else/unwrap_or_else on both, Option::filter,
and Result::map_err. Default evaluation, short-circuiting and cleanup agree with
independent expansions. Unique visible unconditional inherent headers can supply
closure argument equations; ordinary selection still proves visibility,
prerequisites, precedence and callable kind.

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

Destructor-free scalar/reference descriptor trees can be released while descendants
borrow external owners. Actual addressed descriptor storage and custom/owned
destructors retain ordinary protection. Ordinary selected constructors establish
empty-variant evidence; joins discard templates inactive on every feasible edge.
Guarded patterns still use cached pure tests when a guard consumes the scrutinee.

Modern callback and stored-task programs print through captured Stdout. Data now
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

`B=target/debug/lkjscript`. Final candidate
`R=target/contracts-final/lkjscript`, SHA-256
`0040ff5280fb14bb6499792c5f843e9bb6585f564d1e53f7674961c573426c8d`.
Details are disposable under target/. Timings use existing build/reference caches.

The expanded execution/scaling freeze is `G=target/contracts-snapshot/lkjscript`,
SHA-256 `5a236cc112dcdf14cc6a448f501cd9aa4812a1b88ed107f3de9499779aff1912`.
The final delta filters private census artifacts, excludes Stdout from Data,
adds places to retained-effect errors, and activates three stale capability
fixtures. Final gate/conformance use R's state; larger generator/scale observations
below explicitly use G.

| Observation | Command and seed | Result |
| --- | --- | --- |
| Routine native/K gate | `bash tools/check.sh` | **15.328 s**; 435 Cargo tests; 489 conformance passes, 4 pending, 397 Rust comparisons per route |
| Gate parts | same command | format 0.412 s; host 0.062 s; release 0.026 s; Rust 14.757 s; lint 0.041 s; overlapping suites 14.808 s |
| Suite parts | same command | conformance 8.753 s; values 2.989 s; verdicts 2.457 s; output 1.357 s; drops 2.901 s; sugars 6.953 s; closures 2.701 s |
| All-route conformance | `LKJ_ENGINES=native,k,node node tools/conformance.mjs R` | **489 passes, 4 pending, 397 Rust comparisons** per route |
| Independent examples, each route | `LKJ_ENGINE=native node tools/corpus.mjs G`; repeat with k/node | idioms **26/30**; ownership **26/30**; rejected **16/16** |
| Values, all routes | `LKJ_ENGINES=native,k,node node tools/generators/value.mjs G 601 100` | 92 equal values; 8 equal traps |
| Verdicts, all routes | `LKJ_ENGINES=native,k,node node tools/generators/verdict.mjs G 601 100` | 73 both reject; 26 equal values; 1 valid E_BORROW rejection |
| Output, all routes | `LKJ_ENGINES=native,k,node node tools/generators/output.mjs G 601 50` | 39 equal normal exits; 11 equal traps |
| Destruction, all routes | commands below: 1101–1104, 400 each; one-sided 1201, 300 | **1,900 equal outputs per route**; zero rejections or invalid cases |
| Sugar/expansion pairs, all routes | `LKJ_ENGINES=native,k,node node tools/generators/sugar.mjs G 1901 147 target/contracts-next/sugars` | 122 equal normal exits; 25 equal traps; 49 families |
| Closure/environment pairs, all routes | `LKJ_ENGINES=native,k,node node tools/generators/closures.mjs G 1501 176 target/contracts-next/closures` | **176 equal outputs per route**; 22 families |
| Routine sugars, native/K | gate: 1301, 147 pairs | 117 equal normal exits; 30 equal traps |
| Routine closures, native/K | gate: 1501, 44 pairs | 44 equal outputs |
| Routine destruction, native/K | gate: ONE_SIDED=1, 1201, 100 programs | 100 equal outputs |
| Outside must-reject programs | included in conformance | all 26 reject, including r01/r02/r13/r17/r18/r19 |

Other generated outcome classes are zero. The original b16 Option::map idiom now runs.

```sh
LKJ_ENGINES=native,k,node node tools/generators/drops.mjs G 1101 400 target/contracts-next/drops
# Repeat for seeds 1102, 1103 and 1104.
LKJ_ENGINES=native,k,node ONE_SIDED=1 node tools/generators/drops.mjs G 1201 300 target/contracts-next/drops
```

Rust checks all 147 expansions and 141 sugar forms. Six compact 32-way stress forms
exceed Rust's observed compilation envelope; all lkjscript routes and expansions
execute them, and smaller equivalents remain Rust-checked. Closure pairs compare
independently emitted environments and Rust. Caches include generator/source/seed
and compiler identity. Every case starts with fresh execution state.

The batch host checks once for requested routes and shares lowering while keeping
execution independent. Optimized suites overlap the full debug Rust tests/lint.
Timeouts require missing progress rather than merely a long batch. Direct-K trap
fixtures must report exact runtime codes. CI was green at the preceding d8d83251
checkpoint; each publication receives its own CI inspection.

### Compiler census

Commands use final R and the same declared source library:

```sh
mkdir -p target/contracts-next/census-base
git archive 80cd198a src | tar -x -C target/contracts-next/census-base
R check --census target/contracts-next/census-base/src/lib.rs
R check --census src/lib.rs
```

| Unsupported syntax | Original sources | Current sources |
| --- | ---: | ---: |
| Anonymous impl Trait forms | 0 | 7 |
| **Recognized unsupported sites** | **0** | **7** |
| Separate syntax blocked regions | 3 | 16 |

The original sources still have two parse errors in associated Item equalities.
Current sources also have parameter-pattern parse failures. Zero recognized
unsupported forms does not establish complete parsing or compilation.

The second table reaches independent bodies previously blocked behind shared
dependencies. Counts are observed diagnostic sites; incomplete regions remain
separate.

| Observed semantic diagnostics | Original sources | Current sources |
| --- | ---: | ---: |
| Receiver-qualified missing methods | 281 | 390 |
| Source-type diagnostics | 92 | 148 |
| Names/imports | 19 | 32 |
| **Total** | **392** | **570** |
| Distinct receiver/method pairs | 57 | 76 |

Source-type findings include inference/coercion limitations while reading the
Rust-valid compiler; they need reductions before assignment to library work.
Current coverage: 1,343 bodies present, 1,334 attempted, 1,150 returned inference
facts, 148 failed, 36 blocked. Only 21 closed structural bodies reached ownership
construction. Returning inference facts is weaker than proving a body. Overlapping
blocked phase observations are 1,322 layout/ownership, 4,822 method lookup,
13 resolution and 45 source types. The CLI retains reasons and locations.

| Library family | Original missing-method sites | Current sites | Next work |
| --- | ---: | ---: | --- |
| Vec | 190 | 232 | is_empty/get/first/contains, then extension and element algorithms |
| Option | 23 | 43 | ok_or_else/is_some_and/is_none_or; exact lazy/drop behavior |
| Ordered map/set | 13 | 32 | Entry retention; disjoint origins for exclusive iteration |
| Iterator adapters | 7 | 12 | Source I/F structs; checked Item/callable equations |
| String/str | 18 | 27 | UTF-8-aware chars, splitting and trimming |
| Scalar helpers | 24 | 44 | Source comparisons/classifiers; explicit bit contracts |
| Slice | 6 | 0 | Original input retains first/to_vec gaps |

Leading current sites: Vec::is_empty 68, get 45, extend 39, extend_from_slice 23,
contains 15, first 8; BTreeMap::entry 23; Option::ok_or_else 22, is_some_and 13,
is_none_or 4. Four Vec access/search helpers cover 136 observed sites.
Rc, Cow, VecDeque, Index, Default, FromIterator, Range and i128 remain separate
declaration/contract dependencies. Blocked uses provide no absence claims.

### Scaling and native capacity

On G: `node tools/scale/run.mjs G --families traits,refs,variants,fields
--sizes 2000,4000,8000 --repeat 3 --out target/scale/contracts-checkpoint`.
These deterministic families have no random seed. Median check milliseconds:

| Family | 2,000 / 4,000 / 8,000 | Ratios per doubling |
| --- | --- | --- |
| Traits | 165.745 / 333.746 / 680.559 | 2.014 / 2.039 |
| Shared references | 66.309 / 113.839 / 213.743 | 1.717 / 1.878 |
| Enum variants | 72.105 / 126.702 / 251.237 | 1.757 / 1.983 |
| Struct fields | 50.784 / 81.462 / 144.651 | 1.604 / 1.776 |

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
| K contract and safety gate | 6,522 |
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

Named lifetime annotations on locals still have a definition-site gap: the
retained named_local_region_duration reduction is accepted unused while Rust
rejects it. An annotation on borrowed descriptor storage can constrain earlier
and later assignments, so a forward-only check is insufficient. General opaque
output/capture families and mutation of stored reference origins also remain
open. Concrete specializations receive complete K provenance checks.

Other retained reductions cover bounded Option mutation precision, generic
iterator stored-origin forwarding, same-region mutable tuple results, partial
closure return annotations, contextual FnOnce reborrows and an unannotated
Stdout capture. Four conformance cases remain pending: three concurrency imports
and one opaque impl return. Owned reference-bearing containers, loop inclusion
and general exclusive iteration still need contracts. One valid generated
verdict remains rejected.

Definition plans cover inference and rigid method evidence. Borrow/copy/coercion
construction, control, cleanup and part of instance construction still use the
existing elaborator. The recovered semantic table is partial. Compiler closure,
declared file capabilities, the three-build hand-over test and removal of
Rust/Node from compiler development remain future work.

**Next action:** enforce named regions on local annotations through real binding
and instruction identities, all stores (including replace and indirect storage
history), and checked closure construction. Keep metadata private and
transactional. In parallel, add ordinary Vec access/search helpers and
closure-taking iterator adapters; solve associated Item/context equations
without bypassing trait or origin proofs. Then address loop inclusion and
reference-bearing collection ownership.

`run file.rs --engine native|k|node` selects execution; `batch cases.tsv --engine
native|k|node` selects one route; `batch cases.tsv --engines native,k,node`
shares checking/lowering across routes. `build -o file` emits native ELF;
`build --engine node -o file.wasm` emits Wasm. `k` prints checked K;
`check file.k` reaches the independent safety gate. `run file.k --engine k
--unchecked-k` exposes forged K to the model deliberately.
`LKJSCRIPT_TIMINGS=1` reports the stages that actually ran.
