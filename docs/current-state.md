# Current State

## Purpose

Separate observed behavior from accepted targets and later ambitions.

## Status

**Current** baseline plus an explicitly separated **Accepted Target**.

## Current

This snapshot was observed before the foundation cutover.

- Commit: `8aa09d82280c8939d81078b0f040fdf10c550e35`
- Repository: `https://github.com/lkjsxc/lkjscript`
- Branch state: clean `main`, synchronized with `origin/main`
- Host: Linux x86-64, kernel `7.0.0-27-generic`
- Rust: `rustc 1.96.0`, Cargo `1.96.0`, LLVM `22.1.2`
- C compiler: Ubuntu GCC `13.3.0`
- Disk at baseline: 100 GiB available, repository `target/` 63 MiB

The implementation currently has:

- 117 `.lkjml` language files under `src`;
- line-oriented matched markers and raw `str/`, `name/`, and `import/` blocks;
- package-root imports for `std/`, `lib/`, `examples/`, and importer-relative `./`;
- six Rust workspace crates and no third-party Rust dependencies;
- a compiler, static type pass, dense bytecode VM, precise mark-sweep GC, and
  return-adjacent frame reuse;
- Linux-first filesystem, terminal, time, and IPv4 TCP primitives;
- hello, Mandelbrot, one-shot HTTP, benchmark, and lkjedit workloads;
- Docker packaging that bundles the standard library and in-tree packages.

Known current defects and incomplete boundaries include:

- `.lkjml` is still accepted while `.lkjscript` is rejected;
- `check-tree` applies an eight-visible-entry repository rule instead of the
  accepted 16-entry lkjscript source-tree rule;
- arbitrary script-controlled `sys-ioctl` reaches a safe Rust wrapper that does
  not validate the kernel structure size;
- ordinary OS failures from many `sys-*` operations abort the VM instead of
  producing language `ResultErr` values;
- `sys-send` reports zero rather than its successful byte count;
- handles conflate raw descriptors and reusable table indexes;
- the type prelude advertises numeric widths, conversions, and operators that
  code generation and runtime execution do not fully implement;
- `disasm` reports only summary counts, and the CLI contains an unadvertised,
  unimplemented `repl` branch;
- the JIT interface observes calls but cannot transfer execution to compiled
  code;
- native installation, self-update, packages, CI, releases, browser, GUI, and a
  general HTTP server/framework are absent.

## Baseline Evidence

Observed at the commit above:

| Command | Result |
| --- | --- |
| `cargo run --locked -p lkjscript-xtask --quiet -- quiet verify` | passed; 20 Rust tests passed |
| `cargo build --workspace --release --locked` | passed |
| hello workload | passed; output `3628800` |
| Mandelbrot workload | passed; 1,176 bytes, 24 lines |
| `meta/scripts/lkjedit-smoke.sh` | passed |
| `meta/scripts/http-smoke.sh` | passed |
| `cargo fmt --all -- --check` | failed on pre-existing formatting drift |
| strict workspace Clippy | failed on pre-existing production and test lint debt |

Docker and performance comparisons were not rerun for this baseline. They are
not claimed as passing.

## Accepted Target: Foundation Cutover

The active foundation cycle will:

1. make `.lkjscript` the only source extension and remove active LKJML naming;
2. enforce a maximum of 16 combined immediate files and subdirectories as an
   lkjscript source-tree rule, not a Rust/repository layout rule;
3. make documentation status and placeholder labeling machine-checkable;
4. remove the unlabeled REPL stub and make disassembly behavior truthful;
5. replace arbitrary ioctl access with bounded terminal operations;
6. separate opaque handles so stale values cannot alias later resources;
7. return truthful language Results from fallible system operations;
8. align the executable numeric surface with the type contract;
9. make formatting, Clippy, source coverage, and documentation checks honest
   repository gates.

Items move into **Current** only after their focused and acceptance gates run.

## Deferred

Package management, signed installation/update, the process supervisor,
nonblocking scheduling, adaptive/generational GC, native JIT execution,
non-Linux backends, a browser platform, GUI runtime, and web framework remain
later cycles. Their designs may be explored, but no current capability may be
implied.
