# lkjedit

## Purpose

Document `lkjedit`, the modal editor used as an in-tree runtime validation
application before its planned extraction into a separate repository.

## Product Boundary

`lkjedit` is not the primary `lkjscript` product. It exercises terminal,
filesystem, arguments, strings, lists, polling, timing, and long-running LKJML
control flow as a real application rather than a mock fixture.

Its source temporarily lives under `src/lib/lkjedit/`, with runnable entries
under `src/examples/lkjedit/`. The runtime must not depend on `lkjedit`, and new
editor behavior must not justify editor-specific host opcodes. Keeping that
boundary clean is the extraction contract for a future standalone `lkjedit`
repository.

Extraction should happen after package resolution and native distribution can
consume an external LKJML package without copying repository internals. Until
then, the in-tree copy remains part of runtime acceptance verification.

## Scope

- Modes: Normal / Insert / command-line (`:`)
- Motions: `hjkl`, `0`, `$`, `w`, `b`
- Edits: `i` / `a` / `x` / `dd` / `yy` / `p` / `o` / `O`
- Commands: `:w` `:q` `:q!` `:wq` (cmdline shows typed text; backspace works)
- Viewport scroll, line-number gutter, status (mode, path, dirty, message)
- Open one path from CLI argv (`arg` / `argc`); missing paths open as a new
  empty buffer with status `new file` (created on `:w`)
- Key loop via language `while` + `poll-byte`; idle waits without full redraw
- Redraw hides cursor, paints, places cursor (buffer or cmdline), then `flush`

It is not a complete editor: windows, visual mode, search, undo trees, plugins,
Unicode display width, terminal resize, and atomic save remain outside the
current validation scope.

## Run

```sh
cargo run -p lkjscript-app -- run src/examples/lkjedit/main.lkjml path/to/file
docker compose -f meta/docker-compose.yml run -it --rm lkjedit \
  run src/examples/lkjedit/main.lkjml path/to/file
```

Scripted acceptance:

```sh
meta/scripts/lkjedit-smoke.sh
```

## Host Surface Used

- Thin sys: `sys-ioctl` / `sys-poll` / `stdin-fd` / `isatty` / `buf-*` / tty-guard
- Script term: `enter-raw` / `leave-raw` / `poll-byte` under `src/std`
- Wait: script `wait-ms` / `now-ms` on thin `sys-wait-ms` / `sys-now-ms`
- Output: `write-byte` / `write-str` / `flush`
- Files: script `open-read` / `open-write` / `path-exists`; thin
  `close` / `read-byte-fd` / `write-byte-fd` (sys-backed)
- Args: `argc` / `arg`
- Strings: `str-len` / `str-ref` / `str-append` / `str-slice` / `str-from-byte`
- Bits: `bit-and` / `bit-or` / `bit-xor`
