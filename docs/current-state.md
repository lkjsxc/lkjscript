# Current State

## Purpose

State implemented behavior, evidence boundaries, known defects, and accepted
next work without mixing them with long-term vision.

## Status

**Current** for the implementation section. Repairs and future products are
explicitly labeled **Accepted Target**, **Placeholder**, or **Deferred**.

## Current Implementation

- Repository: `https://github.com/lkjsxc/lkjscript`
- Canonical source: `.lkjscript`; other extensions are rejected without shims
- Corpus: 115 language files under `src`; duplicate and builtin-shadowing wrappers were removed
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
  resolved typed HIR with BindingIds, source origins, operation signatures,
  explicit legacy runtime unions, and conservative effects; bytecode consumes
  HIR without re-resolving source names or declarations
- Host implementation: six Rust workspace crates with no third-party Rust
  dependencies; unsafe Rust is confined to `lkjscript-sys`
- Quality gate: the complete Rust workspace is rustfmt-clean and passes strict
  Clippy for all targets/features; docs status/links, explicit `PLACEHOLDER`
  labels, and exact source-closure coverage are machine-checked
- Runtime: dense bytecode, contiguous stacks, precise non-moving mark-sweep,
  and return-adjacent tail-frame reuse
- Numerics: canonical I64/F64 only; complete I64 uses signed 61-bit immediates
  plus boxed wide values, F64 remains distinct, arithmetic/comparison is
  checked or IEEE as declared, and narrower host domains reject truncation
- CLI: `run`, real bytecode `disasm`, help, and version; the unlabeled REPL stub
  was removed
- Workloads: hello, Mandelbrot, lkjedit, one-shot HTTP, and Leibniz comparison
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
  native compilation or execution handoff

## Known Defects

The source identity cutover does not make the runtime semantically complete.
The highest-priority defects are:

1. generic Nil conflates Unit, Option absence, empty lists, falsey/default VM
   state, and uninitialized mutable globals; Nil-joined if expressions are
   explicitly marked as legacy runtime unions in HIR;
2. mutable values and imports still share one program-global namespace, and
   top-level execution/initialization remains order-dependent;
3. out-of-range `arg` still returns Nil despite its declared Str type;
4. user-call effects are a safe all-effects over-approximation rather than
   fixed-point summaries;
5. strings and IO lack a lossless bulk byte contract, and some library file
   operations are per-byte or quadratic;
6. public malformed chunks can reach unchecked VM assumptions;
7. source/import aggregate bytes, depth, count, constants, globals, bytecode,
   VM fuel, heap, handles, output, and wall time are not comprehensively bounded;
8. process exit/terminal state remain process-global, and monotonic handle
   metadata remains until the VM ends.

## Evidence

The current working tree was checked on Linux x86-64 with Rust/Cargo
1.96.0. Evidence is command-specific; Docker and performance are not implied.

| Command or check | Result |
| --- | --- |
| `cargo check --workspace --all-targets --locked` | passed |
| `cargo run --locked -p lkjscript-xtask --quiet -- quiet verify` | passed; rustfmt, strict Clippy, and 60 workspace tests passed |
| `check-tree` boundaries | 16 accepted; 17 including a hidden entry rejected |
| documentation honesty boundaries | missing status, broken local link, and lowercase inert marker rejected; clean tree passed |
| `check-sources` | passed for all 115 `.lkjscript` sources; the 10 compiled entry closures equal the corpus exactly |
| source-closure boundary | an otherwise valid orphan source was rejected; clean exact closure passed |
| HIR conformance | duplicate/unknown/collision, BindingId shadowing, source origin, generic resolution, effect facts, global set, operation signature, and legacy-union boundaries passed |
| top-level control-flow boundary | nonzero-offset short-circuit jumps execute correctly; `set` yields runtime Nil as typed |
| `cargo build --workspace --release --locked` | passed |
| canonical hello | passed; output `3628800` |
| Mandelbrot | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| lkjedit smoke | passed |
| one-shot HTTP smoke | passed |
| terminal safety unit tests | wrong-size buffers rejected before FFI; exact size reaches only fixed requests |
| resource handle unit tests | integer/borrowed close, stale reuse, repeated close, and wrong-kind use rejected |
| terminal Result workload | 59-byte state returned `ResultErr`; VM continued and exited successfully |
| system Result workload | missing open, repeated close, and negative wait returned errors; later expressions ran; exit 0 |
| Result unit coverage | malformed path, invalid handle/timeout/range, error text, and canonical names passed |
| numeric conformance | complete I64 boundaries, boxed transition, checked arithmetic/division, IEEE F64 identity/equality, 64-bit bitwise, byte/u32 narrowing, and removed vocabulary passed |
| numeric CLI boundary | exact `9007199254740993 + 2`; overflow and `1e3` rejected |
| `disasm` hello | passed; 81 lines with decoded offsets, operands, and opcodes |
| script argument `--help` after `--` | passed through to the script |
| `.lkjml` CLI run | rejected with canonical-extension diagnostic |
| HIR diagnostic performance sample | 31 randomized runs: hello 0.990x, Mandelbrot 0.964x, Leibniz 0.984x, Mandelbrot disassembly 0.899x candidate/baseline median; release binary 1.082x size |
| Markdown local links/status audit | 39 files, zero broken links, zero missing statuses |
| `git diff --check` | passed |
| Docker verify profile | passed after the image was corrected to include machine-required `AGENTS.md` |
| decision-grade performance suite | not yet run; HIR figures above are a focused diagnostic comparison |

A gate that did not run did not pass.

## Accepted Next Target

The next semantic migration sequence is:

1. replace Nil-as-Unit and optional-if behavior with Unit and exact three-arm
   branches while retaining typed empty-list and Option migrations as separate
   reviewable slices;
2. add explicit Option/empty-list values and split value/object/structural/F64
   bit equality;
3. establish explicit main/effect-free libraries, immutable product state and
   local var/set, then remove mutable globals;
4. validate chunks and return structured VM outcomes before typed SSA and the
   early Linux x86-64 native AOT experiment.

The contracts are [AI-First Semantic Core](decisions/semantic-core.md),
[Typed Compiler Pipeline And Early AOT](decisions/compiler-pipeline.md), and the
[Performance Scorecard](vision/performance-scorecard.md).

## Deferred

Native installation and update, package manifests/locks/registry, process-safe
VM outcomes, supervisor/scheduler, adaptive or generational GC, native JIT,
non-Linux backends, browser, general HTTP server/framework, and GUI runtime are
later cycles. Their documents are designs or experiments, not capability
claims.
