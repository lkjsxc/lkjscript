# Current State: Semantic Source Foundation V1 Evidence

[Authority](../../current-state.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

the source authority but not Edition 1 runtime semantics.

| Command or check | Result |
| --- | --- |
| focused source-foundation tests | passed; 30 focused tests cover exact 113-file byte roundtrip, structural idempotence, UTF-8 spans, exact-input revision aliases, stale nodes, framed/adversarial keys, same/cross-file duplicates, strict public paths, 1,500-unit iterative import loading, 1,500-level iterative source-tree traversal, cycles, symlink/descriptor containment, non-regular/non-UTF-8 rejection, all-entry width, and checked file/closure/unit/tree budgets | <!-- LKJ-EXACT-DATA -->
| independent architect/adversarial/verification/AI-usability reviews | the first candidate was blocked for recursive import/tree stack overflow, normalized-revision aliases, unframed keys, public path aliases, unbounded reads, descriptor TOCTOU, lossy host paths, duplicate logical origins, and unbounded directory collection; every blocker received a focused repair and regression witness before acceptance | <!-- LKJ-EXACT-DATA -->
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed on the final tree; strict formatting/Clippy/docs/tree/source checks and all workspace tests passed, including 74 compiler tests | <!-- LKJ-EXACT-DATA -->
| `cargo build --workspace --release --locked` and default/VM/forced-baseline/threshold-2-auto scalar, forced optimizing, VM hello/Mandelbrot, Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed on the final tree; forced engine policy and normal streams remained unchanged | <!-- LKJ-EXACT-DATA -->
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | exited 0 in 103 s with `result=ok`; rebuilt release output and reran the canonical gate plus configured runtime smokes | <!-- LKJ-EXACT-DATA -->
| `python3 meta/results/ai-authoring/validate.py meta/results/ai-authoring/results/*.json`; `git diff --check` | passed; retained one strong raw-text pass, one medium isolation failure, and one weaker-provider timeout without deleting negative evidence | <!-- LKJ-EXACT-DATA -->
| Not tested | performance sampling, semantic transaction/entity/hole benchmark variants, full Brainfuck Mandelbrot, Miri, sanitizers, fuzzers, non-Linux host loading, AArch64, or Wasm/components | <!-- LKJ-EXACT-DATA -->

Two local verification attempts failed before the final pass: one found only
required rustfmt changes, and the next found legacy duplicate-diagnostic wording
regressions after moving duplicate rejection earlier. Both were corrected and
rerun. One malformed focused Cargo command supplied two test filters and was
rejected by Cargo; the two commands were rerun separately. No failed product
command is reported as a pass.
## SQLite Evidence

The SQLite implementation tree was verified on Linux x86-64 with the system
`libsqlite3.so.0` using:
