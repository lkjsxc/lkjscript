# lkjscript

`lkjscript` is a source-free, semantic-graph-first programming system. An agent sends typed
queries and revision-checked transactions to `lkjscriptd`; the daemon owns durable Semantic
Program Graph snapshots and compiles complete entry functions directly to one verified Core IR.
The current runtime is the Core IR interpreter.

There is no language parser, source file, syntax tree, HIR, bytecode VM, JIT, native tier, or
compatibility reader in the active product. A `.lkjscript` file is a canonical semantic snapshot
artifact.

## Build and verify

```sh
cargo build --workspace --locked
cargo test --workspace --all-targets --all-features --locked
```

Linux x86-64 with a current stable Rust toolchain is the supported bootstrap host.

## Run the source-free scalar vertical

Start the daemon with an explicit private state directory:

```sh
mkdir -p /tmp/lkjscript-state
cargo run --locked --bin lkjscriptd -- --state /tmp/lkjscript-state --foreground
```

In another terminal, create a workspace and submit the typed aggregate transaction that constructs
`40 + 2`:

```sh
cargo run --locked --bin lkjscript -- --state /tmp/lkjscript-state workspace-create
cargo run --locked --bin lkjscript -- --state /tmp/lkjscript-state bootstrap-42 WORKSPACE_ID
```

The transaction response maps local handle `3` to the stable function Node ID. Use that Node ID's
serial for inspection and execution (the bootstrap workspace normally assigns serial `4`):

```sh
cargo run --locked --bin lkjscript -- --state /tmp/lkjscript-state node WORKSPACE_ID 1 4 --expand
cargo run --locked --bin lkjscript -- --state /tmp/lkjscript-state run WORKSPACE_ID 1 4
cargo run --locked --bin lkjscript -- --state /tmp/lkjscript-state shutdown
```

The run reports typed result `i64=42`. Restarting the daemon with the same state directory reloads
the same revision and Node IDs.

## Current boundary

The bootstrap language implements `unit`, `bool`, and `i64` types plus `const_i64`, `const_bool`,
`add_i64`, typed expression holes, and `return`. It has no host effects, calls, structured control,
aggregates, generics, ownership-bearing values, native code, runtime cells, or public networking.
See [current status](docs/status.md) and the [evidence-gated roadmap](docs/roadmap.md).
