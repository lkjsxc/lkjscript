# Current State

## Purpose

State implemented behavior, evidence boundaries, known defects, and accepted
next work without mixing them with long-term vision.

## Status

**Current** for the implementation section. Repairs and future products are
explicitly labeled **Accepted Target**, **Placeholder**, **Deferred**, or
**Rejected**.

## Current Implementation

- Repository: `https://github.com/lkjsxc/lkjscript`
- Canonical source: `.lkjscript`; other extensions are rejected without shims
- Corpus: 96 language files under `src`; ten executable roots cover the exact corpus closure
- Physical format: one column-one marker/atom per line with matched markers and
  raw `str/`, `name/`, and `import/` blocks
- Source limits: depth 8, form children 16, tokens 384, top-level forms 8,
  product fields 15, and 16 combined immediate files/directories per source
  directory
- Source-tree scope: the width rule applies to language source directories,
  not Rust, docs, metadata, `.git`, or generated Cargo output
- Imports: contained `std/`, `lib/`, `examples/`, and `./` paths with installed
  fallback through `LKJSCRIPT_ROOT`; absolute, parent, wrong-extension, cycle,
  and canonicalized symlink escapes fail
- Compiler boundary: one analysis pass collects immutable headers and produces
  owned, resolved typed HIR with explicit Main and Functions, BindingIds,
  local-slot references, MutableLocal/SetLocal, ProductIds, numeric field
  identities, source origins, exact operation/type facts, and deterministic
  fixed-point function effect summaries; HIR lowers once into verified typed
  SSA, deterministic baseline normalization, and then reference bytecode
- Typed SSA: dependency-free `lkjscript-ir` owns dense function/block/value
  identities, exact types and nominal product metadata, explicit block
  parameters and terminators, direct/indirect/runtime calls, effects,
  safepoints, frame states, source origins, verification, an independent
  bounded evaluator, deterministic isolated baseline passes, and bytecode link
  metadata; SSA conversion renames local mutation and uses stable BindingId-
  ordered block parameters at branch and loop joins
- Host implementation: nine Rust workspace crates with no third-party Rust
  dependencies; unsafe Rust is confined to `lkjscript-sys`
- Quality gate: the complete Rust workspace is rustfmt-clean and passes strict
  Clippy for all targets/features; docs status/links, explicit `PLACEHOLDER`
  labels, and exact source-closure coverage are machine-checked
- Runtime: dense bytecode lowered only from normalized SSA, contiguous stacks,
  precise non-moving mark-sweep, traced immutable product objects, explicit
  validated `Trap`, and return-adjacent tail-frame reuse
- Execution boundary: mutable `Chunk` is builder-only for malformed-bytecode
  construction; one whole-chunk validator produces opaque immutable
  `ValidatedChunk`, and VM, disassembly, and the JIT observation seam accept
  only validated input; compiler `ExecutableProgram` retains verified
  normalized SSA, deterministic function/prototype/main and SSA/bytecode link
  metadata, and validated bytecode through an explicit accessor
- Outcomes: VM execution distinguishes returned, exited, trapped, deadline,
  resource-limit, and host-failure outcomes; the core does not terminate the
  process, returned heap values own their reachable storage, and cleanup occurs
  before CLI exit-status translation
- Runtime budgets: explicit configuration bounds fuel, stack values, frames,
  estimated live heap, aggregate allocations, handles, output, and cooperative
  wall time; hard-deadline mode rejects host wrappers that cannot guarantee
  cancellation
- Semantics: executable roots have exactly one no-parameter typed main;
  imports contain declarations only; top-level `do` and runtime value defs are
  removed; `var` introduces one exactly typed mutable local and local-only
  `set` returns Unit; Unit, typed empty-list, and Option none have distinct
  singleton tags, while Option some is traced; `nil`, `Nil`, `nil?`, and
  `null?` are removed; `arg` returns `Option Str`; universal `eq`/`ne` are
  removed in favor of exact value, object-identity, bounded structural-list,
  and F64-bit equality families; nominal products have ordered named fields,
  exact construction, access, and immutable replacement
- Numerics: canonical I64/F64 only; complete I64 uses signed 61-bit immediates
  plus boxed wide values, F64 remains distinct, arithmetic/comparison is
  checked or IEEE as declared, and narrower host domains reject truncation
- CLI: `run`, real bytecode `disasm`, help, and version; the unlabeled REPL stub
  was removed
- Workloads: hello, native lkjscript Mandelbrot, Brainfuck interpreted by
  lkjscript, lkjedit, one-shot HTTP, and Leibniz comparison; Brainfuck,
  terminal, and editor state is passed explicitly in immutable nominal products
  and evolved through local vars
- Resource handles: integers are rejected, stdin uses a reserved borrowed token,
  owned file/socket tokens are monotonic, and closed tokens are never reused
- Terminal ABI: arbitrary ioctl is absent; fixed `sys-tty-get`/`sys-tty-set`
  operations validate the exact 60-byte Linux state before FFI and return Results
- System Results: open, path existence, close/read/write, `isatty`, time,
  socket, poll, terminal, and terminal-guard failures return operation-qualified
  `ResultErr` values; standard wrappers unwrap explicitly
- Canonical resource names: `stdin-handle`, `sys-close`, `sys-read-byte`,
  `sys-write-byte`, and `sys-isatty`; descriptor-era aliases are absent
- Send behavior: successful `sys-send` reports its byte count and uses Linux
  `MSG_NOSIGNAL` instead of risking process termination on a broken peer
- SSA evaluator: independent of bytecode, VM, native, and host helpers; it
  covers exact scalar/control semantics, calls and recursion, SSA-converted
  local mutation, products, Option/Result, lists, strings, deterministic args,
  host-independent buffers, traps, exits, and explicit fuel/frame/allocation/
  buffer/list bounds; console, filesystem, sockets, terminal, time, and handle
  operations return explicit unsupported-evaluator outcomes
- Callable baseline JIT: `lkjscript-jit` consumes only `VerifiedProgram`,
  rejects every reference/allocation/host path, lowers allocation-free scalar
  Unit/Bool/I64/F64 CFG and acyclic direct calls to `lkjscript-native`, installs
  bounded owned non-Send code objects through `lkjscript-sys`, and actually
  invokes generated System V AMD64 entries; direct native calls stay unboxed
- Native runtime ABI: enum-identified `EnterFunctionV1` and `PollV1` calls record
  per-source-function entries and enforce cooperative native poll fuel and a
  monotonic deadline; structured return/trap/exit/deadline/resource/host status
  returns to the execution owner and generated code never exits the process
- Engine modes: explicit `vm`, `baseline-jit`, and `auto` work; `vm` remains the
  default, forced baseline compiles the complete reachable supported group
  before main effects and never falls back, while auto compiles synchronously at
  a hot function entry for later calls and keeps unsupported code in the VM
- Tier/code ownership: the former observation hook is removed. Per-function
  states are `VmOnly`, `Observed`, `BaselineCompiling`, `BaselineNative`, or
  `Disabled` with saturating calls, bounded attempts, epoch/failure/object facts,
  and native entries. Code objects retain ABI/tier/group, size/accounting,
  relocation/runtime/safepoint/source/outcome, compile/install, invalidation,
  W^X, and entry metadata under bounded synchronous session ownership
- Native limits: recursion, indirect calls, polymorphic or unsupported
  signatures, references, strings, collections, products, Option/Result,
  buffers, allocation, and host IO are explicit forced-mode engine errors.
  Empty native stack maps are exact for this allocation-free scalar subset
- Deferred tiers: loop OSR, background compilation, optimizing/speculative
  tiers, deoptimization, native references/allocation, persistent profiles, and
  persistent code caches remain absent

## Known Defects

The source identity cutover does not make the runtime semantically complete.
The highest-priority defects are:

1. strings and IO lack a lossless bulk byte contract, and some library file
   operations are per-byte or quadratic;
2. source/import aggregate bytes and counts are not comprehensively bounded;
   bytecode tables/data/code/metadata and VM execution resources are bounded;
3. cooperative deadlines can overrun inside filesystem, console-write,
   send/write, terminal-cleanup, or other non-cancellable wrappers;
   hard-deadline mode reports those operations as unsupported `HostFailure`
   before effects; live-heap accounting is estimated at VM instruction
   boundaries, and `print` builds its host-format string before the output check;
4. stdin/stdout and the terminal guard remain process-global, so concurrent VM
   supervision is unsupported; handle metadata is VM-local and bounded but
   monotonically allocated until that VM ends.

## Evidence

Phase A implementation commit
`12836da90d886c9e741a5ac9f8148a17d00f0505` and the state-threaded editor
behavior follow-up `91d7e9bb734307269eb44b2d3a0882ba55d2f5b2`, based on `e4c1d0e`, were
checked on Linux x86-64 with Rust/Cargo 1.96.0. Evidence is command-specific; Docker, full Brainfuck
Mandelbrot, and performance are not implied.

| Command or check | Result |
| --- | --- |
| `cargo check --workspace --all-targets --locked` | passed |
| focused `lkjscript-compiler` and app HIR/numeric tests | passed; 37 compiler and 10 app integration tests |
| `cargo run --locked -p lkjscript-xtask --quiet -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 82 workspace tests |
| `check-sources` | passed for all 94 `.lkjscript` sources; the nine compiled executable closures equal the corpus exactly |
| HIR/local mutation conformance | explicit Main/Function, missing/duplicate/imported main, declaration-only imports, rejected top-level effects/value defs, stable BindingId/local-slot shadowing, initializer scope, local-only set rejection and exact typing, same-function isolation, ProductId/field resolution, and StoreLocal execution passed |
| `cargo build --workspace --release --locked` | passed |
| canonical hello | passed; output `3628800` |
| Mandelbrot | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| Brainfuck smoke | direct and run-folded correctness/failure boundaries passed |
| lkjedit smoke | passed; existing-file insert/save/reopen, missing-file creation, CRLF redraw, and command paint |
| one-shot HTTP smoke | passed |
| validated-chunk boundaries | centralized decode/CFG/metadata validation and random raw-chunk no-panic tests passed after integration |
| structured execution boundaries | return/exit/trap/deadline and configured resource categories passed; returned heap values remain owned after VM teardown |
| native-backend decision spike | 8 randomized warmups plus 31 retained pairs; exact generated calls passed; owned execution median/MAD 48.406374/0.540016 ms versus Cranelift 0.134.2 119.422902/0.566505 ms; temporary artifacts removed; no production backend implemented |
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
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 101 workspace tests |
| fixed-point effect conformance | passed; pure leaf, direct/transitive propagation, direct and mutual recursion, recursive effects, allocation, memory read/write, local mutation, host IO, process exit, trap, declaration-order independence, generic canonical direct calls, retained argument effects, and conservative indirect calls |

Phase C typed-SSA/reference-bytecode contract commit `787d7b1` and
implementation commits `41deaef`, `0c9903b`, `d9a6917`, `47c3b83`, and
`1b7b1ce`, based on
`ec2afbb1161eff437370d1e75c9522af9a261342`, were checked on Linux x86-64 with
Rust/Cargo 1.96.0. This evidence establishes typed SSA and the reference
cutover, not native execution, JIT tiering, OSR, Docker, or performance.

| Phase C command or check | Result |
| --- | --- |
| focused crate tests | passed; 6 `lkjscript-ir`, 44 compiler, 14 core, 31 VM, and 14 app tests |
| SSA differential conformance | passed; exact focused Unit/Bool/I64/F64/control/loops/calls/recursion/local mutation/products/Option/Result/buffers/traps/exits, explicit unsupported host operations, tail-call bytecode shape, and 64 deterministic bounded randomized typed scalar programs |
| malformed SSA and pass conformance | passed; direct malformed identity/use/dominance/edge/loop/effect cases, each isolated pass, repeated determinism, post-pass verification, combined normalization, and evaluator bounds |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | passed |
| `check-docs`, `check-tree`, and `check-sources` | passed; all nine executable closures cover all 94 canonical sources through SSA and validated bytecode |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 112 workspace tests |
| `cargo build --workspace --release --locked` | passed |
| canonical hello | passed; output `3628800` |
| Mandelbrot | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| Brainfuck smoke | passed after preserving return-adjacent tail calls and liveness-allocated typed bytecode locals; direct and run-folded correctness/failure checks passed; full Brainfuck Mandelbrot was not run |
| lkjedit and one-shot HTTP smoke | passed |

The native-foundation commit based on `ec2afbb` passed six focused native/sys
unit and integration tests, strict Clippy, the then-current 106-test canonical
gate, and generated-code invocation for multi-block control, a 100-iteration
loop, direct native calls, an allowlisted runtime call, exact I64 traps, F64
bits/comparisons, structured exit, W^X permissions, limits, and 32 repeated
install/invoke/drop cycles. It did not connect source or SSA to native code and
therefore was not a JIT test.

Earlier decision-grade and diagnostic performance records remain in
[Experiment Registry](vision/experiments.md); they were not rerun for Phase A,
Phase B, or Phase C. A gate that did not run did not pass. Docker, full
Brainfuck Mandelbrot, source-to-native execution, and performance were not
tested for Phase C.

The callable scalar baseline implementation chain through
`3117819d890cd1b3eda8651b9fae104a6ec31214`, based on current-main
`c4c96094260072323f9399fe7f0f7b4a14d1eef6`, was checked in isolated worktree
`/tmp/pi-agent-a98a8be7-b37a-422-f33e779d` on Linux
`7.0.0-27-generic` x86-64 with Rust/Cargo 1.96.0. The evidence establishes the
exact allocation-free scalar subset, not full-language native execution, OSR,
or a performance result.

| Callable baseline command or check | Result |
| --- | --- |
| focused IR/compiler/native/sys/JIT/VM/app tests | passed; the final canonical workspace gate reports 116 tests, including 7 source-engine and 1 direct verified-SSA JIT tests |
| strict workspace Clippy, all targets/features | passed with `-D warnings` |
| `check-docs`, `check-tree`, `check-sources` | passed; ten roots exactly cover all 96 canonical sources |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs/tree/source closure, rustfmt, strict Clippy, and all 116 tests |
| `cargo build --workspace --release --locked` | passed in the shared target tree |
| scalar workload, explicit `vm` / `baseline-jit` / threshold-2 `auto` | all exited 0 with empty stdout and exact test-oracle F64 bits |
| forced scalar diagnostics | one installed W^X object; compiled `scalar-step` and `main`; 100,001 native entries, 100,000 direct native calls, 300,002 PollV1 calls, zero VM fallbacks/failures |
| auto scalar diagnostics | 99,998 later-call native entries, 99,998 PollV1 calls, exactly two initial VM calls, zero compile failures; no OSR claim |
| explicit VM and threshold-2 auto hello | both output `3628800`; auto recorded 15 native leaf entries and one retry-suppressed recursive-group failure |
| direct Mandelbrot in VM | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| Brainfuck smoke only | passed direct/run-folded correctness and failure checks; full Brainfuck Mandelbrot was not run |
| lkjedit and one-shot HTTP smokes | passed |
| opt-in generated binary plus external `objdump` | passed; 1,926-byte source-derived object dumped, disassembled, then removed; normal stdout stayed empty |

Docker, full Brainfuck Mandelbrot, performance sampling, OSR, background work,
optimizing/speculative tiers, native references/allocation/host IO, and
non-Linux/non-x86-64 acceptance were not run or implemented.

## Accepted Next Target

The real callable allocation-free scalar baseline-JIT cycle is Current on Linux
x86-64. Emission alone did not complete it: canonical source now reaches actual
installed calls with nonzero main/callee/PollV1 counts and no forced fallback.
The next dependency sequence is:

1. retain and broaden exact scalar baseline evidence without weakening forced
   errors or bounded code-object ownership;
2. design loop-header state transfer separately before making any OSR claim.
   Native references/allocation require exact live-reference maps first.

OSR, background compilation, optimizing JIT, guards, deoptimization, persistent
profiles/caches, offline PGO, and non-Linux/non-x86-64 acceptance are outside
this cycle. The exact syntax, validation, outcome, SSA, backend-selection, ABI,
engine, safety, and evidence contract is
[Callable Linux x86-64 Baseline JIT Cycle](decisions/callable-baseline-jit.md).

The supporting contracts are [AI-First Semantic Core](decisions/semantic-core.md),
[Explicit Equality Families](decisions/equality-families.md),
[Immutable Nominal Products](decisions/immutable-nominal-products.md),
[Linux x86-64 Native Backend](decisions/linux-x86-64-native-backend.md),
[Typed Compiler Pipeline And Runtime JIT](decisions/compiler-pipeline.md),
[Runtime JIT Instead of Offline PGO](decisions/runtime-jit-instead-of-offline-pgo.md),
and the [Performance Scorecard](vision/performance-scorecard.md).

## Rejected

Offline PGO, instrumented training builds, profile generation/merging/use, and
persistent PGO artifacts are rejected by product decision, not measurement.
Persistent cross-run JIT profiles and native-code caches are not planned without
a later explicit decision. Current-process bounded JIT counters are local,
ephemeral, and not telemetry.

## Deferred

Package installation and update, package manifests/locks/registry,
supervisor/scheduler, adaptive or generational GC, background JIT compilation,
guarded runtime specialization/deoptimization, non-Linux native backends,
browser, general HTTP server/framework, and GUI runtime are later cycles.
Their documents are designs or experiments, not capability claims.
