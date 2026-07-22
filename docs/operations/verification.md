# Verification

## Purpose

Define current evidence gates and their exact boundaries.

## Status

Source identity, source-tree, formatting, Clippy, and workspace test checks are
**Current**. Generated semantic conformance and stronger documentation checks
remain an **Accepted Target**.

## Current Local Gates

```sh
cargo run --locked -p lkjscript-xtask -- check-docs
cargo run --locked -p lkjscript-xtask -- check-tree
cargo run --locked -p lkjscript-xtask -- check-sources
cargo run --locked -p lkjscript-xtask -- quiet test
cargo run --locked -p lkjscript-xtask -- quiet verify
```

`quiet verify` currently checks:

1. required documentation paths, including architecture, experiments, and the
   canonical source-format document;
2. absence of the superseded active `docs/language/lkjml.md` path;
3. at most 16 immediate entries in every directory under the language `src`
   tree, using the compiler's shared language rule;
4. rejection of `.lkjml` and syntax validation of every `.lkjscript` source;
5. successful compilation of 11 roots whose import closures cover the corpus;
6. `cargo fmt --all -- --check`;
7. strict Clippy for the workspace, all targets, and all features;
8. workspace unit tests with the locked Cargo graph.

Test modules may locally allow panic-oriented assertion ergonomics. Product
code remains under workspace `expect`, `unwrap`, `panic`, `todo`, and
`unimplemented` denials. Runtime smokes, benchmarks, and Docker stay separate.

## Runtime Acceptance

```sh
cargo build --workspace --release --locked
./target/release/lkjscript run src/examples/hello/main.lkjscript
./target/release/lkjscript run src/examples/mandel/main.lkjscript
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/lkjedit-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/http-smoke.sh
```

The HTTP workload accepts one request and exits; it is not a general server.

## Docker Acceptance

Run from the repository checkout so the Docker build context contains Cargo,
crates, docs, metadata, and source:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

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

The next `quiet verify` revision adds:

- Markdown status and local-link checks;
- explicit placeholder scanning;
- import/source coverage validation rather than relying on a hand-maintained
  root assumption;
- focused generated type/prelude/codegen/VM conformance.

Runtime smokes remain separate so focused compiler work does not need a socket
or terminal. Docker remains the final package and installed-library gate.

## Performance Evidence

A benchmark is decision-grade only with a declared baseline, environment,
correctness oracle, randomized repetitions, dispersion, and adoption threshold.
Use [../vision/experiments.md](../vision/experiments.md). The current
single-shot C script is diagnostic only.

## Rule

A command that did not run did not pass. Historical success is not evidence for
a later commit.
