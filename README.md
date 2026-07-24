# lkjscript

## Purpose

`lkjscript` is a typed, line-oriented functional language with a compact Rust
compiler, dense bytecode VM, fixed source budgets, and an ecosystem intended to
grow in lkjscript itself rather than in host frameworks.

## Status

Canonical source files use **`.lkjscript`**. Other extensions, including the
superseded `.lkjml`, are rejected without a compatibility mode. Linux x86-64 is
the current acceptance platform.

Current capabilities and known defects are recorded in
[docs/current-state.md](docs/current-state.md). Future installation, update,
package, GUI, browser, server, loop-OSR, optimizing-JIT, and full-language
native work remains direction rather than current product behavior. The exact
current allocation-free scalar baseline tier is documented separately.

## Development Commands

```sh
cargo run --locked -p lkjscript-app -- run --engine vm src/examples/hello/main.lkjscript
cargo run --locked -p lkjscript-app -- run --engine baseline-jit src/examples/jit-scalar/main.lkjscript
cargo run --locked -p lkjscript-app -- run --engine auto --auto-jit-threshold 2 src/examples/jit-scalar/main.lkjscript
cargo run --locked -p lkjscript-app -- run --engine vm src/examples/mandel/main.lkjscript
cargo run --locked -p lkjscript-app -- run src/examples/lkjedit/main.lkjscript path/to/file
cargo run --locked -p lkjscript-app -- run src/examples/http/hello.lkjscript
cargo run --locked -p lkjscript-app -- disasm src/examples/hello/main.lkjscript
cargo run --locked -p lkjscript-xtask -- quiet verify
```

Runtime acceptance:

```sh
cargo build --workspace --release --locked
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/lkjedit-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/http-smoke.sh
LKJSCRIPT_BIN=target/release/lkjscript meta/scripts/sqlite-smoke.sh
```

Docker acceptance, run from this repository:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

## Architecture

- `crates/lkjscript-core`: bytecode and shared value contracts
- `crates/lkjscript-ir`: verified typed SSA, evaluator, and baseline passes
- `crates/lkjscript-compiler`: loading, parsing, typing, SSA, and bytecode lowering
- `crates/lkjscript-vm`: interpreter, heap, host operations, and auto entry transfer
- `crates/lkjscript-native`: source-independent closed Linux x86-64 scalar foundation
- `crates/lkjscript-jit`: verified scalar SSA adapter, code objects, and tier state
- `crates/lkjscript-sys`: owned Linux FFI/W^X boundary and the only unsafe Rust
- `crates/lkjscript-app`: command-line product
- `crates/lkjscript-xtask`: repository honesty gates
- `src/std`: language-level standard library
- `src/lib`: reusable language packages
- `src/examples`: executable acceptance workloads

Each lkjscript source directory may contain at most 16 immediate files plus
subdirectories. This language rule does not constrain Rust, docs, metadata, or
generated build trees.

See [docs/operations/architecture.md](docs/operations/architecture.md).

## Read Order

1. [docs/current-state.md](docs/current-state.md)
2. [docs/operations/architecture.md](docs/operations/architecture.md)
3. [docs/language/README.md](docs/language/README.md)
4. [docs/operations/verification.md](docs/operations/verification.md)
5. [docs/vision/README.md](docs/vision/README.md)

Agent instructions: [AGENTS.md](AGENTS.md)
