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
target/lkjscript test tests/conformance --jobs 8
```

All five commands accept source or `.lkjir` input. `check` is silent on success.
`build` writes a static Linux x86-64 ELF with no libc or dynamic loader. `run`
executes `main`, falling back to `probe`; `--entry NAME` selects another entry.
Arguments after `--` reach a World entry as exact bytes. Program streams and
exit status remain observable on both routes.

The driver prints a scalar entry's return value: integers in decimal, bool as
`true` or `false`, char as UTF-8, and f64 as its exact `f64:0x0123456789abcdef`
bit representation, each followed by a newline. Unit adds no output.

`test` runs `#[test]` functions, IR test entries, or single-file expectation
cases found recursively in a directory. Native workers execute both routes and
check source/IR byte identity. Select one route with `--engine native` or
`--engine interpreter`; `--jobs` selects up to eight workers. Passing suites
produce one line; failures identify the entry and route. See the
[case format](tests/conformance/README.md). `--help` lists options; `--stats`
reports phase times, counts and peak RSS on stderr.

`build --profile` adds CPU sampling and a function-address map to the executable.
Each process writes its own `OUTPUT.samples.PID` file, with a twenty-digit PID.
Use `lkjscript profile SAMPLE_FILE` for a compact report of self and inclusive
function samples. Profiling needs no external profiler or symbol/debug package.

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

The development gate bootstraps from the stored seed and needs no Rust or Node:

```sh
LKJSCRIPT_GATE_JOBS=8 bash tools/check.sh
```

It compares native execution with the independent interpreter, checks serialized
IR and executable identity, runs seeded source generators, mutates and reduces
IR, and verifies compiler reproduction. Semantic tests, the runner, generators,
mutator, reducer, and process tests are lkjscript programs. The interpreter shares
no checker or backend implementation.

Rust is an optional witness under `witness/`; the repository root is not a Cargo
package. `bash witness/check.sh` compares compiler bytes and marked source cases
using Rust, with a native coordinator. Its CI job is advisory.
