# Edition 2 Identity, Migration, And Cutover Evidence

[Authority](../semantics/edition/identity-and-migration.md)

## Status

**Current evidence** for exact Edition 2 identity, compiler-owned migration
check/diff/publish, atomic closure publication and recovery, the 125-file
canonical corpus migration, and ordinary-compilation cutover. Broader Accepted
Edition 2 surfaces retain their own status.

## Environment

- Base commit: `ef1b41ff140e46f790dfaaf113b25898a73588e8`; implementation left uncommitted.
- Recorded: `2026-07-26T09:14:20Z`.
- Host: `Linux 7.0.0-27-generic x86_64`.
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`.
- Cargo: `cargo 1.96.0 (30a34c682 2026-05-25)`.

## Migration And Cutover Evidence

- All 125 tracked `.lkjscript` files are exact Edition 2; 121 are under `src/`.
  Exact compiler resolution inserted `f64-from-i64-rounded` at five mixed
  Edition 1 numeric sites and nowhere else.
- Migration tests pin old/new revision, tree, source, declaration, and node
  identities and exact per-file/aggregate bytes. They cover deterministic and
  idempotent check/diff, full two-file atomic publication, stale and mixed
  closure rejection, partial-install rollback, prepared-journal crash recovery,
  conflict preservation, exact Profile V2 byte/node boundaries, and resolved
  old/new execution differential results.
- Cutover tests prove explicit source validation and migration still accept
  markerless Edition 1 while ordinary in-memory and path compilation reject it
  with `LKJ-SRC-EDITION-CUTOVER`. There is no edition CLI, path, package,
  neighbor, alias, or fallback inference.

## Commands Run

- `cargo test --locked -p lkjscript-compiler`: 170 passed; doc tests passed.
- `cargo test --locked -p lkjscript-app`: passed across all unit and integration targets.
- `cargo test --locked --workspace --all-targets`: passed.
- `cargo test --locked --workspace --all-targets --release`: passed.
- `cargo build --workspace --release --locked`: passed.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`: passed.
- `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --all-features --no-deps`: passed.
- `cargo run --locked -p lkjscript-xtask -- check-docs`: passed.
- `cargo run --locked -p lkjscript-xtask -- check-tree`: passed.
- `cargo run --locked -p lkjscript-xtask -- check-sources`: passed.
- `cargo run --locked -p lkjscript-xtask -- structure check`: passed.
- `cargo run --locked -p lkjscript-xtask -- quiet test`: passed.
- `cargo run --locked -p lkjscript-xtask -- quiet verify`: passed.
- Release runtime acceptance passed for auto, VM, baseline, optimizing, and
  thresholded-auto JIT commands; hello and Mandel VM; Brainfuck smoke; and the
  lkjedit, HTTP, bulk-bytes, durable-files, SHA-256, and SQLite smoke scripts.

## Explicit Boundaries

Docker, Miri, sanitizers, fuzzing, non-Linux, AArch64, and Wasm were not run.
Docker remains a separate packaging gate and is reported untested rather than
inferred from local acceptance.
