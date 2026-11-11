# lkjscript

A language designed for coherent ownership, explicit meaning and reliable work
by agents. Generation two uses Rust-compatible source, an explicit K core,
and WebAssembly. Cargo builds the bootstrap compiler; Node 24 runs its output.

```sh
cargo build --release
./target/release/lkjscript run tests/conformance/pass/arithmetic.rs
bash tools/check.sh
```

Source text in Git is the durable program. `check` is silent on success; `run`
prints the entry result; `build -o program.wasm` writes a WebAssembly module;
`test` runs `#[test]` functions. `k` writes readable K; `trusted` lists trusted
implementation boundaries. See [language](docs/language.md), [status](docs/status.md)
and [representation/engine constraints](docs/limits.md).

The runnable subset includes affine ownership, one-source views, custom Drop,
generics/traits, arrays, slices, Box and Vec. The source standard library loads through real module scopes, with declaration
identity, imports, visibility and prelude shadowing. `compiler::compile` accepts
an explicit source/library bundle. Structural cleanup uses growing frames and
local lowering spills excess live values into memory64 storage. Generic bodies are
checked under declared constraints, with method selections retained through
specialization. Immutable UTF-8 text and source-owned String cloning, equality and
appending execute, as do the production binary encoders, canonical-name helper and
JSON encoder. Complete generic witness/lifetime contracts, closures,
World, task scopes and compiler self-compilation remain unfinished.

Generation one is preserved on the `generation-one` branch. Its compiler, store,
services and formats are frozen and are not generation-two dependencies.
