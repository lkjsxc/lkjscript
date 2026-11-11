# lkjscript

A language designed for coherent ownership, explicit meaning and reliable work
by agents. Generation two uses Rust-compatible source, an explicit K core,
and WebAssembly. The compiler is bootstrapped with Cargo and runs generated
programs on Node 24.

```sh
cargo build --release
./target/release/lkjscript run tests/conformance/pass/arithmetic.rs
bash tools/check.sh
```

Source text in Git is the durable program. `check` is silent on success; `run`
prints the entry result; `build -o program.wasm` writes a WebAssembly module;
`test` runs `#[test]` functions. See [language](docs/language.md),
[status](docs/status.md) and [limits](docs/limits.md).

Generation one is preserved on the `generation-one` branch. Its compiler, store,
services and formats are frozen and are not generation-two dependencies.
