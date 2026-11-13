# lkjscript

A language designed for coherent ownership, explicit meaning and reliable work
by agents. Generation two uses Rust-compatible source, an independently checked K
core, one lowered representation and a source runtime. Cargo builds the bootstrap
compiler. Its default output runs as static Linux x86-64 machine code without
Node, libc or a dynamic loader.

```sh
cargo build --release
./target/release/lkjscript run examples/hello.rs
./target/release/lkjscript build examples/hello.rs -o target/hello
./target/hello
bash tools/check.sh
```

Source text in Git is the durable program. `check` is silent on success; `run`
preserves World output and prints scalar entry results; `build` writes an ELF;
`test` runs `#[test]` functions. `--engine k` selects the independent execution
model. `--engine node` selects the optional memory64 WebAssembly route.
`k` writes readable K; `trusted` lists implementation boundaries. See
[language](docs/language.md), [status](docs/status.md) and
[representation and host constraints](docs/limits.md).

The runnable subset includes affine ownership with conditional initialization,
multi-root views, per-leaf call origins, custom Drop, generics and traits, arrays,
slices, Box and Vec. Patterns, guards, or-patterns, question-mark, struct update and
value-producing breaks lower through the existing K operations. World-derived
output is explicit. The source library loads through real module scopes with
declaration identity, imports, visibility and prelude shadowing.

The compiler takes declared source, library and runtime inputs. Natural-width
layout, copy/drop/equality glue and runtime calls are applied once before
encoding. Allocation, growth, UTF-8 validation and the iterative cleanup walker
are source functions. Native and WebAssembly execute that same runtime; K keeps
an independent owner model. Generic bodies are checked under declared constraints
with method selections retained through specialization. Derived Clone/equality
and source Option/Result, String and Vec traits execute.

The production binary encoders, canonical-name helper, JSON encoder and complete
K representation module also run through the ordinary language pipeline, with
Rust comparisons and source mutations. Complete generic witness/lifetime
contracts, closures, collection iteration, task scopes and compiler
self-compilation remain unfinished.

Generation one is preserved on the `generation-one` branch. Its compiler, store,
services and formats are frozen and are not generation-two dependencies.
