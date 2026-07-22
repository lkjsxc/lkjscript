# lkjscript2026

## Purpose

`lkjscript2026` is a typed functional language with the line-oriented,
attribute-less LKJML surface, a dense bytecode VM, fixed AI-friendly source
budgets, and Docker verification. Canonical sources use `.lkjml`.

## Product Shape

- LKJML places each marker or atom on its own column-one line, uses no
  attributes or indentation, and represents text without quote delimiters.
- Nest, child, token, toplevel-form, and directory fan-out limits are
  **hardcoded language constants** (see [docs/language/limits.md](docs/language/limits.md)).
- Host builtins stay minimal; primitives live in `src/std`, reusable packages
  in `src/lib` (editor: `lib/edit/...`), demos under `examples/`.
- A `JitHook` stub is wired for later native hot-path work
  ([performance roadmap](docs/vision/performance-roadmap.md)).

## Common Commands

```sh
cargo run -p lkjscript2026-app -- run examples/hello/main.lkjml
cargo run -p lkjscript2026-app -- run examples/mandel/main.lkjml
cargo run -p lkjscript2026-app -- run examples/texteditor/main.lkjml path/to/file
cargo run -p lkjscript2026-app -- run examples/http/hello.lkjml
cargo run -p lkjscript2026-xtask -- quiet verify
N=20000 meta/scripts/bench-compare.sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Arbitrary project in Docker (the image supplies `src/std` and `src/lib`):

```sh
docker build -f meta/Dockerfile --target runtime -t lkjscript2026 . && \
  docker run --rm -it -v "$PWD:/project" -w /project lkjscript2026 \
  run main.lkjml
```

Agent notes: [meta/AGENTS.md](meta/AGENTS.md)

## Read Order

1. [docs/current-state.md](docs/current-state.md)
2. [docs/vision/README.md](docs/vision/README.md)
3. [docs/language/README.md](docs/language/README.md)
4. [docs/runtime/README.md](docs/runtime/README.md)
5. [docs/operations/README.md](docs/operations/README.md)

## Repository Rules

- Documentation is the implementation contract.
- File size is gated by token and top-level form budgets, not line count.
- At most eight visible children per directory (`check-tree`).
- No fake success or unrun verification claims.
