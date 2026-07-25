# Current State: Optimizer And Allocation Evidence

[Authority](../current-state.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Evidence

The retained optimizing-JIT benchmark-only work at clean commit
`cc967ff7e6f57a3225ae974d64ced6039ed8e9ae` was run on Linux
7.0.0-27-generic x86-64, AMD Ryzen 9 9955HX, with Rust/Cargo 1.96.0. It changes
no engine policy and makes only a forced first-tier performance measurement;
automatic promotion remains disabled and unmeasured.

| Retained optimizing benchmark command or check | Result |
| --- | --- |
| `python3 meta/benchmarks/jit/benchmark.py` | clean locked workspace release build passed; exact silent reference-VM I64 `3333` oracle and all forced outcomes passed; all 4 warmups and 31 measured samples per case were retained in deterministic randomized interleaving with monotonic process wall and polled peak RSS |
| same-commit optimizing workload | baseline native median/MAD exactly 1,999,889/10,469 ns; optimizing 670,029/2,174 ns; 2.984780x speedup; 1,329,860 ns improvement versus 25,286 ns twice-combined-MAD threshold; process wall 3,565,363 to 2,411,023 ns, 1.478776x; 72 checked proof records, 2,424 optimizing versus 13,656 baseline code bytes, 10,001 optimizing entries, zero baseline entry/fallback, and verified W^X |
| retained scalar sentinel comparison | native median 7,982,586 ns versus retained 7,647,935 ns, ratio 1.043757: passed the 1.05 ceiling; process median 9,207,038 versus 9,372,036 ns, ratio 0.982395: passed. This cross-commit sentinel includes compiler/runtime evolution and does not attribute recovery to optimizing passes |
| allocation-graph correctness metrics, once | exact I64 `1`; 3 optimizing entries, 7 allocations, 6 collections, 14 attempted/14 successful heap calls, maximum 3 roots, 6 barriers, zero baseline entry/fallback, and W^X |
| mechanical verdict | **Adopted** for forced first proof-optimizing performance because every exact criterion passed; automatic promotion remains disabled and unmeasured, with no OSR/deoptimization/speculation claim |
| prior retained run | `063668e` remains **Rejected** in `optimizing-jit-linux-x86_64-rejected-scalar-regression.json`: optimizer-local 2.930761x passed, scalar native 8,182,742/7,647,935 ns = 1.069928 failed, and scalar process 9,340,049/9,372,036 ns = 0.996587 passed; its record truthfully lists dirty benchmark README/harness/cache paths. Folding the mandatory entry poll into frame registration removed one runtime transition before the clean adopted rerun without weakening polls or proofs |
| retained JSON validation; `cargo run --locked -p lkjscript-xtask -- check-docs`; `git diff --check` | passed; both JSON files parsed, commit/verdict/SHA-256 identities matched, all 10 adopted criteria were true, exact medians and 1.478776 process ratio matched, docs links passed, and the diff had no whitespace errors |
| Not tested | benchmark rerun, full canonical workspace verification, Docker, automatic optimizing promotion, OSR, deoptimization, speculation, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The final forced-optimizer hardening in this document's containing commit,
based on `114196422fb41b8c1b1dab6304c1680000cf67ed`, was checked in the
primary Linux 7.0.0-27-generic x86-64 checkout with Rust/Cargo 1.96.0. It closes
aggregate cleanup/preflight/pass-accounting and structured pre-entry evidence,
and replaces per-entry pthread stack queries with one invocation-bound query.

| Final forced-optimizer command or check | Result |
| --- | --- |
| focused IR/JIT/sys/app tests | passed; type-parameter-vector preflight, aggregate worst-case cleanup charging, unreachable-before-copy cleanup, validation-inclusive pass totals, and nonzero optimizing entry evidence for zero stack/frame structured outcomes plus prior proof/root/allocation coverage |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | passed |
| docs/tree/source checks and `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; rustfmt, strict Clippy, exact source closure, 213 unit/integration tests, and one non-Send compile-fail doctest |
| `cargo build --workspace --release --locked`; default hello, forced scalar/allocation/optimizing JIT, Mandelbrot, Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed; declared optimizer workload returned I64 `3333`, retained 72 checked-I64 proof records, emitted 2,724 optimizing bytes, entered optimizing code 10,001 times, and recorded zero baseline entries/fallback; allocation optimization returned I64 `1` with 3 optimizing entries and zero downgrade |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed with `result=ok`, 213 tests plus the compile-fail doctest, and all configured smokes |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | retained performance sampling, automatic promotion, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The adversarial proof-optimizer repair in this document's containing commit,
based on `1f9999854d91e3abc033c555bd465f8ce1be36c1`, was checked in an
isolated Linux 7.0.0-27-generic x86-64 worktree with Rust/Cargo 1.96.0 and 96
GiB free in the shared artifact filesystem.

| Adversarial proof-optimizer command or check | Result |
| --- | --- |
| focused IR and app optimizer/JIT/CLI tests | passed; independent forged-proof rejection, exact checked trap identity from source, public oversized-candidate/growth rejection, charged duplicate-expression width, unreachable diamond/loop cleanup, optimizing recursive live roots/maps, help, metrics fields, and retained prior optimizer/JIT coverage |
| `cargo test --locked --workspace` | passed; 213 unit/integration tests plus the non-Send compile-fail doctest |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`; `cargo fmt --all -- --check` | passed |
| separate `check-docs`, `check-tree`, and `check-sources`; `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; the canonical gate reran formatting, strict Clippy, docs/tree/source closure, all 213 tests, and the compile-fail doctest |
| `cargo build --locked --workspace --release`; forced release baseline scalar, optimizing scalar, baseline allocation graph, and optimizing allocation graph | passed; all four smokes exited zero with empty stdout/stderr and no forced downgrade |
| `git diff --check` | passed |
| Not tested | Docker, performance sampling, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The forced first proof-optimizing implementation in this document's containing
commit, based on `cd4eee2d9381decf98ef89f6dc9f8526cbea3aa8`, was checked in an
isolated Linux x86-64 worktree with Rust/Cargo 1.96.0. It makes only the forced
first pipeline Current; it does not select automatic promotion or establish the
1.20x aspirational performance gate.

| First proof-optimizing command or check | Result |
| --- | --- |
| `cargo test --locked --workspace` | passed; 209 unit/integration tests plus the non-Send compile-fail doctest, including deterministic certificates, same-block/dominator checked GVN, forged proof rejection, 64 randomized scalar differentials, evaluator/VM/baseline/optimizing exact outcomes, allocation graphs, traps/exits/deadline/fuel, unsupported/budget no-downgrade, W^X, and entry/tier facts |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | passed |
| separate `check-docs`, `check-tree`, `check-sources`; `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; canonical source closure includes the new optimizing workload and the gate reran formatting, strict Clippy, all 209 tests, and the compile-fail doctest |
| `cargo build --workspace --release --locked`; forced scalar baseline, allocation baseline, explicit-VM optimizing workload, forced optimizing scalar/allocation smokes | passed with silent normal streams; optimizing workload returned exact I64 `0`, installed one optimizing object, entered optimizing code 10,001 times, retained 4 records (3 algebraic, 1 GVN, 1 checked-I64 subset), emitted 2,788 versus baseline 3,405 code bytes, and had zero baseline entries/objects or VM fallback; optimizing allocation returned exact I64 `1` with 7 allocations, 6 collections, 14 attempted/14 successful heap calls, and zero downgrade |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | Docker, 1.20x performance sampling, automatic promotion, broader optimization passes, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The final allocation-baseline hardening in this document's containing commit,
based on `7942d4e0d57e863b9ffe071cf07dc3ad252c1e23`, was checked in the
primary Linux 7.0.0-27-generic x86-64 checkout with Rust/Cargo 1.96.0. It closes
remaining exact ABI, evaluator accounting, trap identity, stable-index, and
structural-layout boundaries without changing canonical language sources.

| Final allocation-hardening command or check | Result |
| --- | --- |
| focused core/IR/native/sys/JIT/VM/app tests | passed; exact heap-site ABI identity, incremental list equality and error propagation, evaluator buffer payload/wrapper allocation limits, full-u32 explicit trap sites, stable-handle ID exhaustion, and collision-free interned nested layouts plus prior allocation/native coverage |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | passed |
| docs/tree/source checks and `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; rustfmt, strict Clippy, exact source closure, 202 unit/integration tests, and one non-Send compile-fail doctest |
| `cargo build --workspace --release --locked`; default hello, forced scalar/allocation JIT, Mandelbrot, Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed; allocation graph returned I64 `1` with 3 native entries, 7 allocations, 6 collections, maximum 3 roots, 14 attempted/14 successful heap calls, 6 barriers, and zero fallback; Mandelbrot remained 1,176 bytes with SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed with `result=ok`, 202 tests plus the compile-fail doctest, and all configured smokes |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | performance sampling, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The adversarial allocation-baseline repair in this document's containing
commit, based on `3467137b3e2ad9cf15ff55cd4cf38a134126e373`, was checked in
an isolated worktree on Linux x86-64 with Rust/Cargo 1.96.0. It repairs the
Current host-independent slice; Handle/host calls, native/VM reference
transitions, the complete allocation-capable decision, and collection-pause
measurement remain outside this evidence.

| Adversarial repair command or check | Result |
| --- | --- |
| focused core/native/sys/JIT/VM/app tests | passed; auto reference-helper entry gating, non-reused same-layout stale handles, canonical malformed heap descriptors, moving-service argument re-materialization, buffer Result boundaries, MAX/MAX+1 list equality, selected callee trap identity, zero/tiny native active values, transactional mutation rollback/limits, reachable-only snapshots, and attempted/successful heap-call metrics plus retained prior coverage |
| strict workspace Clippy, all targets/features | passed with `-D warnings` |
| separate docs/tree/source checks and `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; formatting, strict Clippy, exact source closure, 193 unit/integration tests, and one compile-fail doctest |
| locked workspace release build; default/VM/forced/threshold-2-auto scalar, VM hello, forced allocation-graph metrics, and Brainfuck smoke | passed; scalar streams were empty, hello was exact `3628800`, allocation graph returned I64 `1` with 14 attempted/14 successful heap calls, 7 allocations, estimated-byte keys, and zero fallback, and Brainfuck direct/run-folded correctness/failure boundaries passed |
| metrics parser correction | the first local parser invocation failed because the metrics file intentionally begins with `LKJSCRIPT_METRICS `; the generated program had exited successfully. A corrected prefix-aware parser was run and passed |
| Not tested | Docker, performance sampling, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The host-independent source allocation/recursion slice in this document's
containing commit, based on `0daa7a0d3064ad487cee2154d91f9db0a0fc0c82`,
was checked in isolated worktree
`/tmp/pi-agent-d9f4b948-568f-497-2a12ad4f` on Linux 7.0.0-27-generic x86-64
with Rust/Cargo 1.96.0. Canonical Brainfuck source was unchanged.

| Source allocation/recursion command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-core -p lkjscript-native -p lkjscript-sys -p lkjscript-jit -p lkjscript-vm -p lkjscript-app` | passed; shared heap boundaries, malformed heap sites/classes/homes, generic three-argument frame-home dispatch, service trap/resource/host propagation, existing CollectReferenceV1 certificates, source forced collection through direct/mutual recursive live-reference frames, nested Product/Option/Result/List/Str/Buf evaluator/VM/native equality, tiny allocation/heap limits, ownership rejection, W^X and existing scalar gates |
| `cargo clippy --locked -p lkjscript-core -p lkjscript-native -p lkjscript-sys -p lkjscript-jit -p lkjscript-vm -p lkjscript-app --all-targets --all-features -- -D warnings` | passed |
| separate `check-docs`, `check-tree`, and `check-sources` | passed; canonical language sources, including Brainfuck, were unchanged |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; formatting, strict workspace Clippy, docs/tree/source closure, 182 unit/integration tests, and one compile-fail doctest |
| `cargo build --workspace --release --locked`; scalar default/VM/forced/threshold-2-auto, explicit-VM hello, Brainfuck smoke, and forced allocation-graph metrics smoke | passed; allocation graph returned exact I64 `1`, recorded 3 native entries, 7 allocations, 6 collections, maximum 3 roots, 14 successful heap calls, 6 barriers, zero fallback, and empty stdout |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | Docker, performance sampling, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

This evidence makes only the host-independent source allocation/recursion slice
Current. It does not establish the full allocation-capable target, an optimizing
tier, or OSR.

The exact native-root repair in this document's containing commit, based on
`cc7ad01c9365b659a8cf909c400788aadde4770a`, was checked in isolated worktree
`/tmp/pi-agent-0917730b-997b-416-8744f760` on Linux 7.0.0-27-generic x86-64
with Rust/Cargo 1.96.0. It establishes pre-touch guarded frame reservation,
verifier-certified root completeness with a private image check, bounded root
construction, exact runtime-service resource classification, and dynamic
shallow-root capacity. It does not establish source-level native allocation or
a shared VM/native heap.

| Exact native-root repair command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-native -p lkjscript-sys` | passed; verifier certificate/adversarial width, omitted-live-root corruption, 64 KiB thread stack rejection, zero-frame bound, configured byte limits, exact reservation release, runtime-service classification, and a valid shallow 1,025-root map plus existing native/sys coverage |
| `cargo clippy --locked -p lkjscript-native -p lkjscript-sys -p lkjscript-jit --all-targets --all-features -- -D warnings` | passed |
| `cargo run --locked -p lkjscript-xtask -- check-docs` | passed |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; formatting, strict workspace Clippy, docs/tree/source closure, 179 unit/integration tests, and one compile-fail doctest proving the Copy adapter token is non-Send |
| `cargo build --workspace --release --locked`; default hello, forced scalar JIT, Mandelbrot, Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed; Mandelbrot retained its exact 1,176-byte output and SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed with `result=ok`, 179 tests plus the compile-fail doctest, and all configured smokes |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | source-level native allocation, shared VM/native collection, performance, full Brainfuck Mandelbrot, Miri, sanitizers, or non-Linux targets |

The closed-machine-plan native-reference/active-frame implementation in this
document's containing commit, based on HEAD
`ec54cde9b93a302c1310d2107c10b785001f184d`, was checked on Linux
7.0.0-27-generic x86-64 with Rust/Cargo 1.96.0. It establishes ABI-2 typed
stable words, exact closed-plan Buf roots, generated active frames, and actual
safe-service collection; it does not establish source-level allocation or a
shared VM/native heap.

| Native-reference/frame command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-native -p lkjscript-sys -p lkjscript-jit -p lkjscript-vm` | passed; plan/image malformed boundaries, non-empty exact maps, generated collection with dead-root exclusion, caller/callee chains, structured epilogues, frame bounds, repeated W^X installation, and existing JIT/VM tests |
| `cargo clippy --locked -p lkjscript-native -p lkjscript-sys -p lkjscript-jit --all-targets --all-features -- -D warnings` | passed |
| separate `check-docs`, `check-tree`, and `check-sources` | passed; canonical language sources were unchanged |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; formatting, strict workspace Clippy, docs/tree/source closure, and all 175 workspace tests |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | source-level native allocation, shared VM/native collection, Docker/release smokes, performance, Miri, sanitizers, or non-Linux targets |

The ownership implementation tree based on main HEAD `c64b3ab` was checked on
Linux 7.0.0-27-generic x86-64 with Rust/Cargo 1.96.0. Canonical Brainfuck source
was unchanged.
