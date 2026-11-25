#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
check_started_ns=$(date +%s%N)
check_jobs="${LKJSCRIPT_GATE_JOBS:-4}"
read -r check_load_start _ < /proc/loadavg
export LKJSCRIPT_STORE="${LKJSCRIPT_STORE:-target/store}"
export LKJSCRIPT_STORE_KEY="${LKJSCRIPT_STORE_KEY:-$(sh tools/store-key.sh || true)}"
mkdir -p "$LKJSCRIPT_STORE"
check_reuse=()
if [[ -n "$LKJSCRIPT_STORE_KEY" ]]; then check_reuse=(--store "$LKJSCRIPT_STORE" --store-key "$LKJSCRIPT_STORE_KEY"); fi
if [[ "$check_jobs" != 4 ]]; then
    echo 'LKJSCRIPT_GATE_JOBS must be four (the process pool specimens exercise four workers)' >&2
    exit 1
fi
check_suite_jobs=$((check_jobs < 4 ? check_jobs : 4))
mkdir -p target/gate
step() {
    local label=$1
    shift
    local started_ns elapsed_ns offset_ns load_start load_end result=0
    read -r load_start _ < /proc/loadavg
    started_ns=$(date +%s%N)
    offset_ns=$((started_ns - check_started_ns))
    local details="target/gate/details-$label.log"
    "$@" >"$details" 2>&1 || result=$?
    if ((result)); then
        local ended_ns failure_timing=
        if ended_ns=$(date +%s%N) && ((ended_ns >= started_ns)) && read -r load_end _ < /proc/loadavg; then
            printf -v failure_timing '; elapsed_ns=%s offset_ns=%s load=%s -> %s' \
                "$((ended_ns - started_ns))" "$offset_ns" "$load_start" "$load_end"
        fi
        printf 'FAIL %s (status %d, details %s)%s\n' "$label" "$result" "$details" "$failure_timing" >&2 || :
        cat "$details" >&2
        return "$result"
    fi
    elapsed_ns=$(($(date +%s%N)-started_ns))
    read -r load_end _ < /proc/loadavg
    awk -v label="$label" -v elapsed_ns="$elapsed_ns" -v before="$load_start" -v after="$load_end" -v offset_ns="$offset_ns" '
        label ~ /^macro-origins-/ && /^macro-origins (shard |scope=|entry=)/ { print }
        /^LKJSCRIPT_STATS / {
            if (match($0, /"test_reused":[0-9]+/)) {
                value=substr($0, RSTART, RLENGTH); sub(/^[^:]*:/, "", value)
                reused += value; counted=1
            }
            if (match($0, /"test_rerun":[0-9]+/)) {
                value=substr($0, RSTART, RLENGTH); sub(/^[^:]*:/, "", value)
                rerun += value
            }
        }
        /^LKJSCRIPT_CLOSED_GATE / {
            for (i=1; i<=NF; i++) {
                if ($i ~ /^reused=/) { value=$i; sub(/^reused=/, "", value); reused += value }
                if ($i ~ /^rerun=/) { value=$i; sub(/^rerun=/, "", value); rerun += value }
            }
            counted=1; closed=1
        }
        END {
            printf "ok %s (%.3f s, load %s -> %s; gate+%.3f..%.3f s)", label, elapsed_ns / 1000000000, before, after, offset_ns / 1000000000, (offset_ns + elapsed_ns) / 1000000000
            if (closed) printf "; cases reused=%d rerun=%d (authenticated exact stable runner, sealed ELF, checked closure and closed input contract)", reused, rerun
            else if (counted && reused + rerun > 0) printf "; cases reused=%d rerun=%d (authenticated producer and input closure; misses/open effects rerun)", reused, rerun
            else printf "; rerun suite (live host observations or fresh admission)"
            printf "\n"
        }
    ' "$details"
}
step bootstrap timeout --kill-after=10s 600s sh tools/bootstrap.sh
check_binary=target/lkjscript
step independence sh tools/check-independence.sh
step tool-build timeout --kill-after=10s 600s bash -c '
    set -e
    reuse=()
    if [[ -n "$LKJSCRIPT_STORE_KEY" ]]; then reuse=(--store "$LKJSCRIPT_STORE" --store-key "$LKJSCRIPT_STORE_KEY"); fi
    target/lkjscript build tools/toolchain-test.lkj -o target/gate/toolchain-test "${reuse[@]}"
    target/lkjscript build tools/world-test.lkj -o target/gate/world-test "${reuse[@]}"
'
step closed-build timeout --kill-after=10s 600s bash tools/closed-semantics.sh "$check_binary" build
toolchain() (
    toolchain_directory=target/gate/toolchain-work
    mkdir "$toolchain_directory" || { echo "toolchain fixture directory is already claimed" >&2; exit 1; }
    trap 'rm -rf "$toolchain_directory"' EXIT HUP INT TERM
    timeout --kill-after=10s 600s target/gate/toolchain-test "$check_binary" "$toolchain_directory" "${check_reuse[@]}"
)
world_cases() (
    world_directory=$(mktemp -d target/gate/world.XXXXXX)
    trap 'rm -rf "$world_directory"' EXIT HUP INT TERM
    timeout --kill-after=10s 600s target/gate/world-test "$check_binary" "$world_directory"
)
diagnostic_cases() {
    timeout --kill-after=10s 600s "$check_binary" test tests/diagnostics --stats "${check_reuse[@]}" || return 1
    timeout --kill-after=10s 600s sh tests/diagnostics/test_library_directory.sh "$check_binary" || return 1
    sh tests/diagnostics/mutation_no_store.sh "$check_binary"
}
semantics() {
    timeout --kill-after=10s 600s target/gate/closed/live
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
check_logs=()
declare -A check_weights=()
check_active=0
check_failed=0
wait_finished() {
    local finished= status=0
    # Do not leave idle workers behind a long-running earlier suite.
    wait -n -p finished "${!check_weights[@]}" || status=$?
    if [[ -z "${finished:-}" ]]; then
        # Bash may already have reaped every completed child before wait -n.
        # Explicit wait still retrieves its status; all registered PIDs are ours.
        for finished in "${!check_weights[@]}"; do break; done
        status=0
        wait "$finished" || status=$?
    fi
    if ((status)); then check_failed=1; fi
    check_active=$((check_active - check_weights[$finished]))
    unset 'check_weights[$finished]'
}
parallel_step() {
    local weight=$1
    shift
    while ((check_active + weight > check_jobs)); do wait_finished; done
    local log="target/gate/$1.log"
    (step "$@") >"$log" 2>&1 &
    check_weights[$!]=$weight
    check_logs+=("$log")
    check_active=$((check_active + weight))
}
macro_origin_step() {
    local engine=${1#--} entry=${2:-main} started_ns status=0
    started_ns=$(date +%s%N)
    timeout --kill-after=10s 1800s bash tools/macro-origin-test.sh "$check_binary" "$@" || status=$?
    printf 'macro-origins shard path=tests/fixtures/macro_origins.lkj#%s engine=%s status=%d elapsed_ns=%d\n' \
        "$entry" "$engine" "$status" "$(($(date +%s%N) - started_ns))"
    return "$status"
}
macro_origin_type_fragment_lexical() {
    local scope status=0
    for scope in type_origins fragment_token_origins lexical_origins; do
        macro_origin_step --interpreter "$scope" || status=$?
    done
    return "$status"
}
# Start the macro suites before other workers queue. Each uses one worker and
# has the existing watchdog. The native suite still runs the full aggregate.
parallel_step 1 macro-origins-native macro_origin_step --native
parallel_step 1 macro-origins-expression macro_origin_step --interpreter expression_origins
parallel_step 1 macro-origins-statement macro_origin_step --interpreter statement_origins
parallel_step 1 macro-origins-type-fragment-lexical macro_origin_type_fragment_lexical
parallel_step 1 closed-semantics timeout --kill-after=10s 600s bash tools/closed-semantics.sh "$check_binary" run
parallel_step 1 closed-proof-attacks timeout --kill-after=10s 600s bash tools/gate-reuse-test.sh "$check_binary"
parallel_step 1 toolchain toolchain
parallel_step 1 world world_cases
parallel_step 1 semantic-query timeout --kill-after=10s 600s bash tools/semantic-query-test.sh "$check_binary"
parallel_step 1 associated-query timeout --kill-after=10s 600s bash tools/associated-query-test.sh "$check_binary"
parallel_step 1 qualified-paths timeout --kill-after=10s 600s bash tools/qualified-paths-test.sh "$check_binary"
parallel_step 1 argument-impl-trait timeout --kill-after=10s 600s bash tools/argument-impl-trait-test.sh "$check_binary"
parallel_step 1 return-impl-trait timeout --kill-after=10s 600s bash tools/return-impl-trait-test.sh "$check_binary"
parallel_step 1 opaque-callable-return timeout --kill-after=10s 600s bash tools/opaque-callable-return-test.sh "$check_binary"
parallel_step 1 const-functions timeout --kill-after=10s 600s bash tools/const-functions-test.sh "$check_binary"
parallel_step 1 trait-constants timeout --kill-after=10s 600s bash tools/trait-constants-test.sh "$check_binary"
parallel_step 1 const-generics timeout --kill-after=10s 600s bash tools/const-generics-test.sh "$check_binary"
parallel_step 1 maybe-sized timeout --kill-after=10s 600s bash tools/maybe-sized-test.sh "$check_binary"
parallel_step 1 outlives-where timeout --kill-after=10s 600s bash tools/outlives-where-test.sh "$check_binary"
parallel_step 1 debug-format timeout --kill-after=10s 600s bash tools/debug-format-test.sh "$check_binary"
parallel_step 1 declarative-macros timeout --kill-after=10s 600s bash tools/declarative-macro-test.sh "$check_binary"
parallel_step 1 macro-providers timeout --kill-after=10s 600s bash tools/macro-provider-test.sh "$check_binary"
parallel_step 1 module-path timeout --kill-after=10s 600s bash tools/module-path-test.sh "$check_binary"
parallel_step 1 macro-statements timeout --kill-after=10s 600s bash tools/macro-statement-test.sh "$check_binary"
parallel_step 1 type-outlives timeout --kill-after=10s 600s bash tools/type-outlives-test.sh "$check_binary"
parallel_step 1 core-facade timeout --kill-after=10s 600s bash tools/core-facade-test.sh "$check_binary"
parallel_step 1 query-signature timeout --kill-after=10s 600s bash tools/query-signature-test.sh "$check_binary"
parallel_step 1 generated-source timeout --kill-after=10s 600s bash tools/generated-source-test.sh "$check_binary"
parallel_step 1 query-store timeout --kill-after=10s 600s bash tools/query-store-test.sh "$check_binary"
parallel_step 1 semantic-index-page timeout --kill-after=10s 600s bash tools/semantic-index-page-test.sh "$check_binary"
parallel_step 1 query-publication timeout --kill-after=10s 600s bash tools/query-publication-test.sh "$check_binary"
parallel_step 1 directory-publication timeout --kill-after=10s 600s bash tools/directory-rename-new-test.sh "$check_binary"
parallel_step 1 directory-identity timeout --kill-after=10s 600s bash tools/directory-identity-test.sh "$check_binary"
parallel_step 1 structure-projection timeout --kill-after=10s 600s bash tools/structure-projection-test.sh "$check_binary"
parallel_step 1 structure-module timeout --kill-after=10s 600s bash tools/structure-module-test.sh "$check_binary"
parallel_step 1 structure-alternate-race timeout --kill-after=10s 600s bash tools/structure-alternate-race-test.sh "$check_binary"
parallel_step 1 structure-create timeout --kill-after=10s 600s bash tools/structure-create-test.sh "$check_binary"
parallel_step 1 structure-create-kills timeout --kill-after=10s 600s bash tools/structure-create-kill-test.sh "$check_binary"
parallel_step 1 structure-edit timeout --kill-after=10s 600s bash tools/structure-test.sh "$check_binary"
parallel_step 1 structure-require timeout --kill-after=10s 600s bash tools/structure-require-test.sh "$check_binary"
parallel_step 1 structure-help timeout --kill-after=10s 600s bash tools/structure-help-test.sh "$check_binary"
parallel_step 1 structure-call timeout --kill-after=10s 600s bash tools/structure-call-test.sh "$check_binary"
parallel_step 1 catalog-namespace timeout --kill-after=10s 600s bash tools/catalog-namespace-test.sh "$check_binary"
parallel_step 1 catalog-roots timeout --kill-after=10s 600s bash tools/catalog-roots-test.sh "$check_binary"
parallel_step 1 structure-embedded timeout --kill-after=10s 600s bash tools/structure-embedded-test.sh "$check_binary"
parallel_step 1 structure-refactor timeout --kill-after=10s 600s bash tools/structure-refactor-test.sh "$check_binary"
parallel_step 1 checked-meaning timeout --kill-after=10s 600s bash tools/meaning-test.sh "$check_binary"
parallel_step 1 associated-bindings timeout --kill-after=10s 600s bash tools/associated-bindings-test.sh "$check_binary"
parallel_step 1 positional-traits timeout --kill-after=10s 600s bash tools/positional-traits-test.sh "$check_binary"
parallel_step 1 type-families timeout --kill-after=10s 600s bash tools/type-families-test.sh "$check_binary"
parallel_step 1 trait-method-arguments timeout --kill-after=10s 600s bash tools/trait-method-arguments-test.sh "$check_binary"
parallel_step 1 trait-equality-inference timeout --kill-after=10s 600s bash tools/trait-equality-inference-test.sh "$check_binary"
parallel_step 1 type-defaults timeout --kill-after=10s 600s bash tools/type-defaults-test.sh "$check_binary"
parallel_step 1 type-defaults-origins timeout --kill-after=10s 600s bash tools/type-defaults-origins-test.sh "$check_binary"
parallel_step 1 numeric-traits timeout --kill-after=10s 600s bash tools/numeric-traits-test.sh "$check_binary"
parallel_step 1 type-identity timeout --kill-after=10s 600s bash tools/type-identity-test.sh "$check_binary"
parallel_step 1 history-replay timeout --kill-after=10s 600s bash tools/replay-test.sh "$check_binary"
parallel_step 1 profiler timeout --kill-after=10s 600s sh tools/profile-test.sh "$check_binary"
parallel_step 1 optimizer timeout --kill-after=10s 600s bash tools/optimizer-test.sh "$check_binary"
parallel_step 1 checked-units timeout --kill-after=10s 600s env LKJSCRIPT_BINARY="$check_binary" sh tools/check-units.sh
parallel_step 1 unit-native timeout --kill-after=10s 600s env LKJSCRIPT_COMPILER="$check_binary" bash tools/unit-native.sh
parallel_step 1 source-units timeout --kill-after=10s 600s env LKJSCRIPT_BINARY="$check_binary" bash tools/unit-source-test.sh
# Hosted runners need more time for the authenticated scratch controls in the interpreter smoke.
parallel_step 1 unit-manifest timeout --kill-after=10s 1800s env LKJSCRIPT_BINARY="$check_binary" bash tools/unit-plan-test.sh
parallel_step 1 cache-budget timeout --kill-after=10s 600s bash tools/budget-test.sh "$check_binary"
parallel_step 1 unit-boundaries timeout --kill-after=10s 600s env LKJSCRIPT_BINARY="$check_binary" bash tools/unit-boundaries-test.sh
parallel_step 1 resources timeout --kill-after=10s 600s bash tools/resource-test.sh "$check_binary"
parallel_step 1 diagnostic-cases diagnostic_cases
parallel_step 1 cache-io timeout --kill-after=10s 600s bash tools/cache-io-test.sh "$check_binary"
parallel_step 1 cache-read timeout --kill-after=10s 600s bash tools/cache-read-test.sh "$check_binary"
parallel_step 2 store timeout --kill-after=10s 600s bash tools/store-test.sh "$check_binary"
parallel_step 2 program-directory timeout --kill-after=10s 600s bash tools/program-directory-test.sh "$check_binary"
parallel_step 2 http timeout --kill-after=10s 600s sh tools/http-test.sh --compiler "$check_binary" --seconds 0 --out target/gate/http-profiles
parallel_step 2 journal timeout --kill-after=10s 600s sh tools/http-journal-cases.sh --compiler "$check_binary"
parallel_step 2 stop timeout --kill-after=10s 600s sh tools/http-stop.sh --compiler "$check_binary"
parallel_step 2 stop-interpreter timeout --kill-after=10s 600s sh tools/http-stop.sh --compiler "$check_binary" --engine interpreter
parallel_step 2 site timeout --kill-after=10s 600s sh tools/http-site-test.sh --compiler "$check_binary" --offline
parallel_step 2 desk timeout --kill-after=10s 600s bash tools/http-desk-test.sh --compiler "$check_binary"
parallel_step 2 desk-events timeout --kill-after=10s 600s bash tools/http-desk-events.sh --compiler "$check_binary"
parallel_step 1 temporary-files timeout --kill-after=10s 600s bash tools/temporary-files-test.sh "$check_binary"
parallel_step 1 tcp-port timeout --kill-after=10s 600s bash tools/tcp-port-test.sh "$check_binary"
parallel_step 1 connect-deadline timeout --kill-after=10s 600s bash tools/connect-deadline-test.sh "$check_binary"
parallel_step 1 child-image timeout --kill-after=10s 600s bash tools/child-image-test.sh "$check_binary"
parallel_step 1 foreign-child timeout --kill-after=10s 600s bash tools/foreign-child-test.sh --compiler "$check_binary"
parallel_step 2 foreign-session timeout --kill-after=10s 600s bash tools/programs-foreign-session-test.sh --compiler "$check_binary"
parallel_step 4 process-workers timeout --kill-after=10s 600s bash tools/programs-process-test.sh --compiler "$check_binary"
parallel_step 1 unix-stream timeout --kill-after=10s 600s bash tools/unix-stream-test.sh --compiler "$check_binary"
parallel_step 1 object-graphs timeout --kill-after=10s 600s bash tools/programs-graph-test.sh --compiler "$check_binary"
parallel_step 1 real-programs timeout --kill-after=10s 600s bash tools/real-programs-test.sh "$check_binary"
parallel_step 1 package-source timeout --kill-after=10s 600s bash tools/programs-package-test.sh "$check_binary"
parallel_step 1 concrete-unit-policy timeout --kill-after=10s 600s bash tools/programs-unit-policy-test.sh "$check_binary"
parallel_step 2 runtime-admission timeout --kill-after=10s 600s bash tools/programs-admission-test.sh "$check_binary"
parallel_step 4 pool-publication timeout --kill-after=10s 600s bash tools/programs-publication-test.sh --compiler "$check_binary"
parallel_step "$check_suite_jobs" conformance timeout --kill-after=10s 1800s "$check_binary" test tests/conformance --stats --jobs "$check_suite_jobs" "${check_reuse[@]}"
parallel_step "$check_suite_jobs" generated timeout --kill-after=10s 600s "$check_binary" test target/gate/generated --stats --jobs "$check_suite_jobs" "${check_reuse[@]}"
parallel_step "$check_suite_jobs" optimized-conformance timeout --kill-after=10s 1800s "$check_binary" test tests/conformance --stats --jobs "$check_suite_jobs" --inline-limit 64 "${check_reuse[@]}"
parallel_step "$check_suite_jobs" optimized-generated timeout --kill-after=10s 600s "$check_binary" test target/gate/generated --stats --jobs "$check_suite_jobs" --inline-limit 64 "${check_reuse[@]}"
parallel_step "$check_suite_jobs" semantic-structure timeout --kill-after=10s 600s bash tools/closed-semantics.sh "$check_binary" structure
parallel_step 1 semantic-host semantics
parallel_step "$check_suite_jobs" semantic-cases timeout --kill-after=10s 600s "$check_binary" test tests/semantics --stats --jobs "$check_suite_jobs" "${check_reuse[@]}"
parallel_step 1 references timeout --kill-after=10s 600s "$check_binary" test tests/reference --stats "${check_reuse[@]}"
while ((check_active)); do wait_finished; done
for log in "${check_logs[@]}"; do cat "$log"; done
awk '
    /; cases reused=/ {
        for (i=1; i<=NF; i++) {
            if ($i ~ /^reused=/) { value=$i; sub(/^reused=/, "", value); reused += value }
            if ($i ~ /^rerun=/) { value=$i; sub(/^rerun=/, "", value); rerun += value }
        }
    }
    END { printf "ok gate selection: skipped %d case requests on authenticated exact compiler/input closure or stable runner/sealed ELF/checked closure/closed contract equality; reran %d misses/open effects; host suites rerun\n", reused, rerun }
' "${check_logs[@]}"
if ((check_failed)); then exit 1; fi
if ! awk '
    /^macro-origins scope=/ && $3 ~ /^cases=[0-9]+$/ {
        scopes[$2]++
        records++
        total += substr($3, length("cases=") + 1)
    }
    END {
        if (records != 5 || total != 63) exit 1
        if (scopes["scope=expression_origins"] != 1 || scopes["scope=statement_origins"] != 1 ||
            scopes["scope=type_origins"] != 1 || scopes["scope=fragment_token_origins"] != 1 ||
            scopes["scope=lexical_origins"] != 1) exit 1
    }
' "${check_logs[@]}"; then
    echo 'FAIL macro-origins (expected five interpreter scopes and 63 controls)' >&2
    exit 1
fi
printf 'ok macro-origins (63 malformed origins; statement positions/splices/roles/rules; both engines)\n'
step ir-mutation timeout --kill-after=10s 600s "$check_binary" mutate 1701 128 --jobs "$check_jobs" "${check_reuse[@]}"
check_elapsed_ns=$(($(date +%s%N)-check_started_ns))
read -r check_load_end _ < /proc/loadavg
awk -v elapsed_ns="$check_elapsed_ns" -v before="$check_load_start" -v after="$check_load_end" 'BEGIN { printf "ok gate (%.3f s, load %s -> %s)\n", elapsed_ns / 1000000000, before, after }'
