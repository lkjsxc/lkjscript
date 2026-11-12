#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
check_started_ns=$(date +%s%N)
cargo fmt --check
node --test tests/worker_host.mjs tools/generators/reduce.test.mjs
cargo test --quiet
cargo clippy --quiet --all-targets -- -D warnings
cargo build --quiet
for engine in node k; do
    LKJ_ENGINE="$engine" node tools/conformance.mjs "${CARGO_TARGET_DIR:-target}/debug/lkjscript"
    LKJ_ENGINE="$engine" node tools/generators/value.mjs "${CARGO_TARGET_DIR:-target}/debug/lkjscript" 601 100 "target/generators/$engine"
    LKJ_ENGINE="$engine" node tools/generators/verdict.mjs "${CARGO_TARGET_DIR:-target}/debug/lkjscript" 601 100 "target/generators/$engine"
done
node tools/generators/output.mjs "${CARGO_TARGET_DIR:-target}/debug/lkjscript" 601 50
check_elapsed_ns=$(($(date +%s%N)-check_started_ns))
awk -v elapsed_ns="$check_elapsed_ns" 'BEGIN { printf "Full check: %.3f s\n", elapsed_ns / 1000000000 }'
