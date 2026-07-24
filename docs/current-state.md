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
- Corpus: 94 language files under `src`; nine executable roots cover the exact corpus closure
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
- Host implementation: eight Rust workspace crates with no third-party Rust
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
- Native foundation: dependency-free `lkjscript-native` verifies a closed
  scalar machine plan and emits owned Linux x86-64 bytes, relocations, ABI/frame
  facts, safepoints, empty scalar stack maps, and opaque installable images;
  `lkjscript-sys` alone owns bounded RW-to-RX installation and typed invocation;
  generated multi-block, loop, direct-call, runtime-call, exact I64/F64/trap/
  exit code has been called in boundary tests, but no SSA adapter or VM transfer
  exists yet
- JIT seam: explicitly labeled **PLACEHOLDER** observation hook; there is no
  source-to-native compilation, engine selector, VM/native execution handoff,
  tier state, OSR, or deoptimization
- Native backend decision: the measured owned Linux x86-64 encoder selected over
  Cranelift 0.134.2 is now the Current low-level foundation; the production
  baseline JIT remains an **Accepted Target**
- Adaptive-performance contract: runtime JIT is the **Accepted Target** after
  semantic/outcome and typed-SSA prerequisites; the VM remains the cold tier
  and oracle, and minimal AOT emission is only a shared-backend test surface

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

## Accepted Next Target

The active engineering cycle must reach a real callable baseline JIT on Linux
x86-64; documentation, typed SSA, machine-code emission, disassembly, or an
observation hook alone cannot complete it. The dependency sequence is:

Explicit typed main, declaration-only imports, local-only mutation, removal of
source runtime globals, product-threaded workload state, whole-chunk validation,
structured outcomes, bounded VM execution, deterministic fixed-point function
effects, verified typed SSA, the independent differential evaluator, isolated
baseline normalization, and reference-bytecode cutover are Current. The
remaining dependency sequence is:

1. lower verified SSA into the selected owned encoder, add the remaining
   versioned runtime calls and code-object/tier ownership, and implement exact
   VM/native and native/native calls plus forced source-to-native execution;
2. expose truthful `vm`, `auto`, and `baseline-jit` modes only after generated
   code has been called. Forced mode never falls back; auto uses synchronous
   function-entry tiering and may remain in the VM for unsupported code.

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
