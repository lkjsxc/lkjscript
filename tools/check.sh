#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
check_started_ns=$(date +%s%N)
check_jobs="${LKJSCRIPT_GATE_JOBS:-8}"
if [[ ! "$check_jobs" =~ ^[1-8]$ ]]; then
    echo 'LKJSCRIPT_GATE_JOBS must be between one and eight' >&2
    exit 1
fi
check_suite_jobs=$((check_jobs < 4 ? check_jobs : 4))
mkdir -p target/gate
step() {
    local label=$1
    shift
    local started_ns elapsed_ns result=0
    started_ns=$(date +%s%N)
    local details="target/gate/details-$label.log"
    "$@" >"$details" 2>&1 || result=$?
    if ((result)); then
        cat "$details" >&2
        return "$result"
    fi
    elapsed_ns=$(($(date +%s%N)-started_ns))
    awk -v label="$label" -v elapsed_ns="$elapsed_ns" 'BEGIN { printf "ok %s (%.3f s)\n", label, elapsed_ns / 1000000000 }'
}
step bootstrap timeout --kill-after=10s 600s sh tools/bootstrap.sh
check_binary=target/lkjscript
step independence sh tools/check-independence.sh
step tool-build timeout --kill-after=10s 600s bash -c '
    set -e
    target/lkjscript build tools/toolchain-test.rs -o target/gate/toolchain-test
    target/lkjscript build tools/world-test.rs -o target/gate/world-test
    target/lkjscript build src/semantic_main.rs -o target/gate/semantic-test
'
toolchain() (
    toolchain_directory=$(mktemp -d target/gate/toolchain.XXXXXX)
    trap 'rm -rf "$toolchain_directory"' EXIT HUP INT TERM
    timeout --kill-after=10s 600s target/gate/toolchain-test "$check_binary" "$toolchain_directory"
)
world_cases() (
    world_directory=$(mktemp -d target/gate/world.XXXXXX)
    trap 'rm -rf "$world_directory"' EXIT HUP INT TERM
    timeout --kill-after=10s 600s target/gate/world-test "$check_binary" "$world_directory"
)
semantics() {
    local pids=() shard result=0
    for ((shard=0; shard<check_suite_jobs; shard++)); do
        timeout --kill-after=10s 600s target/gate/semantic-test "$shard" "$check_suite_jobs" >"target/gate/semantic-$shard.log" 2>&1 &
        pids+=("$!")
    done
    for shard in "${!pids[@]}"; do
        if ! wait "${pids[$shard]}"; then result=1; fi
        cat "target/gate/semantic-$shard.log"
    done
    return "$result"
}
generate() {
    rm -rf target/gate/generated
    mkdir -p target/gate/generated
    "$check_binary" generate --self-test || return 1
    "$check_binary" generate values 601 100 -o target/gate/generated/values || return 1
    "$check_binary" generate ownership 601 100 -o target/gate/generated/ownership || return 1
    "$check_binary" generate output 601 50 -o target/gate/generated/output || return 1
    "$check_binary" generate drops 1201 100 -o target/gate/generated/drops || return 1
    "$check_binary" generate sugar 1301 216 -o target/gate/generated/sugar || return 1
    "$check_binary" generate closures 1501 44 -o target/gate/generated/closures || return 1
    "$check_binary" generate integers 0 0 -o target/gate/generated/integers || return 1
}
step generate generate
# Each suite declares its worker count. The scheduler never exceeds the shared
# host allowance; a worker's native child runs while its compiler waits.
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
parallel_step 1 toolchain toolchain
parallel_step 1 world world_cases
parallel_step 1 profiler timeout --kill-after=10s 600s sh tools/profile-test.sh "$check_binary"
parallel_step 1 resources timeout --kill-after=10s 600s bash tools/resource-test.sh "$check_binary"
parallel_step 1 runner timeout --kill-after=10s 600s "$check_binary" test src/test_runner.rs
parallel_step "$check_suite_jobs" conformance timeout --kill-after=10s 600s "$check_binary" test tests/conformance --jobs "$check_suite_jobs"
parallel_step "$check_suite_jobs" generated timeout --kill-after=10s 600s "$check_binary" test target/gate/generated --jobs "$check_suite_jobs"
parallel_step "$check_suite_jobs" semantic-structure semantics
parallel_step "$check_suite_jobs" semantic-cases timeout --kill-after=10s 600s "$check_binary" test tests/semantics --jobs "$check_suite_jobs"
parallel_step 1 references timeout --kill-after=10s 600s "$check_binary" test tests/reference
while ((check_waited < ${#check_pids[@]})); do wait_oldest; done
for log in "${check_logs[@]}"; do cat "$log"; done
if ((check_failed)); then exit 1; fi
step ir-mutation timeout --kill-after=10s 600s "$check_binary" mutate 1701 128 --jobs "$check_jobs"
check_elapsed_ns=$(($(date +%s%N)-check_started_ns))
awk -v elapsed_ns="$check_elapsed_ns" 'BEGIN { printf "ok gate (%.3f s)\n", elapsed_ns / 1000000000 }'
