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

- Terminal: `term-raw` / `term-cooked` / `poll-byte` / `write-byte` / `write-str` / `flush`
- Wait: `wait-ms` / `now-ms`
- Files: `open-read` / `open-write` / `close` / `read-byte-fd` / `write-byte-fd`
- Args: `argc` / `arg`
- Strings: `str-len` / `str-ref` / `str-append` / `str-slice` / `str-from-byte`
