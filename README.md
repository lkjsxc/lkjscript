# lkjscript

A Rust-compatible subset for reliable work by agents, with affine ownership,
explicit capabilities and an independently checked K core. Cargo builds the
bootstrap compiler. Its output is static Linux x86-64 machine code without Node,
libc or a dynamic loader. The project is moving toward an owned compiler and tools;
the production K compiler is the first self-compilation target.

```sh
cargo build --release
./target/release/lkjscript run examples/hello.rs
./target/release/lkjscript build examples/hello.rs -o target/hello
./target/hello
bash tools/check.sh
```

`check` is silent on success; `run` preserves World output and prints scalar
entry results; `build` writes an ELF; `test` runs test functions. `--engine k`
selects the independent execution model. `k` writes readable K, `runtime-k`
exports the prepared source runtime, and `trusted` lists implementation boundaries.
Native artifacts take execution extents from Linux at startup. Runner budgets
are explicit host controls; they do not restrict accepted source.

The runnable subset includes conditional initialization, multi-root local views,
per-leaf call origins, custom Drop, generics and traits, arrays, slices, Box and
Vec. Patterns, guards, question-mark, closures and value-producing breaks lower
through ordinary K operations. The source library provides text, iterators,
formatting, ordered maps and collection algorithms. Heap containers own their
contents; stored relationships use indices or owned values. World-derived
output is explicit. Arguments, input and directory-relative files are the next
capability work.

The compiler takes declared source, library and runtime inputs. Layout,
copy/drop/equality glue and runtime calls are applied once. Allocation, growth,
UTF-8 validation and cleanup are source functions. Native execution and the K
machine provide independent semantic checks. A separate cargo-built `kc` uses
the production K checker, lowering and native encoder without linking the source
front end:

```sh
./target/release/lkjscript k examples/hello.rs > target/hello.k
./target/release/lkjscript runtime-k > target/runtime.k
./target/release/kc target/hello.k --runtime target/runtime.k -o target/hello-kc
node tools/kc.mjs
```

That boundary now has byte-identity observations; it is not yet a self-built
compiler. [Status](docs/status.md) records current evidence and open obligations.
[Direction](docs/direction.md) defines the consumer-led route out of Rust;
[language](docs/language.md) describes the implemented contract and
[constraints](docs/limits.md) records representation and execution boundaries.

New semantic tests are source/K data. The gate compares native and K results
with Rust under the declared host, exercises generated programs and checks the
K-to-ELF boundary. WebAssembly and its Node execution host have been removed.
Node remains temporary test orchestration. Generation one is preserved on its
branch and is not a generation-two dependency.
