# lkjscript, generation two

Read `docs/status.md`, inspect Git and live jobs, then implement the next action.
The language design comes first, reliable agent development second. Own the
compiler, runtime, tools and persistence, eventually without a Rust toolchain.

The compiler uses a Rust-compatible subset of lkjscript. Parse, elaborate to
explicit K, check once, lower and emit WebAssembly. Only checked K reaches a
backend. Keep the trusted modules small. Rust and Node are bootstrap bridges
and independent test oracles. Unsupported syntax must produce a diagnostic.

No arbitrary resource ceilings, work accounting or policy plumbing. Use growing
collections, explicit stacks and 64-bit extents. Record representation and engine
constraints in `docs/limits.md`. Keep checked arithmetic and ownership sound.

Work in this checkout only. Preserve the read-only `.generation-one` worktree
for specific ports. Do not modify running deployments or neighboring checkouts.
Use English, no Python, ordinary compiler words, and short command output.
Run `bash tools/check.sh` after coherent changes; keep it under a minute.
Commit small and push main directly. No force pushes, history rewrites, releases,
credential changes or protection changes. All routine decisions are delegated.

`docs/language.md` is the single specification. Status records working behavior,
broken or missing pieces, bootstrap, measurements and the next action. Git is
history; do not add decision records, session diaries or duplicate implementations.
