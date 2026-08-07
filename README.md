# lkjscript

`lkjscript` is an experimental, AI-primary, statically typed, memory-safe language and runtime
implemented in Rust. The project is in an architectural reset toward a typed semantic workspace,
scale-safe compilation, and one measured production execution path.

## Prerequisites

- a current stable Rust toolchain with Cargo, Rustfmt, and Clippy;
- the SQLite runtime library (`libsqlite3` on Linux);
- Linux x86-64 for the currently tested native path.

## Build and first run

```sh
cargo build --locked -p lkjscript-app
cargo run --locked -p lkjscript-app --bin lkjscript -- \
  run src/examples/hello/main.lkjscript
```

A successful run prints `3628800`. `run` has one product execution policy: it attempts one eligible
baseline-native group before effects, otherwise executes the unchanged validated program in the VM.
The VM has no JIT dependency or native-transition state. There is no engine, threshold, or forced
native CLI selection, and native entry is a commit point with no VM retry. The discarded
optimizing runtime and forced execution helpers are deleted. The SSA evaluator is available
only as the opt-in `lkjscript-ir/test-oracle` feature used by development and differential tests;
production dependencies do not enable it.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo build --workspace --release --locked
```

The retained container verification also runs application smoke tests:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

## Documentation

- [Authority and conflict rules](docs/authority.md)
- [Normative language semantics](docs/spec/language.md)
- [Semantic workspace contract](docs/spec/workspace.md)
- [Current implementation status](docs/status.md)
- [Current architecture and target deltas](docs/architecture.md)
- [Performance method and baselines](docs/performance.md)
- [Roadmap](docs/roadmap.md)
- [Engineering policy](AGENTS.md)
