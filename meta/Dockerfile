# syntax=docker/dockerfile:1.7
FROM rust:bookworm AS build

WORKDIR /src
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy
COPY .gitignore Cargo.toml Cargo.lock README.md AGENTS.md lkjscript.package.json lkjscript.lock.json ./
COPY crates ./crates
COPY meta/config ./meta/config
COPY meta/platform-revision ./meta/platform-revision
COPY meta/agent-state ./meta/agent-state
COPY meta/unsafe ./meta/unsafe
COPY docs ./docs
COPY src ./src
COPY meta/scripts ./meta/scripts
COPY meta/benchmarks ./meta/benchmarks
COPY meta/Dockerfile meta/docker-compose.yml ./meta/
RUN --mount=type=cache,id=lkjscript-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=lkjscript-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=lkjscript-target-release,target=/src/target,sharing=locked \
    cargo build --locked --release -p lkjscript-app -p lkjscript-xtask \
    && cp /src/target/release/lkjscript /tmp/lkjscript \
    && cp /src/target/release/lkjscript-xtask /tmp/lkjscript-xtask

FROM build AS verified
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*
RUN --mount=type=cache,id=lkjscript-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=lkjscript-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=lkjscript-target-verify,target=/src/target,sharing=locked \
    --mount=type=bind,from=repository-git,source=.,target=/src/.git,readonly \
    git config --global --add safe.directory /src \
    && cargo run --locked -p lkjscript-xtask -- quiet verify \
    && /tmp/lkjscript run src/examples/hello/main.lkjscript | grep -q 3628800 \
    && /tmp/lkjscript run src/examples/mandel/main.lkjscript > /tmp/mandel.out \
    && test -s /tmp/mandel.out \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/lkjedit-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/http-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/bulk-bytes-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/durable-files-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/sha256-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/sqlite-smoke.sh \
    && printf 'result=ok\n' > /tmp/lkjscript-verification-result

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /tmp/lkjscript /usr/local/bin/lkjscript
ENV LKJSCRIPT_ROOT=/usr/local/share/lkjscript
COPY src /usr/local/share/lkjscript/src
WORKDIR /usr/local/share/lkjscript
COPY meta/config ./meta/config
COPY lkjscript.package.json lkjscript.lock.json ./
ENTRYPOINT ["lkjscript"]
CMD ["--help"]
