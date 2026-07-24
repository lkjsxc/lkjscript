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
- Corpus: 97 language files under `src`; 12 executable roots cover the exact corpus closure
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
  fixed-point function effect summaries; bytecode consumes HIR without
  re-resolving source names or declarations
- Host implementation: seven Rust workspace crates with no third-party Rust
  dependencies; unsafe Rust is confined to `lkjscript-sys`
- Quality gate: the complete Rust workspace is rustfmt-clean and passes strict
  Clippy for all targets/features; docs status/links, explicit `PLACEHOLDER`
  labels, and exact source-closure coverage are machine-checked
- Runtime: dense bytecode, contiguous stacks, precise non-moving mark-sweep,
  traced immutable product objects, and return-adjacent tail-frame reuse
- Execution boundary: mutable `Chunk` is builder-only; one whole-chunk
  validator produces opaque immutable `ValidatedChunk`, and VM, disassembly,
  and the JIT observation seam accept only validated input; compiler output
  crosses the same validator
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
- Lossless bulk bytes: bounded `Buf` UTF-8 conversion and offset/length-checked
  file/socket partial-progress reads and writes are Current; legacy Str socket
  operations remain only for old examples
- Durable files and entropy: append/create-new/directory handles, sync,
  truncate, same-filesystem rename, and Linux `getrandom` buffer fill are
  Current; application framing/recovery policy remains in language code
- SHA-256: fixed bounded-buffer digest is Current for verifier/integrity
  consumers; HMAC, password KDF, encryption, and WebAuthn remain absent
- Canonical resource names: `stdin-handle`, `sys-close`, `sys-read-byte`,
  `sys-write-byte`, and `sys-isatty`; descriptor-era aliases are absent
- Send behavior: successful `sys-send` reports its byte count and uses Linux
  `MSG_NOSIGNAL` instead of risking process termination on a broken peer
- JIT seam: explicitly labeled **PLACEHOLDER** observation hook; there is no
  native compilation, engine selector, code object, execution handoff, OSR, or
  deoptimization
- Native foundation: `lkjscript-native` owns a closed source-independent scalar
  machine plan, verifier, deterministic stack-slot lowering, x86-64 encoder,
  symbolic relocations, metadata-complete opaque `InstallableImage`, and exact
  ABI/accounting records; `lkjscript-sys` alone owns bounded RW-to-RX
  installation, allowlisted relocation resolution, typed invocation, permission
  probing, and unmapping. Intermediate tests actually call multi-block, loop,
  checked-trap, F64, direct-call, and runtime-slot generated code. This is a
  **Current native foundation**, not canonical source/SSA lowering, VM transfer,
  a runtime tier, CLI engine, or JIT
- Adaptive-performance contract: runtime JIT is the **Accepted Target** after
  semantic/outcome and typed-SSA prerequisites; the VM remains the cold tier
  and oracle, and minimal AOT emission is only a shared-backend test surface

## Known Defects

The source identity cutover does not make the runtime semantically complete.
The highest-priority defects are:

1. some library file operations remain per-byte or quadratic; SQLite and
   application-level storage recovery are not implemented;
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

The lossless bulk-byte and durable-file changes in this documentation's
containing commits were checked on Linux x86-64 with Rust/Cargo 1.96.0:

| Command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-core -p lkjscript-compiler -p lkjscript-sys -p lkjscript-vm` | passed; focused compiler/core/sys/VM coverage including exact binary socket transfer |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; workspace check, docs/tree/source closure, rustfmt, strict Clippy, and all workspace tests |
| `cargo build --workspace --release --locked`; bulk-byte, durable-file, and HTTP smokes | passed; exact `.lkjscript` file-buffer plus append/replay consumers and legacy HTTP behavior |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed; Docker source closure and all configured runtime smokes including bulk bytes and durable files |
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

The isolated native foundation in this documentation's containing commit,
based on `ec2afbb1161eff437370d1e75c9522af9a261342`, was checked on Linux
7.0.0-27-generic x86-64 with Rust/Cargo 1.96.0. These calls start from validated
machine plans rather than canonical source or SSA, so they are evidence for the
machine/W^X boundary only and not JIT completion.

| Native-foundation command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-native -p lkjscript-sys` | passed; invalid-plan boundaries plus actual multi-block, loop, checked I64 trap, F64/Bool/Unit, direct generated-call, versioned runtime-call, W^X, limit, version, and repeated install/drop execution |
| `cargo check --workspace --all-targets --locked` | passed |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | passed |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 106 workspace tests |
| W^X permission probe | passed; a sys-internal `/proc/self/maps` probe observed initial readable/writable/non-executable and sealed readable/non-writable/executable phases; no post-seal patch API is exposed |
| Not tested | Docker, runtime workload smokes, performance, non-Linux execution, canonical source/SSA lowering, VM/native transfer, tiering, engines, GC-reference stack maps, allocation, and JIT |

Earlier decision-grade and diagnostic performance records remain in
[Experiment Registry](vision/experiments.md); they were not rerun for Phase A,
Phase B, or this native foundation. A gate that did not run did not pass.

## Accepted Next Target

The active engineering cycle must reach a real callable baseline JIT on Linux
x86-64; documentation, SSA scaffolding, machine-code emission, disassembly, or
an observation hook alone cannot complete it. The dependency sequence is:

Explicit typed main, declaration-only imports, local-only mutation, removal of
source runtime globals, product-threaded workload state, whole-chunk validation,
structured outcomes, bounded VM execution, and deterministic fixed-point
function effects are Current. The remaining dependency sequence is:

1. lower HIR through verified typed SSA, an independent differential evaluator,
   and isolated non-speculative normalization; cut reference bytecode over to
   SSA before native lowering becomes authoritative;
2. add a narrow verified SSA adapter to the current closed scalar machine plan,
   then add complete non-scalar representations, exact reference stack maps,
   runtime services, VM/native transfers, native resource/deadline behavior,
   and forced source-derived native execution without reinterpreting HIR or
   bytecode;
3. expose truthful `vm`, `auto`, and `baseline-jit` modes only after generated
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

Native runtime integration and update, package manifests/locks/registry,
supervisor/scheduler, adaptive or generational GC, background JIT compilation,
guarded runtime specialization/deoptimization, non-Linux native backends,
browser, general HTTP server/framework, and GUI runtime are later cycles.
Their documents are designs or experiments, not capability claims.
