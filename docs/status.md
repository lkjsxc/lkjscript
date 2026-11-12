# Status

The Rust bootstrap compiler now offers memory64 WebAssembly and an independent K
machine. Compiler closure, the fixed point, an owned command line and independence
from Rust/Node remain unfinished. Work follows dependencies and corpus evidence.
Generation one remains frozen at `05a9326c`.

## Works

The existing types, generics, traits, ownership, partial moves, provenance bounds,
source library and custom Drop subset still passes its independent K gate.
World-derived Stdout writes arbitrary bytes. Output and normal/trap termination,
including destructor order, agree with a same-source rustc shim in conformance.
Pure Wasm modules have no imports; output modules import only the checked writer.
The K machine uses its own heap, live identity tokens and explicit frame/drop
stacks, and distinguishes dynamic machine errors from program traps.

Definition-local unification propagates expected types before integer defaulting.
The three named inference examples run; generic instances consume recorded plans.
Actual coercion contexts stay distinct from inferred reference types, repairing a
pre-existing mutable-reference move bug in constructors and generic calls.
Parser recovery, independent definition/K-body diagnostics, blocked-region census,
and stage timings are exposed through check. Errors never produce an artifact.

Resolved calls and indexed lexical bindings remove the measured lookup scans.
Persistent live/written facts share unchanged state across branches. Host resource
bridges reserve stacks from memory and retry only failed thread/worker creation.

## Measurements

Exact commands, identities and records: `tools/measurements/`. These observations
cover their recorded programs and sizes; they are not a general soundness claim.

| Observation | Command | Result |
| --- | --- | --- |
| Integrated gate | `bash tools/check.sh` | 35.679 s; 401 Cargo tests; 117 conformance passes, 8 pending per route; 109 rustc comparisons per route |
| Seeded values, each route | `tools/generators/value.mjs B 601 100` | 92 equal values, 8 equal traps |
| Seeded verdicts, each route | `tools/generators/verdict.mjs B 601 100` | 73 both reject ownership, 26 equal values, 1 valid rejected E_MOVE; no other outcomes |
| Seeded output, both routes | `tools/generators/output.mjs B 601 50` | 36 equal normal exits, 14 equal traps per route |
| Independent examples | `node tools/corpus.mjs B` | idioms 8/30; ownership 18/30; all 16 unsound examples rejected |
| Function check medians | `tools/scale/run.mjs B --families funcs` | 105.650 / 217.466 / 442.390 ms; doubling 2.058 / 2.034 |
| If check medians | `tools/scale/run.mjs B --families ifs` | 205.124 / 435.652 / 910.677 ms; doubling 2.124 / 2.090 |
| Match check medians | `tools/scale/run.mjs B --families match` | 38.043 / 74.067 / 148.738 ms; doubling 1.947 / 2.008 |
| Borrowed-box check medians | `tools/scale/run.mjs B --families boxes` | 235.456 / 479.694 / 1004.055 ms; doubling 2.037 / 2.093 |
| Vec check medians | `tools/scale/run.mjs B --families vecs` | 190.692 / 405.396 / 852.505 ms; doubling 2.126 / 2.103 |

Scale rows are isolated release medians of three at 8k / 16k / 32k. Records separate
load/parse, resolve, type, build K and check. Emission, engine loading and execution
are timed on the relevant commands. Peak RSS and source/K/body/emitted denominators
are reported where applicable. The verdict generator now emits additional generic
reference moves; its new tallies are not a comparison with the older generator.

## Missing and old paths

- The retained definition plan covers inference and rigid generic method evidence.
  Explicit borrows/copies/coercions, control flow, cleanup, unresolved method
  obligations and some instance construction still use the existing elaborator.
  K is monomorphic and checked for each concrete instance.
- Ownership joins remain equality joins. Loop invariants, conditional drops,
  multiple-source view contracts, reference-bearing writes through parameters and
  containers still need decision 4. Active provenance/carrier/tag maps still clone
  live contents; persistent facts remove historical-local costs only.
- Recovery stops at shared signature failures, each erroneous semantic body and
  dependent K construction. Census names blocked regions; it does not claim those
  regions were checked. Full diagnostic location/loan explanations remain work.
- Patterns, question-mark, closures, borrowed iteration, text breadth, balanced
  source collections and natural-width storage remain incomplete. The source
  BTreeMap is still a sorted vector. Debug derives have not yet been removed.
- Rust's Rc supplies shared internal nodes/tokens. Cargo, source-library packing,
  host file/process/resource adapters, the rustc output shim and Node loader/tools
  are bootstrap bridges. The K machine retains a rustc build as an independent
  executable model; source-owned host inputs and an owned backend remove the bridges.

## Bootstrap and next action

`cargo build --release`; `bash tools/check.sh` is the routine gate.
`lkjscript run file.rs --engine node|k --entry probe` selects execution;
`build -o file.wasm` emits; `k` prints checked K. `check file.k` reaches the safety
gate directly; `run file.k --engine k --unchecked-k` exposes malformed K to the
model for counterexamples. `check --census src/lib.rs` inventories compiler closure.
`LKJSCRIPT_TIMINGS=1` prints stage records to stderr. `examples/hello.rs` is the
complete World/stdout example. `tools/generators/reduce.mjs` preserves a failure's
category and Rust-validity assumption while deleting statements.

Next: finish the committed long generator measurements and remaining identity /
Debug cleanup, then implement initialization joins and conditional destruction
with forged K fixtures and the independent machine. Continue ownership in the
specified order, with breadth chosen by the census. Do not resume file-order work.
