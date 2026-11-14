# lkjscript

A Rust-compatible subset for reliable work by agents, with affine ownership,
explicit capabilities and an independently checked K core. Cargo builds the
bootstrap compiler. Its output is static Linux x86-64 machine code without Node,
libc or a dynamic loader. The production source front end and K compiler are
self-built, with exact K and ELF fixed points. The installed source-to-K and
K-to-ELF pipeline, input packing, runtime preparation and compiler rebuilds run
without Rust or Node. Cargo and Node remain development/reference tools.

With an installed native toolchain:

```sh
/path/to/toolchain/bin/lkj check examples/hello.rs
/path/to/toolchain/bin/lkj run examples/hello.rs
/path/to/toolchain/bin/lkj build examples/hello.rs -o hello
./hello
/path/to/toolchain/bin/lkj eval probe.rs
```

`check` is silent on success; `run` preserves ordinary process streams and status;
`eval` also prints a scalar return; `build` publishes an ELF only after success.
`k` writes checked K, `pack` captures source/library inputs, `runtime-k` prepares
the declared source runtime, and `verify` checks installed hashes. See the
[native workflow](docs/native.md) for installation, explicit inputs and rebuilding
all compiler programs from an installed seed.

The bootstrap and development gate remain available:

```sh
cargo build --release
./target/release/lkjscript run examples/hello.rs --engine k
bash tools/check.sh
```

The bootstrap runner prints scalar results and provides `test`, `trusted` and
the independent K execution model. It remains useful for reference observations.
Native artifacts take execution extents from Linux at startup. Runner budgets
are explicit host controls; they do not restrict accepted source.

The runnable subset includes conditional initialization, multi-root local views,
per-leaf call origins, custom Drop, generics and traits, arrays, slices, Box and
Vec. Patterns, guards, question-mark, closures and value-producing breaks lower
through ordinary K operations. The source library provides text, iterators,
formatting, ordered maps and collection algorithms. Heap containers own their
contents; stored relationships use indices or owned values. World-derived
effects are explicit: owned byte arguments, binary standard streams, requested
exit status and files beneath directory capabilities. File handles close on
structural destruction. Arguments after `--` are passed to the program unchanged.

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

The ordinary-source World driver in src/kc_main.rs shares those production
modules. Its native compiler checks direct K and emits ELF bytes on stdout. The
measured kc1, kc2 and kc3 are byte-identical; the self-built compiler matches
B0 on all 562 K verdicts and all 498 accepted artifacts in the recorded corpus.
Reproduce the K-compiler fixed point and native parity with:

```sh
node tools/selfhost.mjs
LKJ_KC_OUT=target/kc-native-observation node tools/kc.mjs \
    target/release/lkjscript target/selfhost-observation/kc2.elf --world
```

The source producer in src/frontend_main.rs shares the production parser,
resolver, elaborator and independent K checker through src/frontend.rs. It reads
explicit source/library directories or a prepared bundle and writes K. The same
loader owns module discovery in both cases. Both native generations reproduce
B0's exact own-source K and rebuild to an identical ELF through native kc.
The [source-producer interface](docs/frontend.md) documents bundle packing and
logical paths. After preparing the K compiler above, reproduce this fixed point:

```sh
node tools/frontend.mjs snapshot --source src/frontend_main.rs \
    target/release/lkjscript target/frontend-source-baseline
node tools/frontend-selfhost.mjs prepare target/frontend-source-baseline \
    target/selfhost-observation/kc2.elf target/selfhost-observation/runtime.k \
    target/frontend-selfhost
node tools/frontend-selfhost.mjs emit target/frontend-selfhost
node tools/frontend-selfhost.mjs run target/frontend-selfhost
```

These stages retain declared inputs, hashes, raw outputs, time and peak memory.
Choose fresh observation directories for a new run. The native front end and kc
then operate on declared directories or bundles and runtime K without invoking
the bootstrap. The installed workflow also provides native result decoding and
a complete native rebuild command.

[Status](docs/status.md) records current evidence and open obligations.
[Direction](docs/direction.md) defines the consumer-led route out of Rust;
[language](docs/language.md) describes the implemented contract and
[constraints](docs/limits.md) records representation and execution boundaries.

New semantic tests are source/K data. The gate compares native and K results
with Rust under the declared host, exercises generated programs and checks the
K-to-ELF boundary. Seeded direct K mutations independently compare accepted
programs on both engines and retain resource/timeouts separately. WebAssembly
and its Node execution host have been removed.
Node remains temporary test orchestration. Generation one is preserved on its
branch and is not a generation-two dependency.
