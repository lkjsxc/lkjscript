# lkjedit

## Purpose

Document the modal terminal editor used as a real in-tree runtime acceptance
application.

## Status

The described terminal workload is **Current**. Extraction to a separate
package/repository is **Deferred** until external package resolution and native
distribution exist.

## Product Boundary

`lkjedit` is not the primary runtime product. It exercises terminal,
filesystem, arguments, strings, lists, polling, timing, mutation, and
long-running control flow without editor-specific host opcodes.

Source currently lives under `src/lib/lkjedit/`, with runnable entries under
`src/examples/lkjedit/`. The VM/compiler must never depend on lkjedit. The
accepted extension cutover renames these sources to `.lkjscript` with no
`.lkjml` compatibility.

## Current Scope

- Normal, Insert, and command-line modes
- Motions `hjkl`, `0`, `$`, `w`, and `b`
- Edits `i`, `a`, `x`, `dd`, `yy`, `p`, `o`, and `O`
- Commands `:w`, `:q`, `:q!`, and `:wq`
- Viewport scrolling, line-number gutter, status, path, dirty state, and message
- One CLI path; missing files open as an empty buffer and are created on write
- Language `while` plus polling; idle waits without full repaint
- Cursor hiding, repaint, final placement, and flush

Windows, visual mode, search, undo trees, plugins, Unicode display width,
terminal resize, and atomic save are outside current validation scope.

## Current Run Commands

```sh
cargo run --locked -p lkjscript-app -- run src/examples/lkjedit/main.lkjml path/to/file
LKJ=target/debug/lkjscript meta/scripts/lkjedit-smoke.sh
```

After the source cutover the entry ends in `.lkjscript`.

## Host Surface

- terminal buffer and polling primitives;
- script-level raw-mode and key policy under `src/std`;
- monotonic time and waits;
- bulk terminal output and flush;
- filesystem open/read/write/path existence;
- arguments, strings, lists, buffers, and bit operations.

The current arbitrary ioctl and ambiguous handle surfaces are known foundation
defects, not accepted editor dependencies. lkjedit must migrate to bounded
terminal operations and stale-safe handles with the runtime.
