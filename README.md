# lkjscript

lkjscript is a language an agent can own completely and trust without a sandbox:
one small self-reproducing binary, a checked IR that admits native code by proof
instead of by confinement, and capabilities for every effect.

The lkjscript IR is the semantic boundary. An independent checker verifies types,
ownership, initialization, loans and capability use before native emission.
The Rust-shaped source language is one IR producer. Programs accepted by both
lkjscript and Rust behave the same; support for every Rust form is not a goal.

## Bootstrap and use

On Linux x86-64, from this checkout:

```sh
sh tools/bootstrap.sh
```

This unpacks the previous compiler stored in `seed/`, builds `target/lkjscript`
from the current source, and verifies that the new compiler reproduces itself
byte for byte. Bootstrap requires neither Rust nor Node.

`target/lkjscript` is one static executable containing the frontend, IR checker,
native backend, independent interpreter, command line, library sources and runtime
source. Copy that file to use the toolchain elsewhere; it needs no adjacent
compiler or library files.

```sh
target/lkjscript check examples/hello.rs
target/lkjscript ir examples/hello.rs -o target/hello.lkjir
target/lkjscript build target/hello.lkjir -o target/hello
target/hello
target/lkjscript run examples/hello.rs
target/lkjscript run examples/hello.rs --engine interpreter
target/lkjscript test path/to/tests.rs
```

All five commands accept source or `.lkjir` input. `check` is silent on success.
`build` writes a static Linux x86-64 ELF with no libc or dynamic loader. `run`
executes `main`, falling back to `probe`; `--entry NAME` selects another entry.
Arguments after `--` reach a World entry as exact bytes. Program streams and
exit status remain observable on both routes.

The driver prints a scalar entry's return value: integers in decimal, bool as
`true` or `false`, char as UTF-8, and f64 as its exact `f64:0x0123456789abcdef`
bit representation, each followed by a newline. Unit adds no output.

`test` runs the `#[test]` functions in one source file, or the test entries in one
IR file, on both routes by default. Select one with `--engine native` or
`--engine interpreter`. Passing tests produce one `ok N tests` line; failures
identify the entry and route. Directory discovery and expectation headers belong
to milestone 6.3. `--help` lists options; `--stats` reports phase times, counts and
peak RSS on stderr.

## Language and authority

The language supports conditional initialization, local views, per-leaf returned
reference origins, custom Drop, generics, traits, arrays, slices, Box and Vec.
Patterns, guards, question-mark, closures and value-producing breaks lower to
ordinary IR. The source library provides text, iterators, formatting, ordered
maps and collection algorithms. Heap containers own their contents; stored
relationships use indices or owned values.

World derives authority for byte arguments, streams, exit status, files and
directory changes, child processes with explicit arguments and streams, and a
monotonic clock. Owned file and directory handles close on destruction; dropping
a live Child terminates and reaps it. Listener capabilities and explicit entry
grants follow in milestone 6.4.

## References and development

- [IR reference](docs/ir.md): text form, operations, ownership and backend contract.
- [Language reference](docs/language.md): source forms, command behavior and capabilities.
- [Status](docs/status.md): current milestone, measurements and known gaps.
- [Direction](docs/direction.md): the ordered development course.

The current development gate still uses Cargo and Node:

```sh
LKJSCRIPT_GATE_JOBS=8 bash tools/check.sh
```

It compares native execution with the independent interpreter, checks
Rust-comparable programs, runs seeded source generators, mutates IR directly and
checks compiler reproduction. The interpreter shares no checker or backend
implementation. Milestone 6.3 moves the gate into lkjscript and Rust into an
optional witness.
