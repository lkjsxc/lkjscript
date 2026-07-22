# Verification

## Purpose

Define evidence gates and distinguish their current and accepted scope.

## Status

The baseline commands are **Current**. Stronger documentation, source-tree,
formatting, Clippy, and conformance gates are an **Accepted Target**.

## Current Local Gates

```sh
cargo run --locked -p lkjscript-xtask -- check-docs
cargo run --locked -p lkjscript-xtask -- check-tree
cargo run --locked -p lkjscript-xtask -- check-sources
cargo run --locked -p lkjscript-xtask -- quiet test
cargo run --locked -p lkjscript-xtask -- quiet verify
```

At baseline, `quiet verify` checks selected document existence, the old global
eight-visible-entry tree rule, syntax for `.lkjml` under `src`, nine hardcoded
compile roots, and workspace unit tests. It does not run rustfmt, Clippy,
runtime smokes, benchmarks, or Docker.

Separate current runtime acceptance:

```sh
cargo build --workspace --release --locked
LKJ=target/release/lkjscript meta/scripts/lkjedit-smoke.sh
LKJ=target/release/lkjscript meta/scripts/http-smoke.sh
```

The pre-cutover commands and results are recorded in
[../current-state.md](../current-state.md).

## Docker

The current full acceptance image is built explicitly to avoid stale-image
success:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Docker was not rerun for the recorded foundation baseline and is not claimed as
passing there.

## Accepted Local Gate

After the foundation cutover, `quiet verify` must check:

1. required documentation and valid status/placeholder labeling;
2. the 16-entry rule only for lkjscript source directories;
3. `.lkjscript` source inventory and rejection of `.lkjml`;
4. import suffix, containment, cycle, and corpus coverage;
5. focused type/prelude/codegen/VM conformance;
6. `cargo fmt --all -- --check`;
7. strict workspace Clippy;
8. workspace tests.

Runtime smokes remain separate so focused compiler work is not forced to open a
socket or terminal on every test run. Docker remains the final packaging and
installed-library acceptance gate.

## Performance Evidence

A benchmark is not a pass/fail correctness gate unless it has a declared
baseline, repetitions, noise measure, correctness oracle, and adoption
threshold. Use [../vision/experiments.md](../vision/experiments.md). A
single-shot C comparison is diagnostic only.

## Rule

A gate that did not run did not pass. A historical pass is not evidence for a
new commit.
