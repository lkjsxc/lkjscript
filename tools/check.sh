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
step rust cargo test --quiet
step lint cargo clippy --quiet --all-targets -- -D warnings
step build cargo build --quiet
check_binary="${CARGO_TARGET_DIR:-target}/debug/lkjscript"
# Independent processes have separate case directories. Batching amortizes each
# compiler/engine startup; parallel suites use the host's available cores.
mkdir -p target/gate
check_pids=()
check_logs=()
parallel_step() {
    local log="target/gate/$1.log"
    (step "$@") >"$log" 2>&1 &
    check_pids+=("$!")
    check_logs+=("$log")
}
parallel_started_ns=$(date +%s%N)
check_engines_text="${LKJ_GATE_ENGINES:-native,k}"
IFS=, read -r -a check_engines <<< "$check_engines_text"
for engine in "${check_engines[@]}"; do
    parallel_step "conformance-$engine" env LKJ_ENGINE="$engine" node tools/conformance.mjs "$check_binary"
    parallel_step "values-$engine" env LKJ_ENGINE="$engine" node tools/generators/value.mjs "$check_binary" 601 100 "target/generators/$engine"
    parallel_step "verdicts-$engine" env LKJ_ENGINE="$engine" node tools/generators/verdict.mjs "$check_binary" 601 100 "target/generators/$engine"
done
parallel_step output env LKJ_ENGINES="$check_engines_text" node tools/generators/output.mjs "$check_binary" 601 50
parallel_step drops env LKJ_ENGINES="$check_engines_text" ONE_SIDED=1 node tools/generators/drops.mjs "$check_binary" 1201 100
parallel_step sugars env LKJ_ENGINES="$check_engines_text" node tools/generators/sugar.mjs "$check_binary" 1301 75
parallel_step closures env LKJ_ENGINES="$check_engines_text" node tools/generators/closures.mjs "$check_binary" 1501 28
check_failed=0
for i in "${!check_pids[@]}"; do
    if ! wait "${check_pids[$i]}"; then check_failed=1; fi
    cat "${check_logs[$i]}"
done
parallel_elapsed_ns=$(($(date +%s%N)-parallel_started_ns))
awk -v elapsed_ns="$parallel_elapsed_ns" 'BEGIN { printf "Gate parallel suites: %.3f s\n", elapsed_ns / 1000000000 }'
if ((check_failed)); then exit 1; fi
check_elapsed_ns=$(($(date +%s%N)-check_started_ns))
awk -v elapsed_ns="$check_elapsed_ns" 'BEGIN { printf "Full check: %.3f s\n", elapsed_ns / 1000000000 }'
