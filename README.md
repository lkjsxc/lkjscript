# lkjscript

`lkjscript` is an experimental, AI-primary, statically typed, memory-safe language and
runtime implemented in Rust. The project is undergoing an architectural reset toward a typed
semantic program model, scalable compiler algorithms, and one coherent production runtime path.

Today, programs are imported from the provisional line-oriented `.lkjscript` projection. The
compiler resolves and checks them, builds typed HIR and verified SSA, emits validated bytecode,
and runs them through the default automatic VM/native path. Linux x86-64 is the currently tested
native platform.

## Build and run

A current Rust toolchain and SQLite runtime library are required.

```sh
cargo build --locked -p lkjscript-app
cargo run --locked -p lkjscript-app --bin lkjscript -- \
  run src/examples/hello/main.lkjscript
```

The example prints `3628800`, the factorial of 10.

The text projection is intentionally verbose and is not the permanent source schema. A small
ordinary function currently looks like this:

```text
def/
name/
dec
/name
public
fn/
sig/
inputs/
i64
/inputs
output/
i64
/output
/sig
params/
n
i64
/params
subtract/
n
1
/subtract
/fn
/def
```

See [`src/examples/hello/dec.lkjscript`](src/examples/hello/dec.lkjscript) for the executable
source.

## Verification

The reset uses transparent commands rather than the removed governance wrapper:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo build --workspace --release --locked
```

Docker also runs these commands and the retained application smoke tests:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

## Active documentation

- [Current implementation](docs/current.md)
- [Architecture and trust boundaries](docs/architecture.md)
- [Current language semantics](docs/language.md)
- [Semantic source-model direction](docs/source-model.md)
- [Ordered roadmap](docs/roadmap.md)
- [Engineering policy](AGENTS.md)
