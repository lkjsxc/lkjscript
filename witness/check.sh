#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ ! -x target/lkjscript ]]; then sh tools/bootstrap.sh; fi
export CARGO_TARGET_DIR="$PWD/target/witness"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-8}"
if [[ ! "$CARGO_BUILD_JOBS" =~ ^[1-8]$ ]]; then
    echo 'CARGO_BUILD_JOBS must be between one and eight' >&2
    exit 1
fi
cargo build --manifest-path witness/Cargo.toml --release --quiet
mkdir -p target/witness
"$CARGO_TARGET_DIR/release/lkjscript" build src/main.rs --library-dir library --runtime-source library/runtime.rs -o target/witness/compiler-output
cmp target/lkjscript target/witness/compiler-output
rm target/witness/compiler-output
if command -v rustup >/dev/null 2>&1; then
    witness_rustc=$(rustup which rustc)
else
    witness_rustc=$(command -v rustc)
fi
target/lkjscript test tests/conformance --witness "$witness_rustc"
printf 'ok optional Rust witness and compiler byte identity\n'
