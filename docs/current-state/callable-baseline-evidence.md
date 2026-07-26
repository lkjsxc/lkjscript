# Current State: Callable Baseline Evidence

[Authority](../current-state.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

Every sample and phase distribution is retained at
`meta/benchmarks/jit/results/callable-baseline-jit-linux-x86_64.json`,
`auto-threshold-1.json`, `auto-threshold-1024.json`, and
`pre-jit-c4-vm-comparison.json`. Temporary `c4c9609` worktree, copied binary,
and source copy were removed; the compatible source itself is retained under
`meta/benchmarks/jit/pre-jit-workload/`. Profiling/disassembly improvement was
not required because the 5x target passed. Docker, full Brainfuck Mandelbrot,
OSR, non-scalar native semantics, and non-Linux acceptance were not run.

Final-worktree inventory was recalculated rather than copied from older
records: 96 canonical `src/**/*.lkjscript` files (58,734 bytes, 8,067 physical
lines) are covered by ten executable roots; two additional compatible benchmark
sources live under `meta/benchmarks/jit/pre-jit-workload` and are not canonical
corpus members; the canonical workspace gate reports 126 tests; and `docs/`
contains 42 Markdown documents. The final release binary retains the
1,448,584-byte size and SHA-256 above. The four committed result JSON files are
293,337, 293,879, 293,535, and 29,965 bytes with the exact hashes recorded in
Experiment C4.

Final acceptance ran `cargo run --locked -q -p lkjscript-xtask -- quiet verify`
(126 tests), `cargo build --workspace --release --locked`, ordinary/default,
explicit VM, forced, and threshold-2 auto scalar runs, explicit-VM hello and
Mandelbrot, `python3 meta/benchmarks/brainfuck/benchmark.py --mode smoke
--no-build`, and the lkjedit/HTTP smoke scripts. All passed; scalar streams were
empty, hello was exactly `3628800`, and Mandelbrot remained 1,176 bytes/24 lines
with SHA-256
`222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907`.
The exact final implementation-tree command
`docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify`
passed with `result=ok`; the image reran the canonical gate and release hello,
Mandelbrot, lkjedit, and HTTP boundaries. Separate final commands for rustfmt,
strict workspace Clippy, docs/tree/source checks, locked release build, and
`git diff --check` also exited 0. Full Brainfuck Mandelbrot was not run. The
first aggregate smoke wrapper itself exited 1 only because its extra local assertion incorrectly
expected a newline after the canonical newline-free hello output; every wrapped
command had exited 0. The corrected complete wrapper was rerun and exited 0,
so no failed product command is hidden.
## Accepted Next Target

Semantic Source foundation now completes the parser/load/identity/formatter
cutover and removes the duplicate legacy authority. The next dependency-ordered
