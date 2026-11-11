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
generics/traits, arrays, slices, Box and Vec. The source standard library is
exercised by differential tests. General module loading, explicit generic witnesses
in K, closures, World, task scopes and self-compilation remain unfinished.

Generation one is preserved on the `generation-one` branch. Its compiler, store,
services and formats are frozen and are not generation-two dependencies.
