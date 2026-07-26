# Current State: Forced Enum JIT Evidence

[Authority](../current-state.md)

## Status

**Current** only for monomorphic host-independent enum construction, variant
checks, and verified active-field projection in forced Linux x86-64 baseline
and proof JIT. Source match, polymorphic entries/substitutions, host operations,
automatic reference transfer, and non-Linux native execution are not Current.

## Evidence

The uncommitted implementation worktree based on clean HEAD
`9b1da3c5f6d8c424d969613c8f0d1fb7707aa61c` was checked on Linux
7.0.0-27-generic x86-64 with Rust/Cargo 1.96.0.

- Focused core/IR/native/sys/JIT/VM/app tests passed. They cover proof-preserved
  enum metadata/operations, descriptor exact/+1 limits, malformed layout/tag/
  field rejection, nullary/payload/generic/nested values, active projection,
  inactive pre-access rejection, full enum heap preflight, forced GC roots/maps,
  nonzero generated entries/runtime calls, and zero fallback.
- `cargo test --locked -p lkjscript-sys` passed, including all executable and
  native-reference tests.
- Strict focused Clippy for core/IR/native/sys/JIT/VM/app, all targets/features,
  passed with `-D warnings`.
- Separate `check-docs`, `check-tree`, `check-sources`, and `structure check`
  commands passed.
- `cargo run --locked -p lkjscript-xtask -- quiet verify` passed after the final
  enum reservation change. Formatting, strict workspace Clippy, documentation,
  tree/source/structure gates, all configured tests, and doctests passed.
- `cargo build --locked --workspace --release` passed after the final enum
  reservation change.
- Not tested: Docker, performance sampling, automatic proof promotion, source
  match, host enum operations, native/VM reference transitions, Miri,
  sanitizers, or non-Linux targets.
