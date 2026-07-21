# Vim-like editor

## Purpose

Document the modal single-buffer editor shipped as `.lkjscript` under `src/lib/edit/`.

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

Not full vim: no windows, visual mode, search, undo tree, or plugins.

## Run

```sh
cargo run -p lkjscript2026-app -- run examples/texteditor/main.lkjscript path/to/file
docker compose -f meta/docker-compose.yml run -it --rm lkjscript2026 \
  run examples/texteditor/main.lkjscript path/to/file
```

Scripted acceptance:

```sh
meta/scripts/editor-smoke.sh
```

## Host surface used

- Thin sys: `sys-ioctl` / `sys-poll` / `stdin-fd` / `isatty` / `buf-*` / tty-guard
- Script term: `enter-raw` / `leave-raw` / `poll-byte` under `src/std`
- Wait: `wait-ms` / `now-ms`
- Output: `write-byte` / `write-str` / `flush`
- Files: script `open-read` / `open-write` / `path-exists`; thin
  `close` / `read-byte-fd` / `write-byte-fd` (sys-backed)
- Args: `argc` / `arg`
- Strings: `str-len` / `str-ref` / `str-append` / `str-slice` / `str-from-byte`
- Bits: `bit-and` / `bit-or` / `bit-xor`
