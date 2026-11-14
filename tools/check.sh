#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
check_started_ns=$(date +%s%N)
check_jobs="${LKJ_GATE_JOBS:-$(node -p 'require("node:os").availableParallelism()')}"
if [[ ! "$check_jobs" =~ ^[1-9][0-9]*$ ]]; then
    echo 'LKJ_GATE_JOBS must be a positive integer' >&2
    exit 1
fi
check_default_jobs=$((check_jobs < 4 ? check_jobs : 4))
check_mutation_default=$((check_jobs < 2 ? check_jobs : 2))
export LKJ_CONFORMANCE_JOBS="${LKJ_CONFORMANCE_JOBS:-$check_default_jobs}"
export LKJ_KC_JOBS="${LKJ_KC_JOBS:-$check_default_jobs}"
export LKJ_K_MUTATION_JOBS="${LKJ_K_MUTATION_JOBS:-$check_mutation_default}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$check_default_jobs}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-$check_default_jobs}"
for check_setting in LKJ_CONFORMANCE_JOBS LKJ_KC_JOBS LKJ_K_MUTATION_JOBS CARGO_BUILD_JOBS RUST_TEST_THREADS; do
    check_value="${!check_setting}"
    if [[ ! "$check_value" =~ ^[1-9][0-9]*$ ]] || ((check_value > check_jobs)); then
        echo "$check_setting must be a positive integer at most LKJ_GATE_JOBS=$check_jobs" >&2
        exit 1
    fi
done
check_rust_jobs=$((CARGO_BUILD_JOBS > RUST_TEST_THREADS ? CARGO_BUILD_JOBS : RUST_TEST_THREADS))
echo "Gate workers: total=$check_jobs conformance=$LKJ_CONFORMANCE_JOBS kc=$LKJ_KC_JOBS mutations=$LKJ_K_MUTATION_JOBS rust=$check_rust_jobs"
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
step host node --test --test-concurrency=1 tools/batch.test.mjs tools/generators/reduce.test.mjs tools/generators/k_mutate.test.mjs
step build cargo build --release --quiet
rust_checks() {
    step rust cargo test --quiet
    step lint cargo clippy --quiet --all-targets -- -D warnings
}
check_binary="${CARGO_TARGET_DIR:-target}/release/lkjscript"
step host-boundary bash tools/host-boundary.sh "$check_binary"
# Reservations count active execution workers, including workers inside a suite.
# Rust compilation and tests run in sequence, so their reservation is the larger
# of their two explicit thread counts. Each generator outside K mutation uses
# one batch worker. A reservation is released only after its suite exits.
mkdir -p target/gate
check_pids=()
check_logs=()
check_weights=()
check_waited=0
check_active=0
check_failed=0
wait_oldest() {
    if ! wait "${check_pids[$check_waited]}"; then check_failed=1; fi
    check_active=$((check_active - check_weights[check_waited]))
    check_waited=$((check_waited + 1))
}
parallel_step() {
    local weight=$1
    shift
    while ((check_active + weight > check_jobs)); do wait_oldest; done
    local log="target/gate/$1.log"
    (step "$@") >"$log" 2>&1 &
    check_pids+=("$!")
    check_logs+=("$log")
    check_weights+=("$weight")
    check_active=$((check_active + weight))
}
parallel_started_ns=$(date +%s%N)
check_engines_text="${LKJ_GATE_ENGINES:-native,k}"
parallel_step "$check_rust_jobs" rust-and-lint rust_checks
parallel_step 1 world-io node tools/world-io.mjs "$check_binary"
parallel_step 1 native-workflow bash tools/native-workflow.sh "$check_binary"
parallel_step "$LKJ_KC_JOBS" k-compiler node tools/kc.mjs "$check_binary" "${CARGO_TARGET_DIR:-target}/release/kc"
parallel_step "$LKJ_CONFORMANCE_JOBS" conformance env LKJ_ENGINES="$check_engines_text" node tools/conformance.mjs "$check_binary"
parallel_step "$LKJ_K_MUTATION_JOBS" k-mutation node tools/generators/k_mutate.mjs "$check_binary" 1701 128
parallel_step 1 values env LKJ_ENGINES="$check_engines_text" node tools/generators/value.mjs "$check_binary" 601 100
parallel_step 1 verdicts env LKJ_ENGINES="$check_engines_text" node tools/generators/verdict.mjs "$check_binary" 601 100
parallel_step 1 output env LKJ_ENGINES="$check_engines_text" node tools/generators/output.mjs "$check_binary" 601 50
parallel_step 1 drops env LKJ_ENGINES="$check_engines_text" ONE_SIDED=1 node tools/generators/drops.mjs "$check_binary" 1201 100
parallel_step 1 sugars env LKJ_ENGINES="$check_engines_text" node tools/generators/sugar.mjs "$check_binary" 1301 216
parallel_step 1 closures env LKJ_ENGINES="$check_engines_text" node tools/generators/closures.mjs "$check_binary" 1501 44
while ((check_waited < ${#check_pids[@]})); do wait_oldest; done
for log in "${check_logs[@]}"; do cat "$log"; done
parallel_elapsed_ns=$(($(date +%s%N)-parallel_started_ns))
awk -v elapsed_ns="$parallel_elapsed_ns" 'BEGIN { printf "Gate parallel suites: %.3f s\n", elapsed_ns / 1000000000 }'
if ((check_failed)); then exit 1; fi
check_elapsed_ns=$(($(date +%s%N)-check_started_ns))
awk -v elapsed_ns="$check_elapsed_ns" 'BEGIN { printf "Full check: %.3f s\n", elapsed_ns / 1000000000 }'
