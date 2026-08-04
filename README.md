# lkjscript

`lkjscript` is an AI-primary, statically typed, memory-safe functional language
and daemon-first platform implemented in Rust. One resolved typed authority feeds
HIR, verified SSA, validated bytecode, the evaluator, the reference VM, and the
native JIT tiers.

## Status
<!-- LKJ-F documentation-authority accepted-contract 81b82OE4Zojd_1uy8ObShOosGjncoi--r2Ksh3kp3aE -->
<!-- LKJ-F jit-proof-forced current oSQRIl0aIaw-bZriT4e0HhvsUOLfOZyjRvvTcuYCFmo -->
<!-- LKJ-F no-tracing-runtime current G3iFIfhkJeXDVrUevwng0sXnqvrZGdJwsIUz1qcii1Q -->
<!-- LKJ-F os-resident-runtime-foundation current tv7TmvPjmBID87VD91R-ewgRyhsyYnqTN6Hirq-t9VY -->


Canonical source uses **`.lkjscript`** only. Removed suffixes, language markers,
editions, and compatibility modes have no aliases. Linux x86-64 is the Current
native acceptance platform.

The Current Linux platform runs a foreground `lkjscriptd` coordinator with
bounded authenticated local control and isolated application cells. Standalone
execution remains a bounded bootstrap, recovery, diagnosis, CI, and development
path rather than a second platform authority.

Supported slices execute through an independent evaluator, validated reference
VM, callable baseline JIT, and forced proof-checked JIT. Forced native execution
preflights complete support and never falls back to the VM.

The runtime is collector-free. No tracing heap, liveness root map, collection
service, collector barrier, collector configuration, collector metric, or
tracing fallback remains in production.

Exact Current capability and explicit gaps are in
[Current State](docs/current-state.md). Accepted future work is not implied by
this entry point.

## Development Commands

```sh
cargo run --locked -p lkjscript-app --bin lkjscript -- \
  run --engine vm src/examples/hello/main.lkjscript
cargo run --locked -p lkjscript-app --bin lkjscript -- \
  run --engine baseline-jit src/examples/jit-scalar/main.lkjscript
cargo run --locked -p lkjscript-app --bin lkjscript -- \
  run --engine optimizing-jit src/examples/jit-scalar/main.lkjscript
cargo run --locked -p lkjscript-app --bin lkjscript -- describe --json
cargo run --locked -p lkjscript-app --bin lkjscript -- memory inventory --json
cargo run --locked -p lkjscript-xtask -- quiet verify
```

Docker verification, when Docker is available:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

## Engineering Authority

- [Current State](docs/current-state.md)
- [Architecture](docs/operations/architecture.md)
- [Language](docs/language/README.md)
- [Verification](docs/operations/verification.md)
- [Agent Guide](AGENTS.md)
