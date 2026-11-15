# lkjscript

lkjscript is a language an agent can own completely and trust without a sandbox:
one small self-reproducing binary, a checked IR that admits native code by proof
instead of by confinement, and capabilities for every effect.

The lkjscript IR is the semantic boundary. An independent checker verifies types,
ownership, initialization, loans and capability use before native emission.
The Rust-shaped source language uses `.lkj` files and is one IR producer.
Programs accepted by both lkjscript and Rust behave the same; support for every
Rust form is not a goal. Modules use `child.lkj` or `child/mod.lkj`.

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
target/lkjscript check examples/hello.lkj
target/lkjscript ir examples/hello.lkj -o target/hello.lkjir
target/lkjscript build target/hello.lkjir -o target/hello
target/hello
target/lkjscript run examples/hello.lkj
target/lkjscript run examples/hello.lkj --engine interpreter
target/lkjscript test path/to/tests.lkj
target/lkjscript test tests/conformance --jobs 4
target/lkjscript authority examples/http.lkj
```

All six commands accept source or `.lkjir` input. `check` is silent on success.
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

## Program directories

A directory's `program.lkj` is the reviewed invocation. It contains existing
source constant forms, with literal values only:

```rust
const SOURCE: &str = "main.lkj";
const ENTRY: &str = "main";
const GRANTS: &[&str] = &["out", "clock", "listen=127.0.0.1:8080"];
const CASES: &[&str] = &["cases"];
```

`build path/to/program`, `test path/to/program`, and `authority path/to/program`
use the same entry and grant values. `run` accepts the directory too. Ordinary
module discovery follows `SOURCE`; source and case paths remain beneath the
directory. Directory grant values are relative to that directory unless absolute or the existing env:NAME invocation selector.
The entry signature alone determines authority: missing and unrequested grants
are errors naming both `program.lkj` and the entry's parameters. Command-line
entry, grant and source-root overrides reject instead of bypassing the review.
Testing first validates that invocation, then runs the listed case files or
collections. No manifest expression is executed. The default build output is
`path/to/program/program.elf`; `-o` can select another artifact path.

## Reuse across builds

Bootstrap and the gate use a local, ignored `target/store`. Explicit builds opt in:

```sh
store_key=$(sh tools/store-key.sh)
target/lkjscript build examples/hello.lkj -o target/hello \
    --store target/store --store-key "$store_key"
```

The store retains authenticated checker successes, relocatable function code,
complete images and closed tests using the embedded runtime. Both normal returns
and language traps reuse observations; resource and host failures rerun. Effects
still execute on every test run. Different compiler generations keep separate entries. Every entry binds the exact
compiler, target and dependencies; a changed
source file or previously absent module invalidates the affected result.
`--no-store` disables reuse. Deleting the store is always safe.

The signing key lives outside source and program Directory grants, normally under
`$XDG_STATE_HOME/lkjscript` or `$HOME/.local/state/lkjscript`. Keep it private:
key readers and compiler replacers have the same trust. An untrusted store writer
cannot authorize unchecked code. See [direction](docs/direction.md) for the trust
rule and [benchmarks](bench/README.md) for cold, warm and one-edit measurements.

## Language and authority

The language supports conditional initialization, local views, per-leaf returned
reference origins, custom Drop, generics, traits, arrays, slices, Box and Vec.
Patterns, guards, question-mark, closures and value-producing breaks lower to
ordinary IR. The source library provides text, iterators, formatting, ordered
maps and collection algorithms. Heap containers own their contents; stored
relationships use indices or owned values.

An entry's owned capability parameters are its authority manifest. Supply each by
name with `--grant NAME`; Directory requires `--grant NAME=PATH`, and Listener
and Connector require `--grant NAME=IPv4:PORT`. A sole World parameter declares implicit full
authority. Missing, duplicate and unknown grants are rejected before execution or
artifact publication. `build` records the grants; the executable acquires the
handles at startup. `run` supports the same grants on both execution routes.

World derives byte arguments, streams, exit status, files and directory changes,
child processes with explicit arguments and streams, a monotonic clock, and IPv4
TCP listeners and exact numeric outbound destinations. Directory confines descendant
operations beneath its root. Connector connects only to its granted destination;
Listener accepts connections as owned Files. File, Directory, Listener and Connector close
on destruction; dropping a live Child terminates and reaps it.

`lkjscript authority PATH` reports the declared entry grants and conservatively
reachable effects, calls and owned cleanup from checked IR. Source and serialized
IR produce the same report. The [HTTP example](examples/README.md) serves a granted
directory with explicit Stdout, Directory, Listener and Clock parameters, and
includes a generated-traffic harness with memory measurement and CPU profiling.

## References and development

- [IR reference](docs/ir.md): text form, operations, ownership and backend contract.
- [Language reference](docs/language.md): source forms, command behavior and capabilities.
- [Status](docs/status.md): current milestone, measurements and known gaps.
- [Direction](docs/direction.md): the ordered development course.

The development gate bootstraps from the stored seed and needs no Rust or Node.
Its HTTP test uses an explicit lkjscript client:

```sh
LKJSCRIPT_GATE_JOBS=4 bash tools/check.sh
```

The gate accepts 2..4 workers per stream; the HTTP server/client pair needs two.
Individual `lkjscript test --jobs 1` suites can still run with one worker.

It compares native execution with the independent interpreter, checks serialized
IR and executable identity, runs seeded source generators, mutates and reduces
IR, and verifies compiler reproduction. Semantic tests, the runner, generators,
mutator, reducer, and process tests are lkjscript programs. The interpreter shares
no checker or backend implementation.

Rust is an optional witness under `witness/`; the repository root is not a Cargo
package. `bash witness/check.sh` compares compiler bytes and marked source cases
using Rust, with a native coordinator. Its CI job is advisory.
