# Status

The Rust bootstrap checks monomorphic K and runs it through Node/WebAssembly or
an independent K machine. Compiler closure, an owned native route, the fixed
point and independence from Rust/Node remain unfinished. The next changes are
ordered by their dependencies and trust cost, not by a session boundary.

## What runs

`?` with equal error types, `if let`, `while let`, diverging `let else`, nested
patterns, binding modes, guards, or-patterns, struct update and value-producing
`break` lower through the existing control and ownership mechanisms. Tag tests
now name a place so nested affine payloads need not move before a match succeeds.
Guard bindings freeze their original places until the guard decides; partial
moves and cleanup preserve the original destruction points. Nested alternatives
without bindings/guards share their pattern subtrees. Alternatives with bindings
or guards can still expand a Cartesian product; this is unfinished class 0 work.

Initialization joins track definite and possible owners and partial holes.
Reads require definite initialization; explicit DropIf cleans possible owners at
the original lexical point. Parent shells must exist before projected cleanup.
Loan inventories and loop backedges retain equality until their own contracts
land. A conditional owned field can join inside a reference-bearing aggregate
when its reference inventory is unchanged. The conditional-move corpus case runs.

Trait/implementation candidates, field lookups, loan roots, holder dependencies
and scheduled last uses are indexed. Reference containment is cached. Independent
well-typed bodies reach K checking after another body's error. The four-function
probe reports type, unknown-name, borrow and move errors together. Ownership
errors name the place and loan creation location and carry a real column.
`check` reports every recognized unsupported site; errors never emit an artifact.

The batch command compiles/runs a manifest or directory in one compiler process;
Node loads the resulting modules in one process. Every case has fresh program
state. Rust reference results are cached by source, generator/seed and rustc
identity. Independent suites run in parallel. Detail belongs under `target/`;
commands and seeds here are the record. Measurement diaries were removed.

## Observations and commands

`B=target/debug/lkjscript` below. Scaling uses `R=target/release/lkjscript` after
`cargo build --release`. These are observations of the current implementation,
not a general soundness claim. Gate timing uses a warm Rust reference cache.

| Observation | Command | Result |
| --- | --- | --- |
| Integrated gate | `bash tools/check.sh` | 13.578 s; 410 Cargo tests; 156 conformance passes, 8 pending, 135 rustc comparisons per route |
| Gate parts | same command | format 0.281 s, host 0.056 s, Rust tests 8.634 s, lint 1.070 s, build 0.029 s, parallel suites 3.502 s |
| Independent examples, both routes | `LKJ_ENGINE=node node tools/corpus.mjs B`; repeat with `LKJ_ENGINE=k` | idioms 16/30; ownership 21/30; rejected 16/16 |
| Values, each route | `LKJ_ENGINE=node node tools/generators/value.mjs B 601 100`; repeat with `k` | 92 equal values, 8 equal traps |
| Verdicts, each route | `LKJ_ENGINE=node node tools/generators/verdict.mjs B 601 100`; repeat with `k` | 73 both reject ownership, 26 equal values, 1 valid E_BORROW rejection |
| Output, each route | `node tools/generators/output.mjs B 601 50` | 39 equal normal exits, 11 equal traps |
| Conditional destruction, each route | `ONE_SIDED=1 node tools/generators/drops.mjs B 1201 300` | 300 equal outputs; no invalid generator cases or rejections |
| Sugar/expansion pairs, each route | `node tools/generators/sugar.mjs B 1301 52` | 46 equal normal exits, 6 equal traps; rustc agrees |
| Additional sugar pairs, each route | `node tools/generators/sugar.mjs R 1302 130 target/width` | 116 equal normal exits, 14 equal traps; rustc agrees |
| Outside must-reject probes | `cd /home/coder/lkjscript-claude-20261113 && node suite.mjs /home/coder/workspace/lkjscript/target/debug/lkjscript must-reject` | all 26 rejected by rustc and both routes, including r01/r02/r13/r17/r18/r19 |

All other generated outcome classes in these runs are zero. The routine gate
uses 100 destructor programs at the same seed. Setting `LKJ_SUGAR_RUST=0` keeps
the sugar/expansion oracle and removes only its Rust comparison. The expanded
pair is emitted independently by the generator; it does not call the compiler's
sugar lowering.

### Compiler census

Command: `B check --census src/lib.rs`. The first table counts recognized syntax
sites, including sites inside recoverable blocked regions. Source growth means
this is not subtraction from the old 3,412-site source snapshot.

| Unsupported form | Sites |
| --- | ---: |
| Collection iteration | 513 |
| Closures | 417 |
| Closure parameter patterns | 39 |
| `format!` | 173 |
| `for` binding patterns | 135 |
| Character literals | 86 |
| Range slicing | 30 |
| Repeated Vec literals | 26 |
| `impl` forms | 2 |
| Full range pattern | 1 |
| **Total** | **1,422** |

Blocked regions: 47 (40 statements, 6 items, 1 semantic phase).

The same command now prints a second table of 427 semantic dependency sites.
This is an inventory of the resolvable portion, not a complete library plan:
missing imports account for some dependent names, and method/trait body analysis
is still incomplete in blocked regions. Leading rows are below; the command
prints every row and location-bearing diagnostics.

| Semantic dependency | Sites |
| --- | ---: |
| `Op::Return` | 40 |
| `Type` | 39 |
| `Op::Int` | 29 |
| `Op::Drop` | 25 |
| `BTreeSet` | 24 |
| `Place` | 24 |
| `Field` | 18 |
| `Op::Borrow` | 18 |
| `Op::Read` | 17 |
| `TypeKind::Ref` | 14 |
| Import `std::collections::BTreeSet` | 12 |
| `std::result::Result` | 11 |
| Other reported dependencies | 156 |
| **Total** | **427** |

### Width scaling

Command: `node tools/scale/run.mjs R --families traits,refs,variants,fields
--sizes 2000,4000,8000 --repeat 3 --out target/scale/indexed-checkpoint`.
Values are median check milliseconds, then ratios per doubling. The variants row
was remeasured with `--families variants --out target/scale/variants-followup`
after removing one repeated arm scan; its remaining branch-fact cost is open.

| Family | 2,000 / 4,000 / 8,000 | Doubling ratios |
| --- | --- | --- |
| Trait implementations | 132.288 / 269.586 / 572.937 | 2.038 / 2.125 |
| Live shared references | 42.657 / 81.761 / 171.361 | 1.917 / 2.096 |
| Struct fields | 28.631 / 52.598 / 105.225 | 1.837 / 2.001 |
| Enum variants | 58.136 / 136.253 / 372.287 | 2.344 / 2.732 |

### Trust ledger

Command: `node tools/trust.mjs`. Physical lines include comments and blanks; size
is an audit aid, not a proof. The front end is trusted for source meaning while
K independently checks concrete safety.

| Responsibility | Lines |
| --- | ---: |
| K contract and safety gate | 5,023 |
| Layout, lowering and assembled runtime, still in wasm.rs | 2,413 |
| Other encoding and metadata | 65 |
| Production host boundary | 988 |
| Independent K model | 2,001 |

The model is an oracle, not a production prerequisite. Rust/Cargo, Node/V8 and
Linux remain dependencies. Rewriting the allocator in source will still require
its raw-memory invariants to be audited.

## Old paths and unfinished work

- Decision 3 is not yet installed: wasm.rs still combines lowering, layout,
  runtime assembly and encoding; scalars still occupy eight-byte slots. A split
  and natural-width layout are prepared outside the live compiler and need the
  full gate before acceptance. No native route exists yet.
- No new route-capacity run was made for this checkpoint. The prior million-
  function K run, 10-million-deep Node recursion and 35-million-node Node list
  remain historical observations documented in limits.md. They are not evidence
  for the changed compiler. The Node engine ceilings remain; the stack bridge is
  frozen until it can be deleted.
- Multi-root views, origin-variable call contracts, holder alternatives, loop
  invariants and reference-bearing containers need their own checked contracts.
  Closures and collection iterators wait on them.
- Pattern alternative products with guards/bindings, library breadth and full
  semantic census recovery remain. One routine verdict still rejects valid
  code with E_BORROW. The eight pending conformance cases concern closures,
  concurrency imports and parameterized trait bounds.
- Definition plans cover inference and rigid method evidence. Borrow/copy/
  coercion construction, control, cleanup and part of instance construction
  still use the existing elaborator. K is checked per concrete instance.
- Rust Rc, Cargo, library packing, host/resource adapters, Rust harness formatting,
  the rustc output shim and Node tools remain bootstrap responsibilities.

## Next action

Remove the remaining branch-fact scan in the variants family. Land the prepared
lowered form and thin Wasm encoder, then natural-width layout, each behind the
full gate. In parallel, complete shared pattern alternatives and implement
multi-root views with verified per-result-leaf call origins. Then put allocation,
growth and UTF-8 validation in one explicit runtime source bundle, before any
new runtime assembly or native backend. Continue class 1 and closure/iteration
breadth against the two census tables.

`cargo build --release`; `bash tools/check.sh` is the routine gate.
`lkjscript run file.rs --engine node|k --entry probe` selects execution;
`batch cases.tsv --engine node|k` amortizes host startup; `build -o file.wasm`
emits, `k` prints checked K and `check file.k` reaches the independent safety gate.
`run file.k --engine k --unchecked-k` deliberately exposes forged K to the model.
`LKJSCRIPT_TIMINGS=1` emits stage timings beneath the selected command.
