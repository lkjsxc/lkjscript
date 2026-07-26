# Verification: Current Local Gates

[Authority](../verification.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Current Local Gates

```sh
cargo run --locked -p lkjscript-xtask -- check-docs
cargo run --locked -p lkjscript-xtask -- check-tree
cargo run --locked -p lkjscript-xtask -- check-sources
cargo run --locked -p lkjscript-xtask -- quiet test
cargo run --locked -p lkjscript-xtask -- quiet verify
python3 meta/results/ai-authoring/validate.py \
  meta/results/ai-authoring/results/*.json
```

`quiet verify` currently checks:

1. required documentation paths, including architecture, experiments, numeric,
   AI-first, and explicit equality semantics, typed compiler/JIT pipeline, the
   selected Linux x86-64 native backend, runtime-JIT/no-PGO decision,
   performance scorecard, and the canonical
   source-format document;
2. a `Status` section and valid local links in every `docs/**/*.md`, plus local
   link validity in root Markdown and absence of the superseded active
   `docs/language/lkjml.md` path;
3. exact uppercase `PLACEHOLDER` labeling for any inert marker in Rust or
   lkjscript source;
4. at most 16 immediate entries in every directory under the language `src`
   tree, using the compiler's shared language rule;
5. rejection of `.lkjml` and validated Semantic Source Foundation construction
   for every `.lkjscript` source;
6. successful compilation through verified normalized SSA and validated
   bytecode for every executable root, with exact equality between their
   reported import closures and all canonical sources in the corpus;
7. `cargo fmt --all -- --check`;
8. strict Clippy for the workspace, all targets, and all features;
9. workspace unit tests with the locked Cargo graph.

Workspace tests independently require exact structural parse/format/parse and
byte roundtrip for all 133 tracked `.lkjscript` sources, revision/key/node/path
invariants, iterative deep import/tree behavior, source safety maxima, and
malformed source/descriptor containment boundaries. The Python command validates
retained benchmark schemas and verdict consistency; it does not invoke a model
or fabricate missing semantic/hole variants.

Workspace tests also cover the initial `Owned Buf` safe island: exact type and
operation signatures; generic-laundering and direct/nested aggregate-storage
rejection; direct temporary/let Borrow placement and full-expression loan
extent; affine move/reinitialization failures; lexical and branch-local place
state/result transfer; shared/exclusive conflicts; same-block NLL; consumed
`RefMut` frame liveness; constant-loop cleanup; and exact branch/loop state
boundaries. Public malformed-SSA tests cover original-value reuse after Move,
explicit affine edge transport, equal/mismatched branches, duplicate call/edge
affine arguments, owner aliasing, missing entry/local provenance, cross-block
Borrow rejection, terminator reuse, duplicate LoanIds, duplicate active
`PlaceEnd`, loop mismatch, pass preservation, collection-nested function-type
laundering, explicit affine arm transport, the 4,096-block function cap, bitset
dominance, and bounded ownership/CFG/state work. Evaluator/VM equivalence uses the same
zero-allocation boundary for the focused owned-buffer resource check, and the
scalar-returning forced-baseline fixture reaches allocation/borrow/read before
an ownership/reference support rejection.

Workspace tests include focused numeric and explicit-equality
parser/type/HIR/SSA/bytecode/VM/host boundaries, removed equality vocabulary
and opcodes, and compiled source-to-VM execution across immediate and boxed I64
values. Typed-SSA tests directly cover malformed IDs, use-before-definition,
dominance/edge/loop/effect failures; deterministic isolated and combined
passes; exact bounded evaluation of scalar/control/calls/recursion/local
mutation/products/Option/Result/lists/strings/buffers/traps/exits; explicit
unsupported host operations; focused VM equivalence; and 64 deterministic
bounded randomized type-correct scalar programs. Bytecode tests also cover
whole-chunk reachable/unreachable decode, random small-byte robustness without
panic, explicit Trap, size/index/metadata/product/category/CFG/local
initialization/return validation, owned returned heap values, independent VMs
after exit/trap, and structured fuel/stack/frame/heap/allocation/handle/output/
deadline and hard-deadline-unsupported outcomes.

Native-foundation workspace tests pass only verified closed machine plans to
the encoder and actually call generated Linux x86-64 code for multi-block
scalar control flow, loops, checked I64 trap families, F64 arithmetic/bits/all
ordered comparisons and NaN branches, Bool/Unit, direct generated calls, and
versioned runtime calls. Boundary tests reject invalid plans, unsupported
signatures, code/metadata/work and aggregate install limits, and ABI mismatches;
observe RW then RX permissions through a sys-internal `/proc/self/maps` probe;
and repeat owned install/invoke/drop accounting.

Proof-optimization tests cover deterministic stable-ID algebraic and same-block/
dominator GVN certificates, checked-I64 successful-check reuse, evaluator
equivalence, independently reconstructed exact candidates, ordinary post-pass
verification, and rejection of missing/stale/reordered/wrong-operation/
wrong-operand/non-dominating/effectful/over-budget certificates, plus 64
bounded randomized type-correct scalar evaluator differentials. The public
optimizing authority has no raw `Program` constructor.

the canonical source contract control tests cover early return, divergent `Never` joins, nearest
nested continue, typed value-loop break, Unit-only while break, dynamic Str
trap values, and structured exit across the independent SSA evaluator,
validated reference bytecode/VM, forced baseline, and forced proof JIT. They
require generated entry and zero fallback. Malformed tests reject Never in
public return, parameter, local, collection, product/enum field, and enum
substitution positions; unreachable sequence fallthrough, non-loop control,
wrong while breaks, stale block targets/arguments, and forged non-Str SSA trap
operands fail closed. Semantic Source tests cover closed Never/control nodes,
exact hole control requirements, and checker-valid legal actions.

Source-native tests additionally prove canonical source -> HIR -> verified
normalized SSA -> scalar/reference machine plan -> encoded image -> RW/RX
install -> actual native entry and typed heap runtime sites. They assert installed code/W^X metadata, nonzero native
main and callee entries, direct native call counts, PollV1 counts, zero forced
fallbacks, and exact evaluator/VM/native scalar values or outcome categories.
Focused cases cover I64 multi-block loops/calls/overflow/division, F64 bits,
IEEE comparisons and mixed conversion, exact selected conditional/callee trap
messages, exit, deadline/fuel/code/heap/allocation and zero/tiny active-value
limits, unsupported ownership/host semantics, direct and mutual recursion with
live references, exact/MAX+1 bounded list equality, native buffer Result error
boundaries, nested Product/Option/Result/List/Str/Buf graphs, and auto scalar
later-call transfer while compiled reference helpers remain entry-ineligible.
Test modules may locally allow panic-oriented assertion ergonomics. Product
code remains under workspace `expect`, `unwrap`, `panic`, `todo`, and
`unimplemented` denials. Runtime smokes, benchmarks, and Docker stay separate.

The compiler API returns `ExecutableProgram`, retaining verified normalized SSA,
bytecode link metadata, and `ValidatedChunk`; the latter is available only
through an explicit bytecode accessor. VM and disassembly therefore cannot
accidentally execute a raw builder `Chunk`. Validation failure remains a
compile/validation error rather than an `ExecutionOutcome`.
## Runtime Acceptance

```sh
cargo build --workspace --release --locked
./target/release/lkjscript run src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine baseline-jit src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine optimizing-jit src/examples/jit-optimizing/main.lkjscript
./target/release/lkjscript run --engine auto --auto-jit-threshold 2 src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/hello/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/mandel/main.lkjscript
python3 meta/benchmarks/brainfuck/benchmark.py --mode smoke --no-build
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/lkjedit-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/http-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/bulk-bytes-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/durable-files-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/sha256-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/sqlite-smoke.sh
```

The HTTP workload accepts one request and exits; it is not a general server.
## Docker Acceptance

Run from the repository checkout so the Docker build context contains Cargo,
crates, docs, metadata, and source:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

This exact command passed on the final Linux x86-64 implementation tree with
`result=ok`; it rebuilt the image and reran the canonical workspace gate plus
release hello, Mandelbrot, lkjedit, and HTTP acceptance. Full Brainfuck
Mandelbrot remains intentionally unrun because it is the next-cycle OSR
workload.

For an external project, first build the runtime image from the repository,
then run the project from its own directory:

```sh
# In the lkjscript repository:
docker build -f meta/Dockerfile --target runtime -t lkjscript .

# In the external project directory:
docker run --rm -it -v "$PWD:/project" -w /project lkjscript \
  run main.lkjscript
```
## Accepted Gate Revision

The next `quiet verify` revision adds generated whole-prelude
Type/prelude/codegen/VM conformance. Focused numeric cross-layer conformance is
already current.

Runtime smokes remain separate so focused compiler work does not need a socket
or terminal. Docker remains the final package and installed-library gate.
