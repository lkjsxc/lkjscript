#!/usr/bin/env bash
# The same checked IR snapshot supplies each entry's authority report and image.
# This trusted caller supplies fresh matched pairs and exact entry names; the
# digest footer catches swaps. Compiler verdicts remain fresh. Script bytes are
# part of each closed execution identity.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
action=${2:-run}
closed_directory=${3:-target/gate/closed}
reuse=()
if [[ -n "${LKJSCRIPT_STORE_KEY:-}" ]]; then
    reuse=(--store "${LKJSCRIPT_STORE:-target/store}" --store-key "$LKJSCRIPT_STORE_KEY")
fi
run_closed() {
    local name=$1 count=$2 entry=$3
    "$closed_directory/runner" "$closed_directory/$name" "$closed_directory/$name.authority" \
        tools/closed-semantics.sh "$count" "${LKJSCRIPT_STORE:-target/store}" "${LKJSCRIPT_STORE_KEY:-}" "$entry"
}
if [[ "$action" == build ]]; then
    mkdir -p "$closed_directory"
    "$compiler" build src/gate_reuse_main.lkj -o "$closed_directory/runner" "${reuse[@]}"
    "$compiler" ir src/closed_semantics.lkj -o "$closed_directory/cases.lkjir" "${reuse[@]}"
    "$compiler" build-entries "$closed_directory/cases.lkjir" -o "$closed_directory" \
        --artifact cases=main "${reuse[@]}"
    "$compiler" ir src/semantic_main.lkj -o "$closed_directory/structure.lkjir" "${reuse[@]}"
    "$compiler" build-entries "$closed_directory/structure.lkjir" -o "$closed_directory" \
        --artifact live=main --artifact structure-main=closed_core \
        --artifact structure-source_0=closed_source_0 --artifact structure-source_1=closed_source_1 \
        --artifact structure-source_2=closed_source_2 --artifact structure-source_3=closed_source_3 "${reuse[@]}"
elif [[ "$action" == run ]]; then
    if ! run_closed cases 28 main; then
        "$compiler" run "$closed_directory/cases.lkjir" --engine native "${reuse[@]}" >&2 || true
        exit 1
    fi
elif [[ "$action" == structure ]]; then
    closed_jobs=${LKJSCRIPT_GATE_JOBS:-4}
    [[ "$closed_jobs" =~ ^[1-4]$ ]] || { echo 'closed semantics: expected one to four workers' >&2; exit 1; }
    declare -A closed_active=()
    closed_failed=0
    closed_logs=()
    cleanup() {
        local pid
        for pid in "${!closed_active[@]}"; do kill "$pid" 2>/dev/null || true; done
        for pid in "${!closed_active[@]}"; do wait "$pid" 2>/dev/null || true; done
    }
    trap cleanup EXIT HUP INT TERM
    wait_closed() {
        local finished= status=0
        wait -n -p finished "${!closed_active[@]}" || status=$?
        if [[ -z "${finished:-}" ]]; then
            for finished in "${!closed_active[@]}"; do break; done
            status=0
            wait "$finished" || status=$?
        fi
        if ((status)); then closed_failed=1; fi
        unset 'closed_active[$finished]'
    }
    for entry in main source_0 source_1 source_2 source_3; do
        while (( ${#closed_active[@]} >= closed_jobs )); do wait_closed; done
        count=134
        if [[ "$entry" == main ]]; then count=196; fi
        log="$closed_directory/structure-$entry.log"
        checked_entry="closed_$entry"
        if [[ "$entry" == main ]]; then checked_entry=closed_core; fi
        run_closed "structure-$entry" "$count" "$checked_entry" > "$log" 2>&1 &
        closed_active[$!]=1
        closed_logs+=("$log")
    done
    while (( ${#closed_active[@]} )); do wait_closed; done
    for log in "${closed_logs[@]}"; do cat "$log"; done
    if ((closed_failed)); then
        "$closed_directory/live" closed-report >&2 || true
        exit 1
    fi
else
    echo "closed-semantics: expected build, run or structure" >&2
    exit 1
fi
