#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
check_started_ns=$(date +%s%N)
step() {
    local label=$1
    shift
    local started_ns
    started_ns=$(date +%s%N)
    "$@"
    local elapsed_ns=$(($(date +%s%N)-started_ns))
    awk -v label="$label" -v elapsed_ns="$elapsed_ns" 'BEGIN { printf "Gate %s: %.3f s\n", label, elapsed_ns / 1000000000 }'
}
step format cargo fmt --check
step host node --test tools/generators/reduce.test.mjs
step build cargo build --release --quiet
rust_checks() {
    step rust cargo test --quiet
    step lint cargo clippy --quiet --all-targets -- -D warnings
}
check_binary="${CARGO_TARGET_DIR:-target}/release/lkjscript"
# The optimized compiler runs generated suites; release arithmetic checks stay
# enabled. Debug Rust tests still validate compiler internals. Build once, then
# overlap those tests/lint with independent suites. Each source is checked once
# for all routes; jobs have separate paths and share available execution slots.
mkdir -p target/gate
check_pids=()
check_logs=()
check_waited=0
check_failed=0
check_jobs="${LKJ_GATE_JOBS:-$(node -p 'require("node:os").availableParallelism()')}"
if [[ ! "$check_jobs" =~ ^[1-9][0-9]*$ ]]; then
    echo 'LKJ_GATE_JOBS must be a positive integer' >&2
    exit 1
fi
wait_oldest() {
    if ! wait "${check_pids[$check_waited]}"; then check_failed=1; fi
    check_waited=$((check_waited + 1))
}
parallel_step() {
    if ((${#check_pids[@]} - check_waited >= check_jobs)); then wait_oldest; fi
    local log="target/gate/$1.log"
    (step "$@") >"$log" 2>&1 &
    check_pids+=("$!")
    check_logs+=("$log")
}
parallel_started_ns=$(date +%s%N)
check_engines_text="${LKJ_GATE_ENGINES:-native,k}"
parallel_step rust-and-lint rust_checks
parallel_step conformance env LKJ_ENGINES="$check_engines_text" node tools/conformance.mjs "$check_binary"
parallel_step values env LKJ_ENGINES="$check_engines_text" node tools/generators/value.mjs "$check_binary" 601 100
parallel_step verdicts env LKJ_ENGINES="$check_engines_text" node tools/generators/verdict.mjs "$check_binary" 601 100
parallel_step output env LKJ_ENGINES="$check_engines_text" node tools/generators/output.mjs "$check_binary" 601 50
parallel_step drops env LKJ_ENGINES="$check_engines_text" ONE_SIDED=1 node tools/generators/drops.mjs "$check_binary" 1201 100
parallel_step sugars env LKJ_ENGINES="$check_engines_text" node tools/generators/sugar.mjs "$check_binary" 1301 126
parallel_step closures env LKJ_ENGINES="$check_engines_text" node tools/generators/closures.mjs "$check_binary" 1501 36
while ((check_waited < ${#check_pids[@]})); do wait_oldest; done
for log in "${check_logs[@]}"; do cat "$log"; done
parallel_elapsed_ns=$(($(date +%s%N)-parallel_started_ns))
awk -v elapsed_ns="$parallel_elapsed_ns" 'BEGIN { printf "Gate parallel suites: %.3f s\n", elapsed_ns / 1000000000 }'
if ((check_failed)); then exit 1; fi
check_elapsed_ns=$(($(date +%s%N)-check_started_ns))
awk -v elapsed_ns="$check_elapsed_ns" 'BEGIN { printf "Full check: %.3f s\n", elapsed_ns / 1000000000 }'
