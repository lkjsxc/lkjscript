# Status

The Rust bootstrap has a checked monomorphic K route to Node/WebAssembly and an
independent K machine. Compiler closure, the fixed point, an owned command line
and independence from Rust/Node remain unfinished. Dependencies and the census
set the order of work.

## Works

World-derived Stdout writes bytes under both routes and the same-source rustc
shim; conformance compares output and normal/trap termination, including Drop
order. The K machine has its own heap, live identity tokens and explicit
frame/control/copy/drop stacks. Strict Drop now rejects the repeated projected
drop found by that machine; explicit DropIf performs conditional cleanup at the
original destruction points, with named-place errors and no early cleanup.
The canonical forged fixture is `tests/conformance/k/double_projected_drop.k`.

Definition-local unification propagates expected types before integer defaulting;
the three named inference examples run. Generic instances consume recorded plans.
Resolved uses, indexed lexical bindings and a function-owner relation remove the
measured lookup scans. Actual coercion contexts remain distinct from inferred
reference types. Signature-directed intrinsic reborrowing repairs the earlier
`mem::replace(head, None)` move; the supplied `*head = n.next` case now runs.
The source graph contains no Debug derives or Debug formatting; Rust assertion
formatting lives in the bootstrap bridge.

Parser recovery, independent definition/K-body diagnostics, blocked-region
census and stage timings are exposed through check. Errors never produce an
artifact. Shared facts remove historical-local branch-copy costs. Stack bridges
reserve from host resources and retry only failed thread/worker creation.
Geometric Wasm memory growth, with exact-required fallback, removes the measured
list-construction GC cliff without changing other emitted function bodies.

## Measurements

`B` denotes the compiler binary named in the record. Exact commands, source and
binary identities are in `tools/measurements/` and `tools/scale/records/`.
Long runs below are the completed `9d3d1172` corpus; later generator extensions
emit a different corpus. Value/verdict commands select a route with
`LKJ_ENGINE=node` or `LKJ_ENGINE=k`; the output generator runs both. These observations are not a general soundness claim.

| Observation | Command | Result |
| --- | --- | --- |
| Integrated gate | `bash tools/check.sh` | 34.181 s; 409 Cargo tests; 128 conformance passes, 8 pending and 114 rustc comparisons per route |
| Long values, each route | `node tools/generators/value.mjs B SEED 500`, seeds 601–604 | 2,000: 1,749 equal values, 251 equal traps |
| Long verdicts, each route | `node tools/generators/verdict.mjs B SEED 500`, seeds 601–604 | 2,000: 1,423 both reject ownership, 539 equal values, 4 equal traps, 20 valid E_MOVE and 14 valid E_BORROW rejections; all other classes zero |
| Long output, each route | `node tools/generators/output.mjs B 602 100` | 83 equal normal exits, 17 equal traps |
| Current routine output, each route | `node tools/generators/output.mjs B 601 50` | 39 equal normal exits, 11 equal traps; generator includes partial moves and replacement Drop order |
| Independent examples | `node tools/corpus.mjs B` | idioms 8/30; ownership 19/30; all 16 unsound source examples rejected |
| Compiler census | `B check --census src/lib.rs` | d32a611d: 3,412 unsupported sites, 109 blocked regions |
| Function check | `node tools/scale/run.mjs B --families funcs` | 106.367 / 224.256 / 442.555 ms; doubling 2.108 / 1.973 |
| If check | `node tools/scale/run.mjs B --families ifs` | 225.109 / 452.159 / 931.412 ms; doubling 2.009 / 2.060 |
| Match check | `node tools/scale/run.mjs B --families match` | 40.351 / 76.781 / 154.848 ms; doubling 1.903 / 2.017 |
| Borrowed-box check | `node tools/scale/run.mjs B --families boxes` | 237.331 / 494.667 / 1057.462 ms; doubling 2.084 / 2.138 |
| Vec check | `node tools/scale/run.mjs B --families vecs` | 189.922 / 408.475 / 856.424 ms; doubling 2.151 / 2.097 |
| Method owners, 2k/4k/8k | `node tools/scale/run.mjs B --families methods --sizes 2000,4000,8000` | 32.209 / 59.284 / 115.578 ms; doubling 1.841 / 1.950 |
| Lists, Node, 20m/35m | `node tools/scale/list_phases.mjs B --sizes 20000000,35000000` | 2.421 / 4.459 s; 1.842× time for 1.75× nodes; peak RSS 9.11 / 16.41 GB |
| Large routes | recorded capacity commands | 1m-function chain runs on K (1.037 s execution, 6.71 GB process-tree peak); Node stops at engine load; Node non-tail recursion depth 10m runs |
| Code footprint | `node tools/count-lines.mjs` | 25,973 compiler; 455 library; 2,294 embedded compiler tests; 13,554 Rust harness; 1,051 conformance source + 293 K fixtures; bridges counted separately |

Code counts use physical lines including blanks/comments; direct `#[test]` and
`#[cfg(test)]` items are separated by balanced token spans. The same command
counts baseline git objects; records are `tools/measurements/20261112-lines*.json`.
The five original scale families use d32a611d release medians of three at 8k/16k/32k;
method and list rows state their sizes. Stage records distinguish compilation,
engine load and execution. List phase/profile records locate the old cliff in
V8 external-memory GC during construction. The 50m list constructs and sums,
then traps R0006 during drop at Node's 16 GiB linear-memory extent.

## Broken and old responsibilities

- Recorded definition plans cover inference and rigid method evidence. Explicit
  borrows/copies/coercions, control flow, cleanup, unresolved symbolic method
  obligations and part of instance construction still use the existing
  elaborator. Nominal/alias term identities and repeated field/variant choices
  remain to migrate. K remains monomorphic and checked per concrete instance.
- Ownership joins still require equality. Initialization joins, loop invariants,
  holder liveness, origin-variable call contracts and reference-bearing containers
  remain. Active origin/carrier/tag maps still clone their live contents. Model
  loan scanning and transitive-parent costs are unmeasured.
- Recovery blocks shared-signature failures, failed semantic bodies and dependent
  K construction. Blocked regions are not analyzed; detailed place/loan locations
  remain work. Patterns, question-mark, closures, borrowed iteration, text breadth,
  balanced source collections and natural-width storage remain incomplete. The
  source BTreeMap is still a sorted vector.
- Rust Rc, Cargo, library packing, host/resource adapters, Rust test formatting,
  the rustc output shim and Node loader/tools remain bootstrap bridges. Keep the
  rustc-built K machine as an independent judge while source-owned host inputs,
  tools and execution replace those bridges.

## Bootstrap and next action

`cargo build --release`; `bash tools/check.sh` is the routine gate.
`lkjscript run file.rs --engine node|k --entry probe` selects execution;
`build -o file.wasm` emits, `k` prints checked K, and `check file.k` reaches
the independent safety gate. `run file.k --engine k --unchecked-k` exposes
forged K directly to the model. `LKJSCRIPT_TIMINGS=1` emits stage records;
`examples/hello.rs` is the complete World example. The statement reducer keeps
the failure category and Rust-validity assumption.

Next: persistent must/may initialization for reference-free roots, with
conditional cleanup at original lexical points. Require present parent shells before partial-place
joins; keep loan and loop-backedge equality until their own contracts land.
Continue decision 4 with holder sets, loop invariants, call origins and containers;
choose breadth by dependency and census.
