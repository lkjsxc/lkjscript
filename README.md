# lkjscript

## Purpose

`lkjscript` is a typed, line-oriented functional language with a compact Rust
compiler, dense bytecode VM, explicit source budgets, and an ecosystem intended
to grow in lkjscript itself rather than in host frameworks.

## Status

The checked-out implementation still uses `.lkjml` during the documentation-first
foundation cutover. The accepted canonical extension is **`.lkjscript`**, with
no compatibility mode. Current evidence and the exact accepted target are kept
separate in [docs/current-state.md](docs/current-state.md).

## Current Development Commands

These commands describe the pre-cutover baseline at commit `8aa09d8`:

```sh
cargo run --locked -p lkjscript-app -- run src/examples/hello/main.lkjml
cargo run --locked -p lkjscript-app -- run src/examples/mandel/main.lkjml
cargo run --locked -p lkjscript-app -- run src/examples/lkjedit/main.lkjml path/to/file
cargo run --locked -p lkjscript-app -- run src/examples/http/hello.lkjml
cargo run --locked -p lkjscript-xtask -- quiet verify
```

After the accepted cutover, the same source paths end in `.lkjscript`; `.lkjml`
is rejected rather than aliased.

## Architecture

- `crates/lkjscript-core`: bytecode and shared value contracts
- `crates/lkjscript-compiler`: loading, parsing, typing, and code generation
- `crates/lkjscript-vm`: interpreter, heap, and host-operation dispatch
- `crates/lkjscript-sys`: owned Linux FFI and the only unsafe Rust
- `crates/lkjscript-app`: command-line product
- `crates/lkjscript-xtask`: repository honesty gates
- `src/std`: language-level standard library
- `src/lib`: reusable language packages
- `src/examples`: executable acceptance workloads

See [docs/operations/architecture.md](docs/operations/architecture.md) for the
compile and execution flows.

## Direction

The long-term direction includes self-contained installation and update,
packages, process-safe multi-application execution, native performance work,
servers, frameworks, browsers, and multiplatform GUI applications. These are
not current capability claims. Each layer must land through reproducible
experiments and truthful contracts.

## Read Order

1. [docs/current-state.md](docs/current-state.md)
2. [docs/operations/architecture.md](docs/operations/architecture.md)
3. [docs/language/README.md](docs/language/README.md)
4. [docs/operations/verification.md](docs/operations/verification.md)
5. [docs/vision/README.md](docs/vision/README.md)

Agent instructions: [AGENTS.md](AGENTS.md)
