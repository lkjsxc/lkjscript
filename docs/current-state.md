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
- Runtime: dense bytecode, contiguous stacks, precise non-moving mark-sweep,
  and return-adjacent tail-frame reuse
- CLI: `run`, real bytecode `disasm`, help, and version; the unlabeled REPL stub
  was removed
- Workloads: hello, Mandelbrot, lkjedit, one-shot HTTP, and Leibniz comparison
- Send behavior: successful `sys-send` reports its byte count and uses Linux
  `MSG_NOSIGNAL` instead of risking process termination on a broken peer
- JIT seam: explicitly labeled **PLACEHOLDER** observation hook; there is no
  native compilation or execution handoff

## Known Defects

The source identity cutover does not make the runtime semantically complete.
The highest-priority defects are:

1. arbitrary script-controlled ioctl request/buffer pairs cross an unsound safe
   Rust wrapper;
2. raw descriptor and reusable resource-table handle namespaces overlap, and
   stale handles can alias later resources;
3. many ordinary system failures abort the VM instead of returning the
   Result values promised by the type prelude;
4. numeric widths, literals, arithmetic, casts, and code generation do not all
   match their static signatures;
5. `set`, optional `if`, `arg`, and several comparison/operator paths have
   type/runtime disagreements;
6. imports load files but definitions share one global namespace and top-level
   initialization order can be unsafe;
7. strings and IO lack a lossless bulk byte contract, and some library file
   operations are per-byte or quadratic;
8. public malformed chunks can reach unchecked VM assumptions;
9. source/import aggregate bytes, depth, count, constants, globals, bytecode,
   VM fuel, heap, handles, output, and wall time are not comprehensively bounded;
10. the repository is not rustfmt-clean and strict Clippy still reports
    pre-existing production and test debt.

## Evidence

The source-cutover working tree was checked on Linux x86-64 with Rust/Cargo
1.96.0. Evidence is command-specific; Docker and performance are not implied.

| Command or check | Result |
| --- | --- |
| `cargo check --workspace --all-targets --locked` | passed |
| `cargo run --locked -p lkjscript-xtask --quiet -- quiet verify` | passed; 23 workspace tests passed |
| `check-tree` boundaries | 16 accepted; 17 including a hidden entry rejected |
| `check-sources` | passed for 116 `.lkjscript` sources, 172 imports, and 11 compile roots |
| `cargo build --workspace --release --locked` | passed |
| canonical hello | passed; output `3628800` |
| Mandelbrot | passed; 1,176 bytes and 24 lines |
| lkjedit smoke | passed |
| one-shot HTTP smoke | passed |
| `disasm` hello | passed; 81 lines with decoded offsets, operands, and opcodes |
| script argument `--help` after `--` | passed through to the script |
| `.lkjml` CLI run | rejected with canonical-extension diagnostic |
| Markdown local links/status audit | 33 files, zero broken links, zero missing statuses |
| `git diff --check` | passed |
| Docker | not run for this cutover |
| repeated performance comparison | not run |

A gate that did not run did not pass.

## Accepted Next Target

The next safety/conformance sequence is:

1. replace arbitrary ioctl with fixed size-validated terminal operations;
2. introduce namespace-separated stale-safe resource handles;
3. turn all ordinary fallible system operations into truthful language Results;
4. implement an exact current numeric contract and remove unsupported prelude
   vocabulary;
5. add generated prelude/codegen/VM conformance coverage;
6. make rustfmt, Clippy, documentation status, source coverage, and explicit
   placeholder scanning part of the local honesty gate.

## Deferred

Native installation and update, package manifests/locks/registry, process-safe
VM outcomes, supervisor/scheduler, adaptive or generational GC, native JIT,
non-Linux backends, browser, general HTTP server/framework, and GUI runtime are
later cycles. Their documents are designs or experiments, not capability
claims.
