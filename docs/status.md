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
The retained generic Map example runs on all routes and Rust; conflicting output
equations still reject. Typed callable prerequisites and complete opaque callable
origin/effect contracts remain separate obligations.

Unicode char is a distinct four-byte checked scalar. Repeated Vec construction,
character/text helpers, Display and integer formatting, String/Formatter
write/writeln, and BTreeSet are source library code. Formatting includes named,
positional and captured arguments, escaped braces, Unicode fill, fixed width and
alignment, signs, prefixes and zero padding. Debug, precision/dynamic width,
floating formatting and arbitrary Write sinks remain unsupported. Shared collection
for loops over borrowed Vec/slice values and iterator values now lower to ordinary
source Iterator/IntoIterator calls, match, while and irrefutable let. The outer match
keeps iterable temporaries alive; items and iterators clean up on every exit. Range
loop binding patterns share that subsystem. Range slicing is still open. Option/Result have source
shared/exclusive views and owned combinators, including and/or, transpose,
zip/unzip, flatten and in-place Option replacement/insertion. expect preserves
the value/trap outcome but currently retains unwrap's fixed trap message.

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

## Observations and commands

`B=target/debug/lkjscript`; `R=target/release/lkjscript` after
`cargo build --release`. Details are disposable under `target/`.
Gate timings use existing build artifacts/reference caches. The measured gate is
back below the earlier 16-second time while running 21 more conformance cases and
12 more sugar pairs than the preceding checkpoint. Counts describe observations,
not a general soundness proof. Fresh release evidence below uses the preserved
binary at target/views-snapshot/lkjscript.

| Observation | Command and seed | Result |
| --- | --- | --- |
| Routine native/K gate | `bash tools/check.sh` | **13.569 s**; 420 Cargo tests; 341 conformance passes, 7 pending, 280 Rust comparisons per route |
| Gate parts | same command | format 0.388 s; host 0.059 s; release build 0.011 s; Rust 10.698 s; lint 2.381 s; overlapping tests/suites 13.088 s (conformance 4.152 s) |
| All-route conformance | `LKJ_ENGINES=native,k,node node tools/conformance.mjs R` | 341 passes, 7 pending, 280 Rust comparisons |
| Independent examples, each route | `LKJ_ENGINE=native node tools/corpus.mjs R`; repeat with `k` and `node` | idioms **22/30**; ownership **26/30**; rejected **16/16** |
| Routine values, native/K | gate: seed 601, 100 cases | 92 equal values; 8 equal traps per route |
| Routine verdicts, native/K | gate: seed 601, 100 cases | 73 both reject; 26 equal values; 1 valid E_BORROW rejection per route |
| Routine output, native/K | gate: seed 601, 50 cases | 39 equal normal exits; 11 equal traps per route |
| Destruction, all three routes | command below: seeds 1101–1104, 400 each, and one-sided 1201, 300 | **1,900 equal outputs per route**; no invalid cases or rejections |
| Sugar/expansion pairs, all three routes | `LKJ_ENGINES=native,k,node node tools/generators/sugar.mjs R 1701 96 target/view-next/sugars` | 90 equal normal exits; 6 equal traps per route |
| Routine sugar pairs, native/K | gate: seed 1301, 96 pairs | 87 equal normal exits; 9 equal traps per route |
| Closure/environment pairs, all three routes | `LKJ_ENGINES=native,k,node node tools/generators/closures.mjs R 1501 112 target/view-next/closures` | **112 equal outputs per route**, 14 families; Rust and independent environments agree |
| Routine closure pairs, native/K | gate: seed 1501, 28 pairs | 28 equal outputs per route |
| Outside must-reject programs | included in conformance | all 26 reject with recorded Rust reason codes, including r01/r02/r13/r17/r18/r19 |
| Indexed tag regression | `B check tests/conformance/k/enum_indexed_tag_stale.k`; `B run tests/conformance/k/enum_indexed_tag_stale.k --engine k --unchecked-k` | E_TYPE from the checker; M_INACTIVE_VARIANT at instruction line 12 from the independent machine |

Other generated outcome classes are zero. Destruction commands on release binary
SHA-256 `8b42c6eed22ba89d4bc8e5f0f6d38d3058a271fdb0a60561ee438ff0d7ae4ae2`:

```sh
LKJ_ENGINES=native,k,node node tools/generators/drops.mjs R 1101 400 target/view-next/drops
# Repeat for seeds 1102, 1103 and 1104.
LKJ_ENGINES=native,k,node ONE_SIDED=1 node tools/generators/drops.mjs R 1201 300 target/view-next/drops
```

The 96 sugar pairs cover 32 families. Rust checks all 96 expansions and 90 sugar
forms. Six compact 32-way stress forms exceed Rust's observed compilation
envelope; they still run on all lkjscript routes, and smaller equivalents remain
Rust-checked. This is an oracle limitation, not a source acceptance bound.
The routine gate uses 100 destructor programs at seed 1201. Rust caches are
keyed by generator/source/seed and compiler identity. Every case has fresh
program state.

The batch host checks each case once for all requested routes and shares
lowering between native/Wasm. Execution state stays separate. The gate now uses
the optimized compiler for these suites, with release overflow checks enabled,
while retaining the full debug Rust tests. Rust tests/lint overlap the independent
suites after the build; concurrency follows available host cores, with LKJ_GATE_JOBS
as an operational override. Test timeouts require missing progress, not merely a
long total batch; Node module preparation never substitutes for an execution result.
CI passed at the preceding checkpoint, 95e50f37.

### Compiler census

Current command: `R check --census src/lib.rs`. For a fixed-source comparison:

```sh
mkdir -p target/census-base
git archive 80cd198a src | tar -x -C target/census-base
R check --census target/census-base/src/lib.rs
```

The original source snapshot now has **24** unsupported syntax sites. Current
sources have **54**. This keeps source growth separate from supported width.

| Unsupported syntax | Original sources | Current sources |
| --- | ---: | ---: |
| Range slicing | 24 | 52 |
| `impl` forms | 0 | 2 |
| **Total** | **24** | **54** |

Current blocked regions: 9 (8 items, 1 semantic phase); original sources: 3.
Class 0, char, closures, repeated Vec, collection-for syntax and supported formatting
forms no longer appear as blanket syntax exclusions. Unsupported iterator modes
now fail their semantic contracts instead of a blanket syntax rule. Unsupported
formatting specifications still produce diagnostics.

The second table has **31 semantic dependency sites** in current sources and
**23** on the original snapshot. It remains partial: missing type/import
dependencies and blocked method/trait bodies prevent a complete library plan.
The following table combines import and use sites for each dependency; the
command prints the individual rows and locations.

| Semantic dependency | Original sources | Current sources |
| --- | ---: | ---: |
| Rc | 8 | 13 |
| i128 | 6 | 6 |
| VecDeque | 3 | 3 |
| FromIterator | 2 | 2 |
| Index | 2 | 2 |
| Item | 2 | 2 |
| Cow | 0 | 1 |
| Default | 0 | 1 |
| Range | 0 | 1 |
| **Total** | **23** | **31** |

### Scaling and native capacity

Fresh release checks use SHA-256
`8b42c6eed22ba89d4bc8e5f0f6d38d3058a271fdb0a60561ee438ff0d7ae4ae2`:

`node tools/scale/run.mjs R --families traits,refs,variants,fields
--sizes 2000,4000,8000 --repeat 3 --out target/scale/views-checkpoint`.

Median check milliseconds and ratios per doubling:

| Family | 2,000 / 4,000 / 8,000 | Ratios |
| --- | --- | --- |
| Traits | 164.201 / 316.241 / 642.654 | 1.926 / 2.032 |
| Shared references | 55.085 / 107.304 / 197.407 | 1.948 / 1.840 |
| Enum variants | 62.503 / 114.358 / 225.515 | 1.830 / 1.972 |
| Struct fields | 41.709 / 69.880 / 130.476 | 1.675 / 1.867 |

The following capacity observations belong to the preceding native checkpoint,
not a new maximum run of the expanded front end. They used release snapshot
`8175eefa10a289e01e170333ad5b6bcba5ca1e095759755364132ebbcd57b326`,
saved as `target/owned-snapshot/lkjscript` (`S` below). The native encoder and
source runtime allocation/cleanup algorithms are unchanged; the front end/library,
char lowering and full-view representation have changed.

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
| K contract and safety gate | 5,716 |
| Layout, lowering and lowered contract | 2,725 |
| Source runtime and sealed raw binding | 597 |
| WebAssembly encoder and metadata | 413 |
| Native encoder and Linux entry | 1,193 |
| Production host boundary | 1,383 |
| Independent K and raw-memory models | 2,290 |

The front end is trusted for source meaning; K independently checks concrete
safety. Callable return inference is advisory and the final contract is rechecked.
The source runtime is checked, but its raw-memory invariants and bindings remain
trusted. The models are oracles, not production prerequisites. Rust/Cargo still
build the compiler. Linux runs native artifacts. Node remains in optional Wasm
execution, backend-specific Rust tests and development tools.

## Old paths, limits and next action

Decision 3 steps 1–4 are installed and the requested native observations pass at
the recorded snapshot. The routine conformance/generator gate uses native and K.
Wasm remains a thin encoder. The old hand-built runtime bodies, Node
worker/retries and Linux main-stack adjustment are gone. Rust compiler stack
reservation remains for recursive bootstrap compiler paths.

Shared iteration is installed. Owned/exclusive collection iteration still needs
initialized-range and disjoint-range contracts. A concrete Iter can yield two
held shared items, but the opaque generic next helper conservatively ties Item
to its receiver; precise associated-result transport is still required. Typed
callable prerequisites and the broader adapter library remain unfinished.

Inline full-view representation is installed. Range slicing now needs its
checked bounded-view operation, UTF-8 boundary behavior and source lowering. Loop
inclusion contracts, reference-bearing containers and full generic witness closure
remain open. One routine valid verdict is still rejected. Seven conformance cases
remain pending for concurrency imports, opaque impl return syntax and parameterized
trait bounds.

A retained reduced failure, tests/known-gaps/opaque_callable_result_borrow.rs,
is accepted as an unused generic definition by `R check`; Rust rejects it with
E0506 (`rustc --edition=2024 --crate-type=lib --emit=metadata -A warnings FILE`).
The missing definition-site callable contract includes opaque output/capture
origin families and named-lifetime call effects. This is not fixed by treating
an opaque result as static or by choosing one input. Concrete specializations
still receive complete K provenance and external-mutation checks; the unused
definition does not establish generic lifetime safety.

Definition plans cover inference and rigid method evidence. Borrow/copy/coercion
construction, control, cleanup and part of instance construction still use the
existing elaborator; concrete K is checked independently. The semantic census
still needs missing library types and body recovery before it is a complete
library plan. Rust Rc, source packing, host/file adapters and test tools remain
bootstrap responsibilities.

**Next action:** implement bounded slice/text views over the shared inline ABI;
finish typed callable prerequisites and per-leaf associated result transport;
close the retained definition-site origin/effect gap before claiming general
opaque callable safety. Continue ordinary source adapters and recover independent
semantic census bodies as those contracts permit.

`run file.rs --engine native|k|node` selects execution; `batch cases.tsv --engine
native|k|node` selects one route; `batch cases.tsv --engines native,k,node`
shares checking/lowering across routes. `build -o file` emits native ELF;
`build --engine node -o file.wasm` emits Wasm. `k` prints checked K;
`check file.k` reaches the independent safety gate. `run file.k --engine k
--unchecked-k` exposes forged K to the model deliberately.
`LKJSCRIPT_TIMINGS=1` reports the stages that actually ran.
