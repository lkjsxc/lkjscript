#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
check_started_ns=$(date +%s%N)
check_jobs="${LKJSCRIPT_GATE_JOBS:-8}"
if [[ ! "$check_jobs" =~ ^[1-9][0-9]*$ ]]; then
    echo 'LKJSCRIPT_GATE_JOBS must be a positive integer' >&2
    exit 1
fi
check_default_jobs=$((check_jobs < 4 ? check_jobs : 4))
check_mutation_default=$((check_jobs < 2 ? check_jobs : 2))
export LKJSCRIPT_CONFORMANCE_JOBS="${LKJSCRIPT_CONFORMANCE_JOBS:-$check_default_jobs}"
export LKJSCRIPT_BACKEND_JOBS="${LKJSCRIPT_BACKEND_JOBS:-$check_default_jobs}"
export LKJSCRIPT_IR_MUTATION_JOBS="${LKJSCRIPT_IR_MUTATION_JOBS:-$check_mutation_default}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$check_default_jobs}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-$check_default_jobs}"
for check_setting in LKJSCRIPT_CONFORMANCE_JOBS LKJSCRIPT_BACKEND_JOBS LKJSCRIPT_IR_MUTATION_JOBS CARGO_BUILD_JOBS RUST_TEST_THREADS; do
    check_value="${!check_setting}"
    if [[ ! "$check_value" =~ ^[1-9][0-9]*$ ]] || ((check_value > check_jobs)); then
        echo "$check_setting must be a positive integer at most LKJSCRIPT_GATE_JOBS=$check_jobs" >&2
        exit 1
    fi
done
check_rust_jobs=$((CARGO_BUILD_JOBS > RUST_TEST_THREADS ? CARGO_BUILD_JOBS : RUST_TEST_THREADS))
echo "Gate workers: total=$check_jobs conformance=$LKJSCRIPT_CONFORMANCE_JOBS backend=$LKJSCRIPT_BACKEND_JOBS mutations=$LKJSCRIPT_IR_MUTATION_JOBS rust=$check_rust_jobs"
step() {
    local label=$1
    shift
    local started_ns
    started_ns=$(date +%s%N)
    mkdir -p target/gate
    local details="target/gate/details-$label.log"
    local result=0
    "$@" >"$details" 2>&1 || result=$?
    if ((result)); then
        cat "$details" >&2
        return "$result"
    fi
    local elapsed_ns=$(($(date +%s%N)-started_ns))
    awk -v label="$label" -v elapsed_ns="$elapsed_ns" 'BEGIN { printf "ok %s (%.3f s)\n", label, elapsed_ns / 1000000000 }'
}
step format cargo fmt --check
step host node --test --test-concurrency=1 tools/batch.test.mjs tools/generators/reduce.test.mjs tools/generators/ir_mutate.test.mjs
step build cargo build --release --quiet
check_binary="${CARGO_TARGET_DIR:-target}/release/lkjscript"
# Account for workers inside each suite. Rust build and test phases are sequential;
# their worker count is the larger of the two. Other generators use one worker.
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
check_engines_text="${LKJSCRIPT_GATE_ENGINES:-native,interpreter}"
parallel_step "$check_rust_jobs" rust cargo test --quiet
parallel_step 1 world-io node tools/world-io.mjs "$check_binary"
parallel_step 1 native-workflow bash tools/native-workflow.sh "$check_binary"
parallel_step "$LKJSCRIPT_BACKEND_JOBS" backend node tools/backend.mjs "$check_binary" "${CARGO_TARGET_DIR:-target}/release/lkjscript-backend"
parallel_step "$LKJSCRIPT_CONFORMANCE_JOBS" conformance env LKJSCRIPT_ENGINES="$check_engines_text" node tools/conformance.mjs "$check_binary"
parallel_step "$LKJSCRIPT_IR_MUTATION_JOBS" ir-mutation node tools/generators/ir_mutate.mjs "$check_binary" 1701 128
parallel_step 1 values env LKJSCRIPT_ENGINES="$check_engines_text" node tools/generators/value.mjs "$check_binary" 601 100
parallel_step 1 verdicts env LKJSCRIPT_ENGINES="$check_engines_text" node tools/generators/verdict.mjs "$check_binary" 601 100
parallel_step 1 output env LKJSCRIPT_ENGINES="$check_engines_text" node tools/generators/output.mjs "$check_binary" 601 50
parallel_step 1 drops env LKJSCRIPT_ENGINES="$check_engines_text" ONE_SIDED=1 node tools/generators/drops.mjs "$check_binary" 1201 100
parallel_step 1 sugars env LKJSCRIPT_ENGINES="$check_engines_text" node tools/generators/sugar.mjs "$check_binary" 1301 216
parallel_step 1 closures env LKJSCRIPT_ENGINES="$check_engines_text" node tools/generators/closures.mjs "$check_binary" 1501 44
while ((check_waited < ${#check_pids[@]})); do wait_oldest; done
for log in "${check_logs[@]}"; do cat "$log"; done
parallel_elapsed_ns=$(($(date +%s%N)-parallel_started_ns))
awk -v elapsed_ns="$parallel_elapsed_ns" 'BEGIN { printf "Gate parallel suites: %.3f s\n", elapsed_ns / 1000000000 }'
if ((check_failed)); then exit 1; fi
check_elapsed_ns=$(($(date +%s%N)-check_started_ns))
awk -v elapsed_ns="$check_elapsed_ns" 'BEGIN { printf "Full check: %.3f s\n", elapsed_ns / 1000000000 }'
