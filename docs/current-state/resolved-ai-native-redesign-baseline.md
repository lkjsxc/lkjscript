# Current State: Resolved AI-Native Redesign Baseline

[Authority](../current-state.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Resolved AI-Native Redesign Baseline

The redesign started from clean `main` at
`f6410a22bdcf8e516e5f2a771428f74edf4fcfa1`, one commit after the observed
forced-optimizer evidence commit `ccf53f1937951fc2535b2359fe704c66be0c1010`.
That additional commit selected synchronous automatic proof promotion but did
not implement it. The baseline host was Ubuntu 24.04.4 LTS, Linux
7.0.0-27-generic x86-64, Rust `1.96.0 (ac68faa20 2026-05-25)`, Cargo `1.96.0
(30a34c682 2026-05-25)`, with 96 GiB free on the 512 GiB artifact filesystem.
The workspace had nine Rust packages, zero third-party Cargo packages, and a
clean tree tracking `origin/main`.

Deterministic SHA-256 manifests hash lines of `<file SHA-256><two spaces><tracked
path><LF>` in byte-sorted tracked-path order:

| Baseline set | Files | Bytes | Manifest SHA-256 |
| --- | ---: | ---: | --- |
| canonical `src` `.lkjscript` corpus | 109 | 67,520 | `84fbbac1ba744ed9376f8e98dcf3389d2d914fd2b22d88fcda39dcda022c87d3` | <!-- LKJ-EXACT-DATA -->
| all tracked `.lkjscript` sources/fixtures/workloads | 113 | 69,351 | `78d8469b697c4b6672f28bf47cfb7f96373151ef42c24b85321e6e571df3b737` | <!-- LKJ-EXACT-DATA -->
| `meta/bench`, `meta/benchmarks`, and `meta/scripts` evidence | 31 | 2,814,103 | `4779093114cfe083bd35a3331f5df502687bcf2a14cdc390b9e5a4816ce81af3` | <!-- LKJ-EXACT-DATA -->
| `AGENTS.md` and tracked `docs` | 57 | 457,145 | `36a81144ed96508764f1ce435713ee4f0a86d33a942d6dc30865036e23ba9e5e` |
| complete tracked tree | 287 | 5,050,416 | `1c07d270a667373edac03d5dd224e0deae8f8b2b3b7e0099624547f3b0b0ae34` |

The immutable Brainfuck reference source hash was
`af6250f93ef18b35e35788958e6c1feed1a20155011e7208546940661dbedf1d`;
the benchmark driver hash was
`bd9d6e7f237834592941d53fde484a66cf99c3f240b22c2141003176e92bb220`.

Baseline commands actually run on that clean commit:

| Command/gate | Result |
| --- | --- |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | exited 0 in 7 s; canonical format/Clippy/docs/tree/source and workspace tests passed | <!-- LKJ-EXACT-DATA -->
| `cargo build --workspace --release --locked` | exited 0 in 15 s |
| default/VM/forced-baseline/threshold-2-auto scalar, forced optimizing, VM hello, and VM Mandelbrot runs | all exited 0; hello was `3628800`; Mandelbrot retained its canonical output | <!-- LKJ-EXACT-DATA -->
| `python3 meta/benchmarks/brainfuck/benchmark.py --mode smoke --no-build` | exited 0; direct/run-folded correctness and failure checks passed | <!-- LKJ-EXACT-DATA -->
| lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smoke scripts with the release binary | all exited 0 |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | exited 0 in 29 s with `result=ok` | <!-- LKJ-EXACT-DATA -->

No retained performance sampling, full Brainfuck Mandelbrot, Miri, sanitizer,
fuzzer, non-Linux target, AArch64, or Wasm/component acceptance was run for this
baseline. The baseline runs establish current health; they do not establish any
new redesign Target as Current.
