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
Effectful guards retry feasible alternatives in source order. No new ownership
rule or K operation was needed for the final pattern work.

Initialization joins intersect definite ownership and unite possible ownership.
Conditional destruction preserves lexical destruction points. Reference-holder
joins unite possible sources, parents and absence; every later access is checked
against every possibility. Per-leaf call contracts carry exact or bounded input
alternatives, including repeated generic inputs and named lifetimes. Exact
transport preserves bounded incoming provenance. Loop backedges remain equality
checks; reference-bearing writes still need an exact destination identity.

The source runtime owns size-class allocation/release, growth, UTF-8 validation
and the iterative cleanup walker. Prepared runtime source is checked normally;
only its sealed raw interface gets primitive bodies. Ordinary raw function names
do not confer authority. Native and Wasm consume the same explicit runtime/glue
calls, natural-width layout and generated functions. K separately exercises the
runtime source against a raw-memory model and keeps its typed application heap.

Native conformance, the generators and the requested capacity probes pass.
Native stack/heap extent exhaustion and mapping refusal report R0006; broken
stdout reports R0007 at the source line. The Node reservation worker and Linux
main-stack adjustment are deleted. The remaining Rust compiler thread adapter
serves the bootstrap compiler, not emitted programs.

## Observations and commands

`B=target/debug/lkjscript`; `R=target/release/lkjscript` after
`cargo build --release`. Details are disposable under `target/`.
Gate timings use existing build artifacts/reference caches; new conformance
references were populated during validation. Counts describe observations, not
a general soundness proof.

| Observation | Command and seed | Result |
| --- | --- | --- |
| Routine native/K gate | `bash tools/check.sh` | 14.360 s; 418 Cargo tests; 236 conformance passes, 8 pending, 194 Rust comparisons per route |
| Gate parts | same command | format 0.332 s; host 0.064 s; Rust 7.223 s; lint 1.436 s; build 0.037 s; parallel suites 5.260 s |
| Three-route integration | `LKJ_GATE_ENGINES=node,k,native bash tools/check.sh` | 234 passes, 8 pending and 192 Rust comparisons per route before the two final tuple-rest rejection fixtures |
| Independent examples, each route | `LKJ_ENGINE=native node tools/corpus.mjs B`; repeat with `k` and `node` | idioms 17/30; ownership 26/30; rejected 16/16 |
| Values, each route | `LKJ_ENGINE=native node tools/generators/value.mjs B 601 100`; repeat with `k,node` separately | 92 equal values; 8 equal traps |
| Verdicts, each route | `LKJ_ENGINE=native node tools/generators/verdict.mjs B 601 100`; repeat with `k,node` separately | 73 both reject; 26 equal values; 1 valid E_BORROW rejection |
| Output, each route | `LKJ_ENGINES=node,k,native node tools/generators/output.mjs B 601 50` | 39 equal normal exits; 11 equal traps |
| Conditional destruction, each route | `LKJ_ENGINES=node,k,native ONE_SIDED=1 node tools/generators/drops.mjs B 1201 300 target/checkpoint-generators` | 300 equal outputs; no invalid generator cases or rejections |
| Sugar/expansion pairs | `LKJ_ENGINES=node,k,native node tools/generators/sugar.mjs B 1501 60 target/width` | 55 equal normal exits; 5 equal traps on every route |
| Routine sugar pairs | gate uses seed 1301, 60 pairs | 52 equal normal exits; 8 equal traps |
| Outside must-reject programs | included in conformance | all 26 reject with recorded Rust reason codes, including r01/r02/r13/r17/r18/r19 |

Other generated outcome classes are zero. The 60 sugar pairs cover 20 families.
Rust checks all 60 expansions and 54 sugar forms. The six compact 32-way stress
forms exceed Rust's observed compilation envelope; they still run on all
lkjscript routes, and smaller equivalents remain Rust-checked. This is an oracle
limitation, not a source acceptance bound. The routine gate uses 100 destructor
programs at seed 1201. Rust caches are keyed by generator/source/seed and compiler
identity. Every case has fresh program state.

### Compiler census

Current command: `B check --census src/lib.rs`. For a fixed-source comparison:

```sh
mkdir -p target/census-base
git archive 80cd198a src | tar -x -C target/census-base
B check --census target/census-base/src/lib.rs
```

The original source snapshot now has **1,300** unsupported syntax sites. Current
sources include the new native encoder, runtime and pattern implementation, and
have **1,672**. This comparison keeps source growth separate from supported width.

| Unsupported syntax | Original sources | Current sources |
| --- | ---: | ---: |
| Collection iteration | 475 | 616 |
| Closures | 375 | 487 |
| Closure parameter patterns | 26 | 52 |
| `for` binding patterns | 128 | 166 |
| `format!` | 160 | 184 |
| Character literals | 85 | 90 |
| Range slicing | 24 | 42 |
| Repeated Vec literals | 27 | 33 |
| `impl` forms | 0 | 2 |
| **Total** | **1,300** | **1,672** |

Current blocked regions: 59 (52 statements, 6 items, 1 semantic phase).
Original sources: 29 blocked regions. Class 0 forms no longer appear in either
syntax table.

The second census table has **565 semantic dependency sites** in current sources
(413 on the original snapshot). It remains partial: missing imports explain some
dependent names, and blocked method/trait bodies have not been resolved.
The command prints every row and location; the leading rows are below.

| Semantic dependency | Sites |
| --- | ---: |
| `Op::Return` | 40 |
| `Type` | 39 |
| `Op::Int` | 29 |
| `BTreeSet` | 27 |
| `Op::Drop` | 25 |
| `Place` | 24 |
| `Field` | 18 |
| `Op::Borrow` | 18 |
| `Op::Read` | 17 |
| `TypeKind::Ref` | 14 |
| Import `std::collections::BTreeSet` | 14 |
| `std::result::Result` | 12 |
| Other reported dependencies | 288 |
| **Total** | **565** |

### Scaling and native capacity

These release measurements used snapshot `8175eefa10a289e0`, saved as
`target/owned-snapshot/lkjscript` (`S` below). The native encoder, lowering and
runtime are unchanged since that snapshot; tuple-rest inference and bootstrap
host cleanup were finalized afterward.

Command: `node tools/scale/run.mjs S --families traits,refs,variants,fields
--sizes 2000,4000,8000 --repeat 3 --out target/scale/owned-checkpoint`.
Median check milliseconds and ratios per doubling:

| Family | 2,000 / 4,000 / 8,000 | Ratios |
| --- | --- | --- |
| Traits | 132.515 / 273.281 / 582.201 | 2.062 / 2.130 |
| Shared references | 42.654 / 84.970 / 172.789 | 1.992 / 2.034 |
| Enum variants | 47.514 / 95.484 / 194.620 | 2.010 / 2.038 |
| Struct fields | 29.208 / 53.806 / 107.594 | 1.842 / 2.000 |

Command for each deterministic row:
`node tools/scale/owned.mjs S FAMILY SIZE --out target/owned/full`.
These are the largest new native runs; every result assertion and phase marker
completed, with no signal or trap.

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

Command: `node tools/trust.mjs`. Physical lines include comments and blanks.

| Responsibility | Lines |
| --- | ---: |
| K contract and safety gate | 5,477 |
| Layout, lowering and lowered contract | 2,489 |
| Source runtime and sealed raw binding | 585 |
| WebAssembly encoder and metadata | 413 |
| Native encoder and Linux entry | 1,193 |
| Production host boundary | 1,324 |
| Independent K and raw-memory models | 2,252 |

The front end is trusted for source meaning; K independently checks concrete
safety. The source runtime is checked, but its raw-memory invariants and bindings
remain trusted. The models are oracles, not production prerequisites. Rust/Cargo
still build the compiler. Linux runs native artifacts. Node remains in optional
Wasm execution, existing backend-specific Rust tests and development tools.

## Old paths, limits and next action

Decision 3 steps 1–4 are installed and the requested native observations pass.
The routine conformance/generator gate uses native and K. Wasm is a 348-line
encoder; its backend-specific Rust harnesses still use Node. The old hand-built
runtime bodies, Node worker/retries and Linux main-stack adjustment are gone.
Rust compiler stack reservation remains necessary for recursive bootstrap
compiler paths until compiler closure replaces that host responsibility.

Closures and collection iteration are next, behind the installed multi-root and
origin contracts; class 1 character/text/slicing/repeated-Vec/formatting width
proceeds in parallel. Ordinary callables must not silently become lending
closures. Loop inclusion contracts, reference-bearing containers and full generic
witness closure remain open. One routine valid verdict is still rejected.
Eight conformance cases remain pending for closures, concurrency imports and
parameterized trait bounds.

Definition plans cover inference and rigid method evidence. Borrow/copy/coercion
construction, control, cleanup and part of instance construction still use the
existing elaborator; concrete K is checked independently. The semantic census
still needs import/library resolution and body recovery before it is a complete
library plan. Rust Rc, source packing, host/file adapters and test tools remain
bootstrap responsibilities.

**Next action:** implement closure capture environments and statically selected
Fn/FnMut/FnOnce calls, preserving partial capture cleanup and origin contracts;
advance class 1 source library width and semantic name resolution in parallel.

`run file.rs --engine native|k|node` selects execution; `batch cases.tsv --engine
native|k|node` amortizes compilation startup. `build -o file` emits native ELF;
`build --engine node -o file.wasm` emits Wasm. `k` prints checked K;
`check file.k` reaches the independent safety gate. `run file.k --engine k
--unchecked-k` exposes forged K to the model deliberately.
`LKJSCRIPT_TIMINGS=1` reports the stages that actually ran.
