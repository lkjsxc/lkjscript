# Verification

## Purpose

Define current evidence gates and their exact boundaries.

## Status

Source identity/closure, source-tree, documentation honesty, explicit inert
markers, formatting, Clippy, and workspace tests are **Current**. Generated
whole-prelude semantic conformance remains an **Accepted Target**.

## Current Local Gates

```sh
cargo run --locked -p lkjscript-xtask -- check-docs
cargo run --locked -p lkjscript-xtask -- check-tree
cargo run --locked -p lkjscript-xtask -- check-sources
cargo run --locked -p lkjscript-xtask -- quiet test
cargo run --locked -p lkjscript-xtask -- quiet verify
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
5. rejection of `.lkjml` and syntax validation of every `.lkjscript` source;
6. successful compilation through verified normalized SSA and validated
   bytecode for every executable root, with exact equality between their
   reported import closures and all canonical sources in the corpus;
7. `cargo fmt --all -- --check`;
8. strict Clippy for the workspace, all targets, and all features;
9. workspace unit tests with the locked Cargo graph.

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

Source-native tests additionally prove canonical source -> HIR -> verified
normalized SSA -> scalar machine plan -> encoded image -> RW/RX install ->
actual native entry. They assert installed code/W^X metadata, nonzero native
main and callee entries, direct native call counts, PollV1 counts, zero forced
fallbacks, and exact evaluator/VM/native scalar values or outcome categories.
Focused cases cover I64 multi-block loops/calls/overflow/division, F64 bits,
IEEE comparisons and mixed conversion, exit, deadline/fuel/code limits,
unsupported allocation/product/host semantics, recursion, and auto later-call
native transfer with same-epoch unsupported retry suppression.
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
./target/release/lkjscript run --engine auto --auto-jit-threshold 2 src/examples/jit-scalar/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/hello/main.lkjscript
./target/release/lkjscript run --engine vm src/examples/mandel/main.lkjscript
python3 meta/benchmarks/brainfuck/benchmark.py --mode smoke --no-build
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/lkjedit-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/http-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/bulk-bytes-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/durable-files-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/sha256-smoke.sh
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

## Performance Evidence

A benchmark is decision-grade only with a declared baseline, environment,
correctness oracle, randomized repetitions, dispersion, and adoption threshold.
Use [../vision/experiments.md](../vision/experiments.md). The current
single-shot C script is diagnostic only.

The retained callable scalar gate is:

```sh
cargo run --locked -q -p lkjscript-xtask -- quiet verify
cargo build --workspace --release --locked
python3 meta/benchmarks/jit/benchmark.py
```

It rejects fewer than four warmups or 31 samples per VM/forced/auto variant,
randomizes with a fixed recorded seed, checks exact F64 bits and stream silence,
polls `/proc` RSS, requires fallback-free forced native entry and successful
auto later-call entry, and retains every phase/sample. Results at selected
threshold 64 and alternatives 1/1,024 live under
`meta/benchmarks/jit/results/`. Implementation commit `025cbb2` measured 46.146x
native execution, 37.829x forced process wall, and 1.653x auto process wall over
same-commit VM; the full environment, dispersion, costs, pre-JIT diagnostic,
and limitations are in [Experiment C4](../vision/experiments.md#c4-callable-scalar-baseline-jit-adopted).

## Current Baseline-JIT Gates

Focused forced-native tests prove an installed W^X code object, actual generated
main and callee entries, direct relocatable native calls, versioned PollV1
calls, nonzero counts, no fallback, and exact evaluator/VM/native scalar values
or structured outcome categories. Forced unsupported semantics and native
resource failures are engine errors rather than VM fallback. Auto tests use a
low deterministic threshold and prove compilation at one call is used only by
later calls while unsupported code remains VM-correct and retry-suppressed.

The CLI implements `vm`, `auto`, and `baseline-jit`; ordinary `run` defaults to
`auto` at 64 function entries, while explicit `vm` remains deterministic.
Tests check both selections. Machine diagnostics and low-overhead metrics are
separate, stderr/file-only, opt-in, and silent during normal execution. Metrics
retain exact outcome bits, compile/HIR/effect/SSA/bytecode/native/install/first-
entry/first-call/VM/native/engine times, tier states and failures/fallbacks,
entries/direct calls/PollV1, and code/metadata/accounted cache peaks.
Allocation/reference/host paths,
recursion, OSR, optimizing JIT, GC-native references, and background compilation
are outside the current baseline subset. Performance adoption, broader
malformed/resource equivalence, and native GC evidence remain separate future
gates rather than implied by scalar callable completion.

## Rule

A command that did not run did not pass. Historical success is not evidence for
a later commit.
