# lkjscript

lkjscript is a language an agent can own completely and trust without a sandbox:
one small self-reproducing binary, a checked IR that admits native code by proof
instead of by confinement, and capabilities for every effect.

The lkjscript IR is the semantic boundary. An independent checker verifies types,
ownership, initialization, loans and capability use before native emission.
The Rust-shaped source language is one IR producer. Programs accepted by both
lkjscript and Rust behave the same; support for every Rust form is not a goal.

## Current toolchain

The source frontend and backend are ordinary lkjscript programs with
byte-identical self-built fixed points. The installed workflow compiles and
rebuilds them without Rust or Node. It currently joins three native executables
with a Bash driver; Cargo and Node still run the development gate.
The next milestones replace the stack-form backend, combine the toolchain into
one executable, store a bootstrap seed, then move the gate out of Rust and Node.
See [status](docs/status.md) for the current checkpoint and
[direction](docs/direction.md) for the ordered work.

An installed native toolchain provides:

```sh
/path/to/toolchain/bin/lkjscript check examples/hello.rs
/path/to/toolchain/bin/lkjscript ir examples/hello.rs -o hello.lkjir
/path/to/toolchain/bin/lkjscript build examples/hello.rs -o hello
./hello
/path/to/toolchain/bin/lkjscript run examples/hello.rs
/path/to/toolchain/bin/lkjscript eval probe.rs
```

`check` is silent on success. `run` preserves ordinary process streams and
status; `eval` also prints a scalar return. `build` writes a static Linux
x86-64 ELF with no libc or dynamic loader. The
[native workflow](docs/native.md) documents installation and rebuilding.

The Cargo-built driver also runs the independent IR interpreter:

```sh
cargo build --release
./target/release/lkjscript run examples/hello.rs --engine interpreter
LKJSCRIPT_GATE_JOBS=8 bash tools/check.sh
```

The gate compares native execution with the interpreter, checks Rust-comparable
programs, runs seeded source generators, and mutates IR directly.
The interpreter shares neither the checker nor the lowering.

## Language and authority

The language supports conditional initialization, local views, per-leaf returned
reference origins, custom Drop, generics, traits, arrays, slices, Box and Vec.
Patterns, guards, question-mark, closures and value-producing breaks lower to
ordinary IR. The source library provides text, iterators, formatting, ordered
maps and collection algorithms. Heap containers own their contents; stored
relationships use indices or owned values.

World derives authority for byte arguments, standard streams, exit status and
files beneath directory capabilities. File handles close on destruction.
Process, clock, directory mutation and listener capabilities are planned for
the repository tools and documentation server.

## Interfaces

- [IR reference](docs/ir.md): text form, operations, ownership and backend contract.
- [Language reference](docs/language.md): implemented source forms and rewrites.
- [Source producer](docs/frontend.md): declared source/library inputs and bundles.
- [Current execution implementation](docs/limits.md): representations and host boundary.

The separate Cargo-built backend uses the production checker and emitter without
linking the source frontend:

```sh
./target/release/lkjscript ir examples/hello.rs > target/hello.lkjir
./target/release/lkjscript runtime-ir > target/runtime.lkjir
./target/release/lkjscript-backend target/hello.lkjir \
    --runtime target/runtime.lkjir -o target/hello
```

Generation one and its release line remain separate; this work does not modify them.
