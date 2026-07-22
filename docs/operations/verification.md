# Verification

## Purpose

Define how completion is checked.

## Local

```sh
cargo run -p lkjscript2026-xtask -- check-docs
cargo run -p lkjscript2026-xtask -- check-tree
cargo run -p lkjscript2026-xtask -- check-sources
cargo run -p lkjscript2026-xtask -- quiet test
cargo run -p lkjscript2026-xtask -- quiet verify
LKJ=target/debug/lkjscript2026 meta/scripts/editor-smoke.sh
LKJ=target/debug/lkjscript2026 meta/scripts/http-smoke.sh
N=5000 meta/scripts/bench-compare.sh
```

## Docker

Build and run the full acceptance image from the current checkout:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run an arbitrary project against the bundled standard library:

```sh
docker build -f meta/Dockerfile --target runtime -t lkjscript2026 . && \
  docker run --rm -it -v "$PWD:/project" -w /project lkjscript2026 \
  run main.lkjml
```

Interactive editor:

```sh
docker compose -f meta/docker-compose.yml run -it --rm texteditor \
  run examples/texteditor/main.lkjml /tmp/file
```

One-shot HTTP hello (port 8080):

```sh
docker compose -f meta/docker-compose.yml run --rm -p 8080:8080 http
```

`quiet verify` checks required docs, directory fan-out, every `.lkjml` file,
absence of legacy `.lkjscript` source, and Rust unit tests. The verify image
also runs hello/mandel demos, editor-smoke, and http-smoke.

A gate that did not run did not pass.
