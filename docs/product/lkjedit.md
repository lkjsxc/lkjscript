# lkjedit

## Purpose

Document the modal terminal editor used as a real in-tree runtime acceptance
application.

## Status

The terminal workload is **Current**. Extraction to an external package or
repository is **Deferred** until external package resolution and native
distribution exist.

## Product Boundary

`lkjedit` is not the primary runtime product. It exercises terminal,
filesystem, arguments, strings, lists, polling, timing, mutation, and
long-running control flow without editor-specific host opcodes.

Source lives under `src/lib/lkjedit/`, with runnable entries under
`src/examples/lkjedit/`. Every source uses `.lkjscript`. The VM and compiler do
not depend on lkjedit.

## Current Scope

- Normal, Insert, and command-line modes
- Motions `hjkl`, `0`, `$`, `w`, and `b`
- Edits `i`, `a`, `x`, `dd`, `yy`, `p`, `o`, and `O`
- Commands `:w`, `:q`, `:q!`, and `:wq`
- Viewport scrolling, line-number gutter, status, path, dirty state, and message
- One CLI path; missing files open empty and are created on write
- Language `while` plus polling; idle waits without full repaint
- Cursor hiding, repaint, final placement, and flush

Windows, visual mode, search, undo trees, plugins, Unicode display width,
terminal resize, and atomic save are outside current validation scope.

## Run

```sh
cargo run --locked -p lkjscript-app -- run src/examples/lkjedit/main.lkjscript path/to/file
LKJSCRIPT_BIN=target/debug/lkjscript meta/scripts/lkjedit-smoke.sh
```

## Host Surface

- terminal buffer and polling primitives;
- script-level raw-mode and key policy under `src/std`;
- monotonic time and waits;
- bulk terminal output and flush;
- filesystem open/read/write/path existence;
- arguments, strings, lists, buffers, and bit operations.

Arbitrary ioctl and ambiguous handles remain known foundation defects, not
accepted editor architecture. lkjedit will migrate with their bounded,
stale-safe replacements.
