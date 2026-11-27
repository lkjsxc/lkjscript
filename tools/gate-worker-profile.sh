#!/usr/bin/env bash
# Observe a timeout-wrapped lkjscript test --stats command without replacing
# its native batch workers. /proc samples are lower bounds, not exit accounting.
set -euo pipefail
mkdir -p target/gate
scratch=$(mktemp -d target/gate/worker-profile.XXXXXX)
cleanup() {
    if [[ -n ${test_pid:-} ]]; then kill "$test_pid" 2>/dev/null || :; wait "$test_pid" || :; fi
    if [[ -n ${observer_pid:-} ]]; then kill "$observer_pid" 2>/dev/null || :; wait "$observer_pid" || :; fi
    rm -rf "$scratch"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

sample_workers() {
    local root=$1 ticks parent pid name command request record i worker
    local -a parents children fields
    local -A users systems samples
    ticks=$(getconf CLK_TCK) || return 1
    while [[ ! -e $scratch/stop ]] && kill -0 "$root" 2>/dev/null; do
        parents=()
        read -r -a parents 2>/dev/null <"/proc/$root/task/$root/children" || :
        for parent in "${parents[@]}"; do
            children=()
            read -r -a children 2>/dev/null <"/proc/$parent/task/$parent/children" || :
            for pid in "${children[@]}"; do
                {
                    IFS= read -r -d '' name &&
                    IFS= read -r -d '' command &&
                    IFS= read -r -d '' request
                } 2>/dev/null <"/proc/$pid/cmdline" || continue
                [[ $command == batch && $request =~ ^\.lkjscript-test-[0-9]+/worker-([0-9]+)$ ]] || continue
                worker=${BASH_REMATCH[1]}
                read -r record 2>/dev/null <"/proc/$pid/stat" || continue
                # Fields after the last ") ": state is field 3. Include CPU
                # from already-reaped native children (cutime/cstime).
                read -r -a fields <<<"${record##*) }"
                [[ ${#fields[@]} -ge 15 ]] || continue
                users[$worker]=$((fields[11] + fields[13]))
                systems[$worker]=$((fields[12] + fields[14]))
                samples[$worker]=$(("${samples[$worker]:-0}" + 1))
            done
        done
        sleep 0.2
    done
    for i in "${!samples[@]}"; do
        printf 'CPU %s %s %s %s %s\n' "$i" "${users[$i]}" "${systems[$i]}" "${samples[$i]}" "$ticks"
    done
}

status=0
"$@" >"$scratch/details" 2>&1 &
test_pid=$!
sample_workers "$test_pid" >"$scratch/cpu" &
observer_pid=$!
wait "$test_pid" || status=$?
test_pid=
: >"$scratch/stop"
wait "$observer_pid" || printf 'LKJSCRIPT_GATE_WORKER observation=unavailable\n' >>"$scratch/details"
observer_pid=
cat "$scratch/details"
if ((status == 0)); then
    # test_runner prints worker stderr in ascending partition order, followed
    # by the parent compiler's own final stats record. Never count that record
    # as a worker or confuse wall-time phases with CPU time.
    LC_ALL=C awk '
        /^CPU / {
            cpu[$2] = sprintf("cpu_user_s_lower_bound=%.3f cpu_system_s_lower_bound=%.3f samples=%d", $3/$6, $4/$6, $5)
        }
        function emit(line, slot,   pair, result) {
            result = ""
            while (match(line, /"[[:alnum:]_]+_ms":[0-9]+(\.[0-9]+)?|"test_(reused|rerun)":[0-9]+/)) {
                pair = substr(line, RSTART, RLENGTH)
                line = substr(line, RSTART+RLENGTH)
                gsub(/"/, "", pair); sub(/:/, "=", pair)
                result = result " " pair
            }
            printf "LKJSCRIPT_GATE_WORKER worker=%d %s%s\n", slot, (slot in cpu ? cpu[slot] : "cpu=unavailable"), result
        }
        /^LKJSCRIPT_STATS / {
            if (seen++) emit(previous, seen-2)
            previous = $0
        }
    ' "$scratch/cpu" "$scratch/details"
fi
exit "$status"
