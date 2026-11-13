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
Associated callable bounds such as FnMut(I::Item) now prepare associated types
before signature selection, including reversed generic declaration order. Shared
source iterator adapters using these bounds run on all routes.

Unicode char is a distinct four-byte checked scalar. Repeated Vec construction,
character/text helpers, Display and integer formatting, String/Formatter
write/writeln, and BTreeSet are source library code. Formatting includes named,
positional and captured arguments, escaped braces, Unicode fill, fixed width and
alignment, signs, prefixes and zero padding. Debug, precision/dynamic width,
floating formatting and arbitrary Write sinks remain unsupported. Collection
iteration and range slicing are still open. Option/Result now have source
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
calls, natural-width layout and generated functions. K separately exercises the
runtime source against a raw-memory model and keeps its typed application heap.

Native stack/heap extent exhaustion and mapping refusal report R0006; broken
stdout reports R0007 at the source line. The Node reservation worker and Linux
main-stack adjustment are deleted. The remaining Rust compiler thread adapter
serves the bootstrap compiler, not emitted programs. Source type inference now
uses explicit traversal frames after the unchanged small-stack capacity test
exposed recursion through deeply nested nominal types.

## Observations and commands

`B=target/debug/lkjscript`; `R=target/release/lkjscript` after
`cargo build --release`. Details are disposable under `target/`.
Gate timings use existing build artifacts/reference caches. Counts describe
observations, not a general soundness proof.

| Observation | Command and seed | Result |
| --- | --- | --- |
| Routine native/K gate | `bash tools/check.sh` | **23.519 s**; 419 Cargo tests; 320 conformance passes, 7 pending, 259 Rust comparisons per route |
| Gate parts | same command | format 0.378 s; host 0.057 s; Rust 9.928 s; lint 0.039 s; build 0.024 s; parallel suites 13.069 s |
| Optional Node conformance | `LKJ_ENGINE=node node tools/conformance.mjs R` | 320 passes, 7 pending, 259 Rust comparisons |
| Independent examples, each route | `LKJ_ENGINE=native node tools/corpus.mjs R`; repeat with `k` and `node` | idioms **20/30**; ownership **26/30**; rejected **16/16** |
| Routine values, native/K | gate: seed 601, 100 cases | 92 equal values; 8 equal traps per route |
| Routine verdicts, native/K | gate: seed 601, 100 cases | 73 both reject; 26 equal values; 1 valid E_BORROW rejection per route |
| Routine output, native/K | gate: seed 601, 50 cases | 39 equal normal exits; 11 equal traps per route |
| Destruction, all three routes | command below: seeds 1101–1104, 400 each, and one-sided 1201, 300 | **1,900 equal outputs per route**; no invalid cases or rejections |
| Sugar/expansion pairs, all three routes | `LKJ_ENGINES=native,k,node node tools/generators/sugar.mjs R 1701 84 target/shared-sugars` | 73 equal normal exits; 11 equal traps per route |
| Routine sugar pairs, native/K | gate: seed 1301, 84 pairs | 73 equal normal exits; 11 equal traps per route |
| Closure/environment pairs, all three routes | `LKJ_ENGINES=native,k,node node tools/generators/closures.mjs R 1501 112 target/shared-closures` | **112 equal outputs per route**, 14 families; Rust and independent environments agree |
| Routine closure pairs, native/K | gate: seed 1501, 28 pairs | 28 equal outputs per route |
| Outside must-reject programs | included in conformance | all 26 reject with recorded Rust reason codes, including r01/r02/r13/r17/r18/r19 |
| Indexed tag regression | `B check tests/conformance/k/enum_indexed_tag_stale.k`; `B run tests/conformance/k/enum_indexed_tag_stale.k --engine k --unchecked-k` | E_TYPE from the checker; M_INACTIVE_VARIANT at instruction line 12 from the independent machine |

Other generated outcome classes are zero. Destruction commands on release binary
SHA-256 `bff5443acaa162f17f87d33cdbaa7103da7b0c131254047cbd32c1ba3d68f326`:

```sh
LKJ_ENGINES=native,k,node node tools/generators/drops.mjs R 1101 400 target/shared-drops
# Repeat for seeds 1102, 1103 and 1104.
LKJ_ENGINES=native,k,node ONE_SIDED=1 node tools/generators/drops.mjs R 1201 300 target/shared-drops
```

The 84 sugar pairs cover 28 families. Rust checks all 84 expansions and 78 sugar
forms. Six compact 32-way stress forms exceed Rust's observed compilation
envelope; they still run on all lkjscript routes, and smaller equivalents remain
Rust-checked. This is an oracle limitation, not a source acceptance bound.
The routine gate uses 100 destructor programs at seed 1201. Rust caches are
keyed by generator/source/seed and compiler identity. Every case has fresh
program state.

The warm gate fell from 32.206 s to 23.519 s while adding 22 conformance cases
and nine sugar pairs. The earlier approximately 16-second objective remains open.
The batch host now checks each case once for all requested routes and shares
lowering between native/Wasm. Execution state stays separate. Method lookup
indexes method name and receiver head, retaining conservative generic/structural
candidates and the complete existing match/bound/visibility checks.

A controlled 100-check comparison of the same 36,116-byte library and trivial
probe measured 5.563 s before indexing and 3.940 s afterward; the type phase fell
from 4.205 s to 2.668 s. Commands: `target/gate-profile/baseline/phases current 100`
and `target/gate-profile/phases-indexed frozen 100`. Both produce the same
187 source functions, 43 K bodies and 1,356 instructions. Driver SHA-256 prefixes
are `06735646521a99a7` and `d815c9b501179d14`; the frozen library is from `1bb742a7`.
These are scoped profiling observations, not a whole-gate speed ratio.

CI at `1bb742a7` passed Rust tests but exceeded whole-batch test timeouts while
cases were still making progress. Batches now time out on missing progress;
module preparation in Node-only runs does not count as an execution result.
Missing records and host failures still fail the suite. Suite concurrency follows
available host cores, with `LKJ_GATE_JOBS` as an operational override.

### Compiler census

Current command: `B check --census src/lib.rs`. For a fixed-source comparison:

```sh
mkdir -p target/census-base
git archive 80cd198a src | tar -x -C target/census-base
B check --census target/census-base/src/lib.rs
```

The original source snapshot now has **627** unsupported syntax sites. Current
sources have **898**. This keeps source growth separate from supported width.

| Unsupported syntax | Original sources | Current sources |
| --- | ---: | ---: |
| Collection iteration | 475 | 664 |
| `for` binding patterns | 128 | 182 |
| Range slicing | 24 | 50 |
| `impl` forms | 0 | 2 |
| **Total** | **627** | **898** |

Current blocked regions: 7 (6 items, 1 semantic phase); original sources: 3.
Class 0, char, closures, repeated Vec and supported formatting forms no longer
appear as blanket syntax exclusions. Unsupported formatting specifications still
produce diagnostics.

The second table has **32 semantic dependency sites** in current sources and
**24** on the original snapshot. It remains partial: missing type/import
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
| Iterator | 1 | 1 |
| Cow | 0 | 1 |
| Default | 0 | 1 |
| Range | 0 | 1 |
| **Total** | **24** | **32** |

### Scaling and native capacity

Fresh release checks use SHA-256
`bff5443acaa162f17f87d33cdbaa7103da7b0c131254047cbd32c1ba3d68f326`:

`node tools/scale/run.mjs R --families traits,refs,variants,fields
--sizes 2000,4000,8000 --repeat 3 --out target/scale/shared-checkpoint`.

Median check milliseconds and ratios per doubling:

| Family | 2,000 / 4,000 / 8,000 | Ratios |
| --- | --- | --- |
| Traits | 151.438 / 315.240 / 654.678 | 2.082 / 2.077 |
| Shared references | 55.417 / 102.265 / 200.670 | 1.845 / 1.962 |
| Enum variants | 62.857 / 117.579 / 233.040 | 1.871 / 1.982 |
| Struct fields | 41.636 / 71.874 / 133.927 | 1.726 / 1.863 |

The following capacity observations belong to the preceding native checkpoint,
not a new maximum run of the expanded front end. They used release snapshot
`8175eefa10a289e01e170333ad5b6bcba5ca1e095759755364132ebbcd57b326`,
saved as `target/owned-snapshot/lkjscript` (`S` below). The native encoder and
source runtime algorithms are unchanged; the front end/library and char lowering
have changed.

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
| Layout, lowering and lowered contract | 2,502 |
| Source runtime and sealed raw binding | 585 |
| WebAssembly encoder and metadata | 413 |
| Native encoder and Linux entry | 1,193 |
| Production host boundary | 1,365 |
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

Collection iteration is next behind ordinary source Iterator/IntoIterator
contracts. Shared iteration and associated callable adapter prototypes run on all
routes. For lowering must preserve temporary iterable lifetime, irrefutable Item
binding and cleanup on every exit. Owned and exclusive iteration need
initialized-range and disjoint-range contracts. Generic callable output types
still need inference from their callable bounds, and borrowed results need
symbolic captured origins when both the callable and its argument carry views.
The existing single-input fallback is not that general contract; ordinary
callables must not silently become lending closures.

Range slicing needs an explicit multiword view/lowering contract. Inline
data/length metadata is preferred to allocated reference descriptors, whose
cleanup would otherwise infect ordinary reference operations. Migrate existing
full views through shared scalar-word mapping and a caller-provided result slot
before adding ranges. Loop inclusion contracts,
reference-bearing containers and full generic witness closure remain open.
One routine valid verdict is still rejected. Seven conformance cases remain
pending for concurrency imports, opaque impl return syntax and parameterized
trait bounds.

Definition plans cover inference and rigid method evidence. Borrow/copy/coercion
construction, control, cleanup and part of instance construction still use the
existing elaborator; concrete K is checked independently. The semantic census
still needs missing library types and body recovery before it is a complete
library plan. Rust Rc, source packing, host/file adapters and test tools remain
bootstrap responsibilities.

**Next action:** implement ordinary shared collection iteration and structural
for lowering; migrate existing slice/text views to inline data/length metadata
through one shared lowering ABI; finish callable output inference and captured
origin contracts for source adapters. Continue measured gate work without
reducing coverage or introducing a total-batch ceiling.

`run file.rs --engine native|k|node` selects execution; `batch cases.tsv --engine
native|k|node` selects one route; `batch cases.tsv --engines native,k,node`
shares checking/lowering across routes. `build -o file` emits native ELF;
`build --engine node -o file.wasm` emits Wasm. `k` prints checked K;
`check file.k` reaches the independent safety gate. `run file.k --engine k
--unchecked-k` exposes forged K to the model deliberately.
`LKJSCRIPT_TIMINGS=1` reports the stages that actually ran.
