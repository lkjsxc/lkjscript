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
- Corpus: 127 language files under `src`; duplicate and builtin-shadowing wrappers were removed
- Physical format: one column-one marker/atom per line with matched markers and
  raw `str/`, `name/`, and `import/` blocks
- Source limits: depth 8, form children 16, tokens 384, top-level forms 8, and
  16 combined immediate files/directories per source directory
- Source-tree scope: the width rule applies to language source directories,
  not Rust, docs, metadata, `.git`, or generated Cargo output
- Imports: contained `std/`, `lib/`, `examples/`, and `./` paths with installed
  fallback through `LKJSCRIPT_ROOT`; absolute, parent, wrong-extension, cycle,
  and canonicalized symlink escapes fail
- Compiler boundary: one analysis pass collects headers and produces owned,
  resolved typed HIR with BindingIds, source origins, exact operation/type
  facts, and conservative effects; bytecode consumes HIR without re-resolving
  source names or declarations
- Host implementation: six Rust workspace crates with no third-party Rust
  dependencies; unsafe Rust is confined to `lkjscript-sys`
- Quality gate: the complete Rust workspace is rustfmt-clean and passes strict
  Clippy for all targets/features; docs status/links, explicit `PLACEHOLDER`
  labels, and exact source-closure coverage are machine-checked
- Runtime: dense bytecode, contiguous stacks, precise non-moving mark-sweep,
  and return-adjacent tail-frame reuse
- Semantics: Unit, typed empty-list, and Option none have distinct singleton
  tags, while Option some is traced; `nil`, `Nil`, `nil?`, and `null?` are
  removed; `arg` returns `Option Str`; empty `do`/`while`/`set` return Unit;
  universal `eq`/`ne` are removed in favor of exact value, object-identity,
  bounded structural-list, and F64-bit equality families
- Numerics: canonical I64/F64 only; complete I64 uses signed 61-bit immediates
  plus boxed wide values, F64 remains distinct, arithmetic/comparison is
  checked or IEEE as declared, and narrower host domains reject truncation
- CLI: `run`, real bytecode `disasm`, help, and version; the unlabeled REPL stub
  was removed
- Workloads: hello, native lkjscript Mandelbrot, Brainfuck Mandelbrot interpreted by lkjscript, lkjedit, one-shot HTTP, and Leibniz comparison
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
- JIT seam: explicitly labeled **PLACEHOLDER** observation hook; there is no
  native compilation, engine selector, code object, execution handoff, OSR, or
  deoptimization
- Adaptive-performance contract: runtime JIT is the **Accepted Target** after
  semantic/outcome and typed-SSA prerequisites; the VM remains the cold tier
  and oracle, and minimal AOT emission is only a shared-backend test surface

## Known Defects

The source identity cutover does not make the runtime semantically complete.
The highest-priority defects are:

1. mutable values and imports still share one program-global namespace, and
   top-level execution/initialization remains order-dependent;
2. user-call effects are a safe all-effects over-approximation rather than
   fixed-point summaries;
3. strings and IO lack a lossless bulk byte contract, and some library file
   operations are per-byte or quadratic;
4. public malformed chunks are not prevalidated, although stack underflow,
   uninitialized slots, bad slot indexes, removed opcodes, and non-Bool control
   now return VM errors instead of semantic fallback values;
5. source/import aggregate bytes, depth, count, constants, globals, bytecode,
   VM fuel, heap, handles, output, and wall time are not comprehensively bounded;
6. process exit/terminal state remain process-global, and monotonic handle
   metadata remains until the VM ends.

## Evidence

The current working tree was checked on Linux x86-64 with Rust/Cargo
1.96.0. Evidence is command-specific; Docker and performance are not implied.

| Command or check | Result |
| --- | --- |
| `cargo check --workspace --all-targets --locked` | passed |
| `cargo run --locked -p lkjscript-xtask --quiet -- quiet verify` | passed; rustfmt, strict Clippy, and 80 workspace tests passed |
| `check-tree` boundaries | 16 accepted; 17 including a hidden entry rejected |
| documentation honesty boundaries | missing status, broken local link, and lowercase inert marker rejected; clean tree passed |
| `check-sources` | passed for all 127 `.lkjscript` sources; the 11 compiled entry closures equal the corpus exactly |
| source-closure boundary | an otherwise valid orphan source was rejected; clean exact closure passed |
| HIR conformance | duplicate/unknown/collision, BindingId shadowing, generic resolution, effects, global set, exact `if`, typed empty-list, Option, explicit equality, and `arg` boundaries passed |
| top-level control-flow boundary | nonzero-offset short-circuit jumps execute correctly; `set` yields dedicated Unit as typed |
| `cargo build --workspace --release --locked` | passed |
| canonical hello | passed; output `3628800` |
| Mandelbrot | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| Brainfuck smoke | direct and run-folded correctness/failure boundaries passed |
| Brainfuck Mandelbrot interpreted by lkjscript | direct implementation exceeded the 1,800-second ceiling; run-folded output matched the 6,240-byte independent oracle; three measured end-to-end release-process runs had 1,281.143690-second median and 6.731340-second MAD; post-equality full correctness remained byte-identical |
| lkjedit smoke | passed |
| one-shot HTTP smoke | passed |
| terminal safety unit tests | wrong-size buffers rejected before FFI; exact size reaches only fixed requests |
| resource handle unit tests | integer/borrowed close, stale reuse, repeated close, and wrong-kind use rejected |
| terminal Result workload | 59-byte state returned `ResultErr`; VM continued and exited successfully |
| system Result workload | missing open, repeated close, and negative wait returned errors; later expressions ran; exit 0 |
| Result unit coverage | malformed path, invalid handle/timeout/range, error text, and canonical names passed |
| equality conformance | exact scalar/Option/Result value equality, Buf/Handle identity, bounded structural List equality, F64 bit equality, category errors, removed `eq`/`ne`, and retired opcode 21 passed |
| numeric conformance | complete I64 boundaries, boxed transition, checked arithmetic/division, IEEE F64 identity/equality, exact F64-bit equality, 64-bit bitwise, byte/u32 narrowing, and removed vocabulary passed |
| numeric CLI boundary | exact `9007199254740993 + 2`; overflow and `1e3` rejected |
| `disasm` hello | passed; 81 lines with decoded offsets, operands, and opcodes |
| script argument `--help` after `--` | passed through to the script |
| `.lkjml` CLI run | rejected with canonical-extension diagnostic |
| HIR diagnostic performance sample | 31 randomized runs: hello 0.990x, Mandelbrot 0.964x, Leibniz 0.984x, Mandelbrot disassembly 0.899x candidate/baseline median; release binary 1.082x size |
| Unit/strict-if diagnostic sample | 31 randomized runs: hello 0.993x, Mandelbrot 0.985x, Leibniz-200,000 1.004x, Mandelbrot disassembly 1.005x candidate/baseline median; release binary 0.982x size |
| typed-empty-list diagnostic sample | 31 randomized runs: hello 1.002x, Mandelbrot 0.979x, Leibniz-200,000 0.937x, Mandelbrot disassembly 1.003x candidate/baseline median; release binary 1.009x size |
| Option/no-nil diagnostic sample | 31 randomized runs: hello 0.994x, Mandelbrot 1.027x, Leibniz-200,000 1.015x, Mandelbrot disassembly 1.014x candidate/baseline median; release binary 1.012x size |
| explicit-equality diagnostic sample | 31 randomized runs: hello 1.029x, Mandelbrot 1.003x, Leibniz-200000 0.978x, Mandelbrot disassembly 0.972x, Brainfuck hello 0.998x candidate/baseline median; release binary 1.006x size |
| Markdown local links/status audit | 41 files, zero broken links, zero missing statuses |
| `git diff --check` | passed |
| Docker verify profile | passed; the verification image includes machine-required `AGENTS.md` and committed benchmark documentation link targets |
| decision-grade performance suite | not yet run; HIR figures above are a focused diagnostic comparison |

A gate that did not run did not pass.

## Accepted Next Target

The next implementation sequence is:

1. establish explicit main/effect-free libraries, immutable product state and
   local var/set, then remove mutable globals;
2. compute required effect summaries, validate chunks, and return structured
   process-safe VM outcomes;
3. implement typed SSA and its verifier/differential oracle before the shared
   Linux x86-64 code-object backend and function-triggered baseline JIT.

The contracts are [AI-First Semantic Core](decisions/semantic-core.md),
[Explicit Equality Families](decisions/equality-families.md),
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

Native installation and update, package manifests/locks/registry,
supervisor/scheduler, adaptive or generational GC, background JIT compilation,
guarded runtime specialization/deoptimization, non-Linux native backends,
browser, general HTTP server/framework, and GUI runtime are later cycles.
Their documents are designs or experiments, not capability claims.
