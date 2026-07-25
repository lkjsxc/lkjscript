# Current State: Ownership Through Native-Foundation Evidence

[Authority](../current-state.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

| Ownership correction command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-compiler -p lkjscript-ir -p lkjscript-core -p lkjscript-vm -p lkjscript-app` | passed; source/HIR/SSA malformed boundaries plus evaluator/reference-VM equivalence and existing scalar JIT app gates | <!-- LKJ-EXACT-DATA -->
| `cargo clippy --locked -p lkjscript-ir -p lkjscript-compiler -p lkjscript-jit -p lkjscript-app --all-targets --all-features -- -D warnings` | passed | <!-- LKJ-EXACT-DATA -->
| separate `check-docs`, `check-tree`, and `check-sources` | passed; canonical language sources were not modified |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; formatting, strict workspace Clippy, docs/tree/source closure, and all 168 workspace tests | <!-- LKJ-EXACT-DATA -->
| `cargo build --workspace --release --locked`; default hello, forced scalar JIT, Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed; Brainfuck source remained unchanged | <!-- LKJ-EXACT-DATA -->
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed with `result=ok` and all configured smokes | <!-- LKJ-EXACT-DATA -->
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | performance, full Brainfuck Mandelbrot, Miri, sanitizers, or non-Linux targets |

The marker-trait implementation tree based on `5c6ba38` was checked on Linux
7.0.0-27-generic x86-64 with Rust/Cargo 1.96.0:

| Marker-trait command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-compiler -p lkjscript-ir -p lkjscript-app` | passed; declaration/coherence/bound solving, structural auto traits, malformed SSA witnesses, and evaluator/VM marker-call equivalence | <!-- LKJ-EXACT-DATA -->
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --locked -p lkjscript-ir -p lkjscript-compiler -p lkjscript-jit -p lkjscript-app --all-targets --all-features -- -D warnings` | passed | <!-- LKJ-EXACT-DATA -->
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs/tree/source closure, rustfmt, strict workspace Clippy, and all 151 workspace tests | <!-- LKJ-EXACT-DATA -->
| `cargo build --workspace --release --locked` plus Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed | <!-- LKJ-EXACT-DATA -->
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed with `result=ok`; rebuilt release runtime and reran the configured gates/smokes | <!-- LKJ-EXACT-DATA -->
| Not tested | full Brainfuck Mandelbrot, performance, Miri, sanitizers, or non-Linux targets |

This evidence establishes only marker declarations, exact nominal impls,
generic marker bounds, bounded structural Copy/Send/Sync solving, and verified
erased witness identity. It does not establish trait methods, associated items,
ownership, package coherence/orphan rules, dynamic dispatch, specialization, or
native generic monomorphization.

The lossless bulk-byte and durable-file changes in this documentation's
containing commits were checked on Linux x86-64 with Rust/Cargo 1.96.0:

| Command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-core -p lkjscript-compiler -p lkjscript-sys -p lkjscript-vm` | passed; focused compiler/core/sys/VM coverage including exact binary socket transfer | <!-- LKJ-EXACT-DATA -->
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; workspace check, docs/tree/source closure, rustfmt, strict Clippy, and all workspace tests | <!-- LKJ-EXACT-DATA -->
| `cargo build --workspace --release --locked`; bulk-byte, durable-file, and HTTP smokes | passed; exact `.lkjscript` file-buffer plus append/replay consumers and legacy HTTP behavior | <!-- LKJ-EXACT-DATA -->
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed; Docker source closure and all configured runtime smokes including bulk bytes and durable files | <!-- LKJ-EXACT-DATA -->
| Not tested | performance and application-level HTTP/storage workloads |

Phase A implementation commit
`12836da90d886c9e741a5ac9f8148a17d00f0505` and the state-threaded editor
behavior follow-up `91d7e9bb734307269eb44b2d3a0882ba55d2f5b2`, based on `e4c1d0e`, were
checked on Linux x86-64 with Rust/Cargo 1.96.0. Evidence is command-specific; Docker, full Brainfuck
Mandelbrot, and performance are not implied.

| Command or check | Result |
| --- | --- |
| `cargo check --workspace --all-targets --locked` | passed |
| focused `lkjscript-compiler` and app HIR/numeric tests | passed; 37 compiler and 10 app integration tests |
| `cargo run --locked -p lkjscript-xtask --quiet -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 82 workspace tests | <!-- LKJ-EXACT-DATA -->
| `check-sources` | passed for all 94 `.lkjscript` sources; the nine compiled executable closures equal the corpus exactly | <!-- LKJ-EXACT-DATA -->
| HIR/local mutation conformance | explicit Main/Function, missing/duplicate/imported main, declaration-only imports, rejected top-level effects/value defs, stable BindingId/local-slot shadowing, initializer scope, local-only set rejection and exact typing, same-function isolation, ProductId/field resolution, and StoreLocal execution passed | <!-- LKJ-EXACT-DATA -->
| `cargo build --workspace --release --locked` | passed |
| canonical hello | passed; output `3628800` |
| Mandelbrot | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` | <!-- LKJ-EXACT-DATA -->
| Brainfuck smoke | direct and run-folded correctness/failure boundaries passed |
| lkjedit smoke | passed; existing-file insert/save/reopen, missing-file creation, CRLF redraw, and command paint |
| one-shot HTTP smoke | passed |
| validated-chunk boundaries | centralized decode/CFG/metadata validation and random raw-chunk no-panic tests passed after integration | <!-- LKJ-EXACT-DATA -->
| structured execution boundaries | return/exit/trap/deadline and configured resource categories passed; returned heap values remain owned after VM teardown | <!-- LKJ-EXACT-DATA -->
| native-backend decision spike | 8 randomized warmups plus 31 retained pairs; exact generated calls passed; owned execution median/MAD 48.406374/0.540016 ms versus Cranelift 0.134.2 119.422902/0.566505 ms; temporary artifacts removed; no production backend implemented | <!-- LKJ-EXACT-DATA -->
| Phase A `check-docs` and `git diff --check` | passed |

Phase B fixed-point effect inference in this documentation's containing commit,
based on `061f7c51c74412fcb19cd43df8385ac692a26367`, was checked on Linux x86-64
with Rust/Cargo 1.96.0. Only effect inference and its HIR facts changed; typed
SSA, native code, runtime JIT, runtime smokes, Docker, and performance were not
tested or implemented.

| Phase B command or check | Result |
| --- | --- |
| `cargo test --locked -q -p lkjscript-compiler` | passed; 44 compiler tests |
| `cargo check --workspace --all-targets --locked` | passed |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | passed |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 101 workspace tests | <!-- LKJ-EXACT-DATA -->
| fixed-point effect conformance | passed; pure leaf, direct/transitive propagation, direct and mutual recursion, recursive effects, allocation, memory read/write, local mutation, host IO, process exit, trap, declaration-order independence, generic canonical direct calls, retained argument effects, and conservative indirect calls | <!-- LKJ-EXACT-DATA -->

Phase C typed-SSA/reference-bytecode contract commit `787d7b1` and
implementation commits `41deaef`, `0c9903b`, `d9a6917`, `47c3b83`, and
`1b7b1ce`, based on
`ec2afbb1161eff437370d1e75c9522af9a261342`, were checked on Linux x86-64 with
Rust/Cargo 1.96.0. This evidence establishes typed SSA and the reference
cutover, not native execution, JIT tiering, OSR, Docker, or performance.

| Phase C command or check | Result |
| --- | --- |
| focused crate tests | passed; 6 `lkjscript-ir`, 44 compiler, 14 core, 31 VM, and 14 app tests |
| SSA differential conformance | passed; exact focused Unit/Bool/I64/F64/control/loops/calls/recursion/local mutation/products/Option/Result/buffers/traps/exits, explicit unsupported host operations, tail-call bytecode shape, and 64 deterministic bounded randomized typed scalar programs | <!-- LKJ-EXACT-DATA -->
| malformed SSA and pass conformance | passed; direct malformed identity/use/dominance/edge/loop/effect cases, each isolated pass, repeated determinism, post-pass verification, combined normalization, and evaluator bounds | <!-- LKJ-EXACT-DATA -->
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | passed |
| `check-docs`, `check-tree`, and `check-sources` | passed; all nine executable closures cover all 94 canonical sources through SSA and validated bytecode | <!-- LKJ-EXACT-DATA -->
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 112 workspace tests | <!-- LKJ-EXACT-DATA -->
| `cargo build --workspace --release --locked` | passed |
| canonical hello | passed; output `3628800` |
| Mandelbrot | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` | <!-- LKJ-EXACT-DATA -->
| Brainfuck smoke | passed after preserving return-adjacent tail calls and liveness-allocated typed bytecode locals; direct and run-folded correctness/failure checks passed; full Brainfuck Mandelbrot was not run | <!-- LKJ-EXACT-DATA -->
| lkjedit and one-shot HTTP smoke | passed |

At historical native-foundation commit `ec2afbb`, six focused native/sys unit
and integration tests, strict Clippy, the then-current 106-test canonical gate,
and generated-code invocation covered multi-block control, a 100-iteration
loop, direct native calls, an allowlisted runtime call, exact I64 traps, F64
bits/comparisons, structured exit, W^X permissions, limits, and 32 repeated
install/invoke/drop cycles. That commit did not connect source or verified SSA
to native code and was not JIT acceptance evidence. Later callable-baseline
records establish the Current verified-SSA-to-generated-entry boundary; this
historical limitation must not be restated as the current implementation.

Earlier decision-grade and diagnostic performance records remain in
[Experiment Registry](../vision/experiments.md); they were not rerun for Phase A,
Phase B, or Phase C. A gate that did not run did not pass. Docker, full
Brainfuck Mandelbrot, source-to-native execution, and performance were not
tested for Phase C.

The callable scalar baseline implementation chain through
`a9d0584ad0106817c4eac5de7dbc9191e7537105`, based on current-main
`c4c96094260072323f9399fe7f0f7b4a14d1eef6`, was checked in isolated worktree
`/tmp/pi-agent-a98a8be7-b37a-422-f33e779d` on Linux
`7.0.0-27-generic` x86-64 with Rust/Cargo 1.96.0. The evidence establishes the
exact allocation-free scalar subset, not full-language native execution, OSR,
or a performance result.

| Callable baseline command or check | Result |
| --- | --- |
| focused IR/compiler/native/sys/JIT/VM/app tests | passed; the final canonical workspace gate reports 125 tests, including 7 source-engine and 1 direct verified-SSA JIT tests | <!-- LKJ-EXACT-DATA -->
| strict workspace Clippy, all targets/features | passed with `-D warnings` |
| `check-docs`, `check-tree`, `check-sources` | passed; ten roots exactly cover all 96 canonical sources |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs/tree/source closure, rustfmt, strict Clippy, and all 125 tests | <!-- LKJ-EXACT-DATA -->
| `cargo build --workspace --release --locked` | passed in the shared target tree |
| scalar workload, explicit `vm` / `baseline-jit` / threshold-2 `auto` | all exited 0 with empty stdout and exact test-oracle F64 bits | <!-- LKJ-EXACT-DATA -->
| forced scalar diagnostics | one installed W^X object; compiled `scalar-step` and `main`; 100,001 native entries, 100,000 direct native calls, 300,002 PollV1 calls, zero VM fallbacks/failures | <!-- LKJ-EXACT-DATA -->
| auto scalar diagnostics | 99,998 later-call native entries, 99,998 PollV1 calls, exactly two initial VM calls, zero compile failures; no OSR claim | <!-- LKJ-EXACT-DATA -->
| explicit VM and threshold-2 auto hello | both output `3628800`; auto recorded 15 native leaf entries and one retry-suppressed recursive-group failure | <!-- LKJ-EXACT-DATA -->
| direct Mandelbrot in VM | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` | <!-- LKJ-EXACT-DATA -->
| Brainfuck smoke only | passed direct/run-folded correctness and failure checks; full Brainfuck Mandelbrot was not run | <!-- LKJ-EXACT-DATA -->
| lkjedit and one-shot HTTP smokes | passed |
| opt-in generated binary plus external `objdump` | passed; 1,926-byte source-derived object dumped, disassembled, then removed; normal stdout stayed empty | <!-- LKJ-EXACT-DATA -->

Docker, full Brainfuck Mandelbrot, performance sampling, OSR, background work,
optimizing/speculative tiers, native references/allocation/host IO, and
non-Linux/non-x86-64 acceptance were not run or implemented in that callable
implementation chain.

The retained measurement/default commit
`025cbb2feadbb18fbae51e68e38b9c849798d068`, following instrumentation/default
commit `56535c589998eeefa045fca622720662a2f78662`, was measured from a clean
isolated worktree on Linux 7.0.0-27-generic x86-64, AMD Ryzen 9 9955HX with 20
logical CPUs available, 32 GiB RAM, Rust/Cargo 1.96.0, and Python 3.12.3. The
release binary was 1,448,584 bytes with SHA-256
`94dec3b623f07333ed57659c67d8461c8ac30e7c13684f147700b72cefd9a638`;
the 289-byte workload SHA-256 was
`aa8acecbad8add81f7a3a79b19a69e8f503d36c8af6e1f503b572bfadd14157e`.

| Retained scalar metric | Result |
| --- | --- |
| protocol/oracle | four warmups and 31 randomized samples per variant, seed `0x4c4b4a534d455452`, no removed samples; every process returned exact F64 bits `0x401af3ef5a48f5f0` with zero stdout and no unexpected stderr | <!-- LKJ-EXACT-DATA -->
| process wall median / MAD / p95 / min / max | VM 354.533038 / 4.711766 / 362.572659 / 347.360647 / 369.390164 ms; forced 9.372036 / 0.467328 / 10.364211 / 8.711153 / 10.472645 ms; auto-64 214.482019 / 3.352331 / 226.691819 / 206.949992 / 228.798658 ms | <!-- LKJ-EXACT-DATA -->
| generated execution | forced native median 7.647935 ms versus VM execution 352.918413 ms: **46.146x**, meeting the aspirational 5x target | <!-- LKJ-EXACT-DATA -->
| compile/install/entry | native lowering+encoding 0.040096 ms, relocation/W^X install 0.036558 ms, 0.076654 ms combined; forced time to first native entry 0.080141 ms and first-call duration 7.647935 ms; measured whole-workload break-even one invocation | <!-- LKJ-EXACT-DATA -->
| auto-64 | 1.653x process-wall speedup over VM; median time to first native entry 0.297720 ms; 64 expected initial VM entries, 99,936 native entries/PollV1 calls, zero compile failures; main remained VM and no OSR is claimed | <!-- LKJ-EXACT-DATA -->
| forced counts/cache | 100,001 native entries, 100,000 direct calls, 300,002 PollV1 calls, zero fallback/failure; one object, 1,926 code bytes, 2,618 metadata bytes, 4,096 accounted allocation bytes | <!-- LKJ-EXACT-DATA -->
| peak RSS median | VM 2,736 KiB; forced 2,724 KiB; auto 2,808 KiB, polled from `/proc` |
| threshold decision | auto process medians at thresholds 1/64/1,024 were 211.286082 / 214.482019 / 211.901028 ms with overlapping dispersion; 64 is retained as the middle conservative policy, keeping 63 cold calls in VM while avoiding the 1,024-entry trigger delay | <!-- LKJ-EXACT-DATA -->
| pre-JIT VM diagnostic | compatible exact-oracle source: current VM 357.510855 ms versus `c4c9609` 364.419240 ms (0.981x); difference below twice larger MAD, so no regression/improvement claim; old/current binaries 1,129,440/1,448,584 bytes and median RSS 2,272/2,756 KiB | <!-- LKJ-EXACT-DATA -->
