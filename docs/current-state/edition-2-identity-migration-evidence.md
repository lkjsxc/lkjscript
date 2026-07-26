# Edition 2 Identity And Migration Slice Evidence

[Authority](../decisions/semantics/edition-2/identity-and-migration.md)

## Status

**Current evidence** for the bounded Edition 2 identity and non-publishing
migration compiler API. This does not evidence Edition 2 ADTs, matches, changed
execution semantics, semantic publication, corpus migration, or cutover.

## Environment

- Base commit: `1606cf55f9b766547772151d2443b80c23363118` with the implementation left uncommitted.
- Recorded: `2026-07-26T00:51:24Z`.
- Host: `Linux 7.0.0-27-generic x86_64`.
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`.
- Cargo: `cargo 1.96.0 (30a34c682 2026-05-25)`.

## Focused Evidence

- `cargo test --locked -p lkjscript-compiler source::tests`: 46 passed; marker,
  trivia, malformed/missing/order/duplicate, mixed closure, identity, format,
  migration, stale/idempotent/non-publishing, and exact/+1 Profile V2 tests.
- `cargo test --locked -p lkjscript-compiler semantic::tests`: 29 passed; Schema V2
  edition/source/tree identity, strict marker projection, subtree rejection,
  and existing protocol tests.
- `cargo test --locked -p lkjscript-compiler`: 134 passed.
- `cargo test --locked -p lkjscript-app`: 35 passed across unit and integration targets.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`:
  passed.
- `cargo run --locked -p lkjscript-xtask -- check-docs`: passed.
- `cargo run --locked -p lkjscript-xtask -- structure check`: passed with every
  new path present in the Git index as intent-to-add.
- `cargo run --locked -p lkjscript-xtask -- quiet verify`: passed formatting,
  Clippy, docs/tree/source, and workspace tests.
- `git diff --check`: passed.

## Explicit Boundaries

The compiler accepts Edition 2 only for the exact edition marker plus already
implemented declarations and execution semantics. `enum`, constructors,
patterns, `match`, changed numeric rules, and their HIR/SSA/runtime work are
absent. The migration API returns replacement sources and identities but has no
filesystem write, semantic endpoint, or publish mode. The tracked Edition 1
corpus was not migrated. Runtime smoke, release, Docker, Miri, sanitizers,
fuzzing, non-Linux, AArch64, and Wasm gates were not run for this slice.
