#!/usr/bin/env bash
# Explicit-package scale workload, outside the gate. One compiler process at a time.
set -euo pipefail
export LC_ALL=C
script_root=$(cd "$(dirname "$0")" && pwd -P)
repository=$(cd "$script_root/.." && pwd -P)
compiler=
results=
rungs=10000,100000,1000000,10000000
memory_ceiling=8192
time_ceiling=3600
real=yes
while (($#)); do
    case "$1" in
        --repository) repository=$(realpath "$2"); shift 2 ;;
        --compiler) compiler=$(realpath "$2"); shift 2 ;;
        --results) results=$(realpath -m "$2"); shift 2 ;;
        --rungs) rungs=$2; shift 2 ;;
        --memory-ceiling-mib) memory_ceiling=$2; shift 2 ;;
        --time-ceiling) time_ceiling=$2; shift 2 ;;
        --skip-real) real=no; shift ;;
        *) printf 'unknown unit-scale option: %s\n' "$1" >&2; exit 2 ;;
    esac
done
[[ -n "$compiler" ]] || compiler="$repository/target/lkjscript"
[[ -n "$results" ]] || results="$repository/bench/unit-scale-results"
[[ -x "$compiler" && -d "$repository/library" && -f "$repository/src/main.lkj" ]] || exit 2
[[ "$memory_ceiling" =~ ^[1-9][0-9]*$ && "$time_ceiling" =~ ^[1-9][0-9]*$ ]] || exit 2
((memory_ceiling <= 8192)) || { echo 'shared-host memory ceiling must not exceed 8192 MiB' >&2; exit 2; }
IFS=, read -r -a sizes <<< "$rungs"
previous=0
for lines in "${sizes[@]}"; do
    [[ "$lines" =~ ^[1-9][0-9]*$ ]] && ((lines >= 100 && lines > previous && lines <= 10000000)) || exit 2
    previous=$lines
done
[[ ! -e "$results" ]] || { echo 'result directory already exists; a cohort is never overwritten' >&2; exit 2; }
mkdir -p "$repository/target"
mkdir -p "$(dirname "$results")"
mkdir "$results"
run=$(mktemp -d "$repository/target/unit-scale.XXXXXX")
key_root=$(mktemp -d /tmp/lkjscript-unit-scale-key.XXXXXX)
cleanup() {
    local status=$?
    trap - EXIT
    rm -rf -- "$run" "$key_root"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
umask 077
dd if=/dev/urandom of="$key_root/key" bs=32 count=1 status=none
cp "$compiler" "$run/producer"
chmod u-w "$run/producer"
producer="$run/producer"
unset LKJSCRIPT_STORE LKJSCRIPT_STORE_KEY
{
    printf 'started_utc=%s\nrepository=%s\ncompiler=%s\nrungs=%s\n' "$(date -u +%FT%TZ)" "$repository" "$compiler" "$rungs"
    printf 'compiler_sha256='; sha256sum "$producer" | cut -d' ' -f1
    printf 'source_commit='; git -C "$repository" rev-parse HEAD
    printf 'memory_ceiling_mib=%s\ntime_ceiling_s=%s\n' "$memory_ceiling" "$time_ceiling"
    printf 'workload=connected explicit-unit DAG; separate from the historical shape-sampled ladder\n'
    printf 'ten_million=one cold-build invocation only; no fresh check, warm build or retry at that rung\n'
    printf 'memory=per-process virtual-address envelope; GNU time reports largest process RSS\n'
} > "$results/cohort.txt"
(cd "$repository" && rg --files library -g '*.lkj' -0 | sort -z | xargs -0 sha256sum) > "$results/library.sha256"
sha256sum "$script_root/unit-scale.sh" "$script_root/unit-scale-generate.pl" "$script_root/unit-scale-record.pl" > "$results/harness.sha256"
git -C "$repository" status --short -- src library > "$results/source-status.txt"
current_memory=2048
((current_memory <= memory_ceiling)) || current_memory=$memory_ceiling
current_seconds=300
((current_seconds <= time_ceiling)) || current_seconds=$time_ceiling
rung=
rung_peak=0
rung_disk=0
cold_wall=0
store_size() {
    local total=0 directory bytes
    for directory in "$rung/store-build" "$rung/store-check"; do
        if [[ -d "$directory" ]]; then
            bytes=$(du -sb "$directory" | cut -f1)
            total=$((total + bytes))
        fi
    done
    printf '%s\n' "$total"
}
run_phase() {
    local label=$1 phase=$2 lines=$3 metadata=$4 artifact=$5
    shift 5
    local prefix="$results/$label-$lines-$phase" before after status bytes=0 stores=0 peak disk
    read -r before _ < /proc/loadavg
    printf '%q ' "$@" > "$prefix.command"
    printf '\n' >> "$prefix.command"
    status=0
    /usr/bin/time -o "$prefix.time" -f '%e\t%U\t%S\t%M\t%x' \
        timeout --signal=TERM --kill-after=10s "${current_seconds}s" \
        bash -c 'ulimit -v "$1" || exit 125; shift; exec "$@"' unit-scale "$((current_memory * 1024))" "$@" \
        > "$prefix.stdout" 2> "$prefix.stderr" || status=$?
    read -r after _ < /proc/loadavg
    [[ "$status" != 0 || "$artifact" == '-' || ! -f "$artifact" ]] || bytes=$(stat -c%s "$artifact")
    if [[ -n "$rung" && -d "$rung" ]]; then
        stores=$(store_size)
        disk=$(du -sb "$rung" | cut -f1)
        ((disk <= rung_disk)) || rung_disk=$disk
    fi
    perl "$script_root/unit-scale-record.pl" "$results" "$label" "$phase" "$lines" "$metadata" "$prefix" \
        "$status" "$before" "$after" "$current_memory" "$current_seconds" "$bytes" "$stores"
    peak=$(awk -F'\t' 'NF == 5 && $4 ~ /^[0-9]+$/ { value=$4 } END { print value+0 }' "$prefix.time")
    ((peak <= rung_peak)) || rung_peak=$peak
    if [[ "$phase" == cold-build ]]; then cold_wall=$(awk -F'\t' 'NF == 5 { value=$1 } END { print value+0 }' "$prefix.time"); fi
    return "$status"
}
native_control() {
    local phase=$1 expected=$2 artifact=$3
    local prefix="$results/generated-$lines-$phase-native"
    timeout --signal=TERM --kill-after=10s 120s "$artifact" > "$prefix.stdout" 2> "$prefix.stderr"
    printf '%s\n' "$expected" > "$prefix.expected"
    cmp "$prefix.expected" "$prefix.stdout"
    [[ ! -s "$prefix.stderr" ]]
}
cd "$repository"
if [[ "$real" == yes ]]; then
    current_memory=$memory_ceiling
    current_seconds=$time_ceiling
    for phase in check build; do
        command=("$producer" "$phase" src/main.lkj --source-dir "$repository" --library-dir "$repository/library" --no-store --stats)
        artifact=-
        if [[ "$phase" == build ]]; then
            artifact="$run/ordinary-compiler"
            command+=(--runtime-source "$repository/library/runtime.lkj" --inline-limit 64 -o "$artifact")
        fi
        if ! run_phase compiler "$phase" NA - "$artifact" "${command[@]}"; then
            printf 'compiler specimen refused; generated cohort continues\n' >> "$results/walls.txt"
        fi
    done
    real_source=examples/real_programs/levenshtein/main.lkj
    if [[ -f "$real_source" ]]; then
        cp examples/real_programs/levenshtein/upstream.json "$results/levenshtein-provenance.json"
        for phase in check build; do
            command=("$producer" "$phase" "$real_source" --source-dir "$repository" --library-dir "$repository/library" --no-store --stats)
            artifact=-
            if [[ "$phase" == build ]]; then
                artifact="$run/levenshtein"
                command+=(--runtime-source "$repository/library/runtime.lkj" --grant output -o "$artifact")
            fi
            if ! run_phase levenshtein-normalized "$phase" NA - "$artifact" "${command[@]}"; then
                printf 'levenshtein specimen refused; inspect its exact diagnostic\n' >> "$results/walls.txt"
            fi
        done
    else
        printf 'levenshtein specimen is not present in the selected repository revision\n' >> "$results/walls.txt"
    fi
    current_memory=2048
    ((current_memory <= memory_ceiling)) || current_memory=$memory_ceiling
    current_seconds=300
    ((current_seconds <= time_ceiling)) || current_seconds=$time_ceiling
fi
previous_lines=0
previous_disk=0
for lines in "${sizes[@]}"; do
    free_bytes=$(df -Pk "$repository/target" | awk 'NR == 2 {printf "%.0f", $4 * 1024}')
    required_bytes=$((256 * 1024 * 1024 + lines * 128))
    if ((previous_lines > 0)); then required_bytes=$((256 * 1024 * 1024 + previous_disk * lines / previous_lines * 2)); fi
    if ((free_bytes < required_bytes)); then
        printf '%s lines: pool-space wall, available=%s predicted_required=%s bytes\n' "$lines" "$free_bytes" "$required_bytes" >> "$results/walls.txt"
        break
    fi
    rung="$run/rung-$lines"
    mkdir "$rung"
    perl "$script_root/unit-scale-generate.pl" generate "$rung/source" "$lines" > "$results/workload-$lines.summary.json"
    cp "$rung/source/workload.json" "$results/workload-$lines.json"
    metadata="$results/workload-$lines.json"
    expected=$(perl -MJSON::PP -0777 -ne 'print decode_json($_)->{expected_baseline}' "$metadata")
    expected_edited=$(perl -MJSON::PP -0777 -ne 'print decode_json($_)->{expected_edited}' "$metadata")
    mkdir "$rung/store-build" "$rung/store-check"
    artifact="$rung/program"
    common=(units.lkj --source-dir "$rung/source" --library-dir "$repository/library" --store-key "$key_root/key" --store-bytes 8589934592 --stats)
    build=("$producer" build-units "${common[@]}" --store "$rung/store-build" --runtime-source "$repository/library/runtime.lkj" -o "$artifact")
    rung_peak=0
    rung_disk=0
    cold_wall=0
    if ((lines == 10000000)); then
        (set -o noclobber; printf '%s\n' "$(date -u +%FT%TZ)" > "$results/ten-million-attempt")
    fi
    if ! run_phase generated cold-build "$lines" "$metadata" "$artifact" "${build[@]}"; then
        printf '%s lines: cold build refused under %s MiB/%s s; see compiler diagnostic\n' "$lines" "$current_memory" "$current_seconds" >> "$results/walls.txt"
        break
    fi
    if ((lines == 10000000)); then
        printf '10000000 lines: the one cold build succeeded; other 10M phases were not attempted\n' >> "$results/walls.txt"
        break
    fi
    native_control baseline "$expected" "$artifact"
    cold_image=$(sha256sum "$artifact" | cut -d' ' -f1)
    if ! run_phase generated cold-check "$lines" "$metadata" - "$producer" check-units "${common[@]}" --store "$rung/store-check"; then
        printf '%s lines: fresh checking refused; see diagnostic\n' "$lines" >> "$results/walls.txt"
        break
    fi
    rm -rf -- "$rung/store-check"
    if ! run_phase generated unchanged-build "$lines" "$metadata" "$artifact" "${build[@]}"; then
        printf '%s lines: unchanged build refused; see diagnostic\n' "$lines" >> "$results/walls.txt"
        break
    fi
    [[ "$(sha256sum "$artifact" | cut -d' ' -f1)" == "$cold_image" ]] || {
        printf '%s lines: warm image differs from cold image\n' "$lines" >> "$results/walls.txt"; exit 1;
    }
    perl "$script_root/unit-scale-generate.pl" edit "$rung/source" > "$results/body-edit-$lines.json"
    if ! run_phase generated body-build "$lines" "$metadata" "$artifact" "${build[@]}"; then
        printf '%s lines: one-body build refused; see diagnostic\n' "$lines" >> "$results/walls.txt"
        break
    fi
    native_control edited "$expected_edited" "$artifact"
    previous_lines=$lines
    previous_disk=$rung_disk
    next_memory=$(((rung_peak + 1023) / 1024 * 3 + 512))
    ((next_memory >= 1536)) || next_memory=1536
    ((next_memory <= memory_ceiling)) || next_memory=$memory_ceiling
    current_memory=$next_memory
    current_seconds=$(awk -v wall="$cold_wall" 'BEGIN {printf "%.0f", int(wall * 30 + 61)}')
    ((current_seconds >= 300)) || current_seconds=300
    ((current_seconds <= time_ceiling)) || current_seconds=$time_ceiling
    rm -rf -- "$rung"
    rung=
done
printf 'finished_utc=%s\n' "$(date -u +%FT%TZ)" >> "$results/cohort.txt"
printf 'unit scale evidence: %s\n' "$results"
