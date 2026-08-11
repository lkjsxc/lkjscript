# syntax=docker/dockerfile:1.7
FROM rust:bookworm AS build

WORKDIR /src
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# COPY preserves host mtimes, while cache mounts retain newer Cargo artifacts across builds.
# Refresh every Cargo input whenever this source layer changes so stale test binaries cannot pass.
RUN --mount=type=cache,id=lkjscript-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=lkjscript-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=lkjscript-target-release,target=/src/target,sharing=locked \
    find Cargo.toml Cargo.lock crates -type f -exec touch {} + \
    && cargo build --quiet --workspace --release --locked \
    && cp /src/target/release/lkjscript /tmp/lkjscript

FROM build AS verified
COPY lkjscript.package.json lkjscript.lock.json ./
COPY src ./src
COPY meta/bench ./meta/bench
COPY meta/scripts ./meta/scripts
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*
RUN --mount=type=cache,id=lkjscript-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=lkjscript-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=lkjscript-target-verify,target=/src/target,sharing=locked \
    cargo fmt --all -- --check \
    && cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings \
    && cargo test --quiet --workspace --all-targets --all-features --locked \
    && /tmp/lkjscript check src/examples/hello/main.lkjscript > /tmp/check.out \
    && test ! -s /tmp/check.out \
    && /tmp/lkjscript run src/examples/hello/main.lkjscript | grep -q 3628800 \
    && /tmp/lkjscript run src/examples/mandel/main.lkjscript > /tmp/mandel.out \
    && test -s /tmp/mandel.out \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/lkjedit-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/http-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/bulk-bytes-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/durable-files-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/sha256-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/sqlite-smoke.sh \
    && LKJSCRIPT_BIN=/tmp/lkjscript bash meta/scripts/bench-compare.sh \
    && printf 'result=ok\n' > /tmp/lkjscript-verification-result

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /tmp/lkjscript /usr/local/bin/lkjscript
ENV LKJSCRIPT_ROOT=/usr/local/share/lkjscript
COPY src /usr/local/share/lkjscript/src
WORKDIR /usr/local/share/lkjscript
COPY lkjscript.package.json lkjscript.lock.json ./
ENTRYPOINT ["lkjscript"]
CMD ["--help"]
