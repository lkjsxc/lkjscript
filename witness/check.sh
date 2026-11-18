#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ ! -x target/lkjscript ]]; then sh tools/bootstrap.sh; fi
export CARGO_TARGET_DIR="$PWD/target/witness"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
if [[ ! "$CARGO_BUILD_JOBS" =~ ^[1-4]$ ]]; then
    echo 'CARGO_BUILD_JOBS must be between one and four' >&2
    exit 1
fi
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS="${CARGO_PROFILE_RELEASE_CODEGEN_UNITS:-1}"
cargo build --manifest-path witness/Cargo.toml --release --quiet
mkdir -p target/witness
# Match the compiler producer policy in tools/bootstrap.sh.
"$CARGO_TARGET_DIR/release/lkjscript" build src/main.lkj --library-dir library --runtime-source library/runtime.lkj --inline-limit 64 -o target/witness/compiler-output
cmp target/lkjscript target/witness/compiler-output
rm target/witness/compiler-output
if command -v rustup >/dev/null 2>&1; then
    witness_rustc=$(rustup which rustc)
else
    witness_rustc=$(command -v rustc)
fi
target/lkjscript test tests/conformance --witness "$witness_rustc"
bash tools/directory-rename-new-test.sh target/lkjscript --witness "$witness_rustc"
bash tools/directory-identity-test.sh target/lkjscript --witness "$witness_rustc"
bash tools/connect-deadline-test.sh target/lkjscript --witness "$witness_rustc"
bash tools/real-programs-rust.sh "$witness_rustc"
printf 'ok optional Rust witness and compiler byte identity\n'
