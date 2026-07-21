# Verification

## Purpose

Define how completion is checked.

## Local

```sh
cargo run -p lkjscript2026-xtask -- check-docs
cargo run -p lkjscript2026-xtask -- check-tree
cargo run -p lkjscript2026-xtask -- quiet test
cargo run -p lkjscript2026-xtask -- quiet verify
LKJ=target/debug/lkjscript2026 meta/scripts/editor-smoke.sh
LKJ=target/debug/lkjscript2026 meta/scripts/http-smoke.sh
N=5000 meta/scripts/bench-compare.sh
```

## Docker

```sh
docker compose -f meta/docker-compose.yml --profile verify run --rm verify
```

Interactive editor:

```sh
docker compose -f meta/docker-compose.yml run -it --rm texteditor \
  run examples/texteditor/main.lkjscript /tmp/file
```

One-shot HTTP hello (port 8080):

```sh
docker compose -f meta/docker-compose.yml run --rm -p 8080:8080 http
```

The verify image runs `quiet verify`, hello/mandel demos, editor-smoke, and
http-smoke.

A gate that did not run did not pass.
