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
- Corpus: 116 language files under `src`; an exact duplicate lkjedit entry was removed
- Physical format: one column-one marker/atom per line with matched markers and
  raw `str/`, `name/`, and `import/` blocks
- Source limits: depth 8, form children 16, tokens 384, top-level forms 8, and
  16 combined immediate files/directories per source directory
- Source-tree scope: the width rule applies to language source directories,
  not Rust, docs, metadata, `.git`, or generated Cargo output
- Imports: contained `std/`, `lib/`, `examples/`, and `./` paths with installed
  fallback through `LKJSCRIPT_ROOT`; absolute, parent, wrong-extension, cycle,
  and canonicalized symlink escapes fail
- Host implementation: six Rust workspace crates with no third-party Rust
  dependencies; unsafe Rust is confined to `lkjscript-sys`
- Quality gate: the complete Rust workspace is rustfmt-clean and passes strict
  Clippy for all targets/features; product panic/unwrap/expect paths stay denied
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

1. `set`, optional `if`, and out-of-range `arg` behavior still have static or
   lifecycle disagreements;
2. imports load files but definitions share one global namespace and top-level
   initialization order can be unsafe;
3. strings and IO lack a lossless bulk byte contract, and some library file
   operations are per-byte or quadratic;
4. public malformed chunks can reach unchecked VM assumptions;
5. source/import aggregate bytes, depth, count, constants, globals, bytecode,
   VM fuel, heap, handles, output, and wall time are not comprehensively bounded;
6. the terminal exit guard remains process-global and is not a supervisor-safe
   terminal lease;
7. monotonic handle tokens retain closed metadata until the VM ends.

## Evidence

The current working tree was checked on Linux x86-64 with Rust/Cargo
1.96.0. Evidence is command-specific; Docker and performance are not implied.

| Command or check | Result |
| --- | --- |
| `cargo check --workspace --all-targets --locked` | passed |
| `cargo run --locked -p lkjscript-xtask --quiet -- quiet verify` | passed; rustfmt, strict Clippy, and 49 workspace tests passed |
| `check-tree` boundaries | 16 accepted; 17 including a hidden entry rejected |
| `check-sources` | passed for 116 `.lkjscript` sources, 172 imports, and 11 compile roots |
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
| Markdown local links/status audit | 36 files, zero broken links, zero missing statuses |
| `git diff --check` | passed |
| Docker | not run for this numeric cycle |
| repeated performance comparison | not run |

A gate that did not run did not pass.

## Accepted Next Target

The next safety/conformance sequence is:

1. add generated whole-prelude/codegen/VM conformance coverage;
2. repair `set`, optional `if`, `arg`, and global initialization contracts;
3. make documentation status, exact source-closure coverage, and explicit
   placeholder scanning machine-checked.

## Deferred

Native installation and update, package manifests/locks/registry, process-safe
VM outcomes, supervisor/scheduler, adaptive or generational GC, native JIT,
non-Linux backends, browser, general HTTP server/framework, and GUI runtime are
later cycles. Their documents are designs or experiments, not capability
claims.
