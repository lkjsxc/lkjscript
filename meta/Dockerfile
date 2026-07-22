# syntax=docker/dockerfile:1.7
FROM rust:bookworm AS build

WORKDIR /src
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy
COPY Cargo.toml Cargo.lock README.md ./
COPY crates ./crates
COPY meta/config ./meta/config
COPY docs ./docs
COPY examples ./examples
COPY src ./src
COPY meta/scripts ./meta/scripts
COPY meta/Dockerfile meta/docker-compose.yml ./meta/
RUN --mount=type=cache,id=lkjscript2026-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=lkjscript2026-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=lkjscript2026-target-release,target=/src/target,sharing=locked \
    cargo build --locked --release -p lkjscript2026-app -p lkjscript2026-xtask \
    && cp /src/target/release/lkjscript2026 /tmp/lkjscript2026 \
    && cp /src/target/release/lkjscript2026-xtask /tmp/lkjscript2026-xtask

FROM build AS verified
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*
RUN --mount=type=cache,id=lkjscript2026-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=lkjscript2026-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=lkjscript2026-target-verify,target=/src/target,sharing=locked \
    cargo run --locked -p lkjscript2026-xtask -- quiet verify \
    && /tmp/lkjscript2026 run examples/hello/main.lkjml | grep -q 3628800 \
    && /tmp/lkjscript2026 run examples/mandel/main.lkjml > /tmp/mandel.out \
    && test -s /tmp/mandel.out \
    && LKJ=/tmp/lkjscript2026 bash meta/scripts/editor-smoke.sh \
    && LKJ=/tmp/lkjscript2026 bash meta/scripts/http-smoke.sh \
    && printf 'result=ok\n' > /tmp/lkjscript2026-verification-result

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /tmp/lkjscript2026 /usr/local/bin/lkjscript2026
ENV LKJSCRIPT2026_ROOT=/usr/local/share/lkjscript2026
COPY src /usr/local/share/lkjscript2026/src
WORKDIR /work
COPY examples ./examples
COPY meta/config ./meta/config
ENTRYPOINT ["lkjscript2026"]
CMD ["--help"]
