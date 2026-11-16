#!/usr/bin/env bash
# Fixed six-case strict replay. Preparation is static; launch needs a released worker.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ ${1:-} == --cleanup && $# == 2 ]]; then
    fixture=$(realpath -e "$2")
    [[ $fixture == "$PWD"/target/compiler-strict-recent.* && -d $fixture && ! -L $fixture ]] || exit 2
    [[ $(cat "$fixture/owner") == compiler-strict-recent-v1 ]] || exit 2
    pid=$(cat "$fixture/pid")
    if kill -0 "$pid" 2>/dev/null; then
        printf 'E_REPLAY_ACTIVE owner pid still exists: %s\n' "$pid" >&2
        exit 2
    fi
    # Snapshot first: an awk command containing the fixture path must not match itself.
    processes=$(ps -eo pid=,ppid=,args=)
    ancestors=" $$ "
    ancestor=$PPID
    while [[ -n $ancestor && $ancestor != 0 ]]; do
        ancestors+="$ancestor "
        ancestor=$(printf '%s\n' "$processes" | awk -v pid="$ancestor" '$1==pid {print $2; exit}')
    done
    if printf '%s\n' "$processes" | awk -v path="$fixture" -v ancestors="$ancestors" \
        '!index(ancestors," "$1" ") && index($0,path) { found=1; print > "/dev/stderr" } END { exit !found }'; then
        printf 'E_REPLAY_ACTIVE fixture has a live user\n' >&2
        exit 2
    fi
    rm -rf -- "$fixture"
    [[ ! -e $fixture ]]
    printf 'removed_fixture=%s\n' "$fixture"
    exit
fi
if [[ $# != 5 ]]; then
    printf 'usage: %s REPOSITORY GATED_BINARY EXPECTED_SHA256 PRODUCER_SOURCE_COMMIT OUTPUT_TSV\n' "$0" >&2
    printf '       %s --cleanup OWNED_FIXTURE\n' "$0" >&2
    exit 2
fi
repository=$(realpath -e "$1")
compiler_origin=$(realpath -e "$2")
expected_sha=$3
producer_source=$(git -C "$repository" rev-parse --verify "$4^{commit}")
output=$(realpath -m "$5")
[[ $expected_sha == adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7 && -x $compiler_origin ]] || exit 2
[[ $producer_source == 4f8c213559b7de96b3b6bd6199da661dfc95f778 ]] || exit 2
[[ $output == "$PWD"/bench/* || $output == "$PWD"/target/* ]] || exit 2
[[ -d $(dirname "$output") ]] || exit 2
for path in "$output" "$output.meta" "$output.logs" "$output.space"; do
    [[ ! -e $path ]] || { printf 'E_REPLAY_EXISTS %s\n' "$path" >&2; exit 2; }
done
for tool in git perl sha256sum prlimit timeout setsid df stat tar; do command -v "$tool" >/dev/null; done
[[ -x /usr/bin/time ]] || exit 2
memory_bytes=4294967296
timeout_seconds=900
store_bytes=2147483648
history_head=6a187b94fb763f7b0565436a98ab5feda3cd642f
perl bench/compiler-strict-recent-inventory.pl "$repository" bench/compiler-strict-recent --check
sha256sum --status -c bench/compiler-strict-recent.prepared.sha256
[[ $(sha256sum "$compiler_origin" | cut -d ' ' -f 1) == "$expected_sha" ]] || {
    printf 'E_REPLAY_PRODUCER origin hash differs from released pin\n' >&2; exit 2;
}
compiler_bytes=$(stat -c %s "$compiler_origin")
mkdir "$output.logs"
: > "$output.space"
printf 'ordinal\trevision\tparent\tstatus\texit_code\toperations\tstdout_lines\twall_s\tuser_s\tsys_s\tpeak_kib\tload_before\tload_after\treason\tlibrary_environment\tsource_root\n' > "$output"
printf 'segment=separate-six-recent-strict-cases\nhistory_head=%s\nmode=replay\nroot_policy=explicit-src-main\nacceptance=canonical-checked-meaning-and-every-changed-language-file\nidentity_domain=lkjscript-checked-meaning-1\nmemory_bytes=%s\ntimeout_seconds=%s\nstore_bytes=%s\nworkers=1\nresume=false\nretries=false\nstore_state=cold-shared-across-six\nfile_cache_reset=false\n' \
    "$history_head" "$memory_bytes" "$timeout_seconds" "$store_bytes" > "$output.meta"
printf 'compiler_origin=%s\ncompiler_sha256=%s\ncompiler_bytes=%s\nproducer_source_commit=%s\nproducer_src_tree=%s\nproducer_library_tree=%s\nsource_binding=explicit-owner-provided-gated-build-provenance\n' \
    "$compiler_origin" "$expected_sha" "$compiler_bytes" "$producer_source" \
    "$(git -C "$repository" rev-parse "$producer_source:src")" \
    "$(git -C "$repository" rev-parse "$producer_source:library")" >> "$output.meta"
printf 'harness_sha256=%s\nprepared_manifest_sha256=%s\nprior_proofs=24\nprior_producer_sha256=4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09\nlibrary_policy=physical-exact-historical-library-on-both-sides\nruntime_policy=no-runtime-selector-no-native-historical-image-execution\ninline_limit=0\n' \
    "$(sha256sum "$0" | cut -d ' ' -f 1)" \
    "$(sha256sum bench/compiler-strict-recent.prepared.sha256 | cut -d ' ' -f 1)" >> "$output.meta"
fixture=
active_pid=
completed=0
launched=0
passed=0
stop_reason=none
group_live() {
    local snapshot
    snapshot=$(ps -eo pgid=,stat=) || return 2
    printf '%s\n' "$snapshot" |
        awk -v group="$1" '$1==group && $2!~/^[ZX]/ {live=1} END {exit !live}'
}
stop_group() {
    local group=$1 attempt state=0
    group_live "$group" || state=$?
    [[ $state != 1 ]] || return 0
    [[ $state == 0 ]] || return 1
    kill -TERM -- "-$group" 2>/dev/null || true
    for attempt in {1..10}; do
        sleep 0.1
        state=0
        group_live "$group" || state=$?
        [[ $state != 1 ]] || return 0
        [[ $state == 0 ]] || return 1
    done
    kill -KILL -- "-$group" 2>/dev/null || true
    for attempt in {1..20}; do
        sleep 0.1
        state=0
        group_live "$group" || state=$?
        [[ $state != 1 ]] || return 0
        [[ $state == 0 ]] || return 1
    done
    return 1
}
finish() {
    local result=$?
    trap - EXIT HUP INT TERM
    if [[ $result != 0 && $stop_reason == none ]]; then stop_reason="harness-exit-$result"; fi
    if [[ -n $active_pid ]]; then
        if stop_group "$active_pid"; then
            wait "$active_pid" 2>/dev/null || true
        else
            stop_reason=undrained-owned-process-group
            result=1
            printf 'undrained_group=%s\n' "$active_pid" >> "$output.meta"
        fi
    fi
    printf 'completed_rows=%s\nlaunched=%s\npassed=%s\nstop_reason=%s\nexit_status=%s\nretained_fixture=%s\n' \
        "$completed" "$launched" "$passed" "$stop_reason" "$result" "$fixture" >> "$output.meta"
    exit "$result"
}
trap finish EXIT
trap 'stop_reason=signal-HUP; exit 129' HUP
trap 'stop_reason=signal-INT; exit 130' INT
trap 'stop_reason=signal-TERM; exit 143' TERM
archive_blocks() {
    git -C "$repository" ls-tree -rl "$1" -- src library tools tests |
        awk '$4 ~ /^[0-9]+$/ { bytes += int(($4 + 4095) / 4096) * 4096 } END { printf "%.0f\n", bytes }'
}
free_space() {
    local before_bytes after_bytes allowance required available
    before_bytes=$(archive_blocks "$1")
    after_bytes=$(archive_blocks "$2")
    allowance=$((2 * (before_bytes + after_bytes)))
    (( allowance >= 134217728 )) || allowance=134217728
    required=$((2 * store_bytes + allowance + compiler_bytes + 134217728))
    available=$(df -B1 --output=avail "$PWD" | tail -n 1)
    printf '%s\t%s\t%s\t%s\t%s\n' "$available" "$required" "$allowance" "$1" "$2" >> "$output.space"
    (( available >= required ))
}
if ! free_space "$history_head" "$history_head"; then
    stop_reason=E_REPLAY_SPACE
    printf 'E_REPLAY_SPACE initial reserve refused\n' >&2
    exit 2
fi
mkdir -p target
fixture=$(mktemp -d "$PWD/target/compiler-strict-recent.XXXXXX")
printf 'compiler-strict-recent-v1\n' > "$fixture/owner"
printf '%s\n' "$$" > "$fixture/pid"
cp -- "$compiler_origin" "$fixture/compiler"
compiler="$fixture/compiler"
[[ $(sha256sum "$compiler" | cut -d ' ' -f 1) == "$expected_sha" ]] || {
    stop_reason=E_REPLAY_COPY; exit 2;
}
chmod 500 "$compiler"
mkdir "$fixture/store"
umask 077
head -c 32 /dev/urandom > "$fixture/key"
[[ $(stat -c %s "$fixture/key") == 32 ]]
umask 022
printf 'fixture=%s\n' "$fixture" >> "$output.meta"
materialize() {
    local revision=$1 destination=$2 paths=$3 area
    local areas=()
    mkdir "$destination" || return 16
    for area in src library tools tests; do
        if git -C "$repository" cat-file -e "$revision:$area" 2>/dev/null; then areas+=("$area"); fi
    done
    git -C "$repository" ls-tree -r --name-only "$revision" -- "${areas[@]}" > "$paths" || return 15
    if rg -q '(^|/)\.generation-one(/|$)' "$paths"; then return 11; fi
    git -C "$repository" archive --format=tar "$revision" -- "${areas[@]}" |
        tar -xf - -C "$destination" || return 15
    [[ -z $(find "$destination" -type l -print -quit) ]] || return 12
    [[ -f "$destination/src/main.lkj" && -f "$destination/library/lib.lkj" ]] || return 14
    # No filename projection, source rewrite, synthetic root, cfg switch or library fallback.
}
load() { awk '{print $1 "," $2 "," $3}' /proc/loadavg; }
measure() {
    local stem=$1 result=0 start end before after group_cleanup
    shift
    printf '%s\0' "$@" > "$stem.argv0"
    before=$(load)
    start=$(date +%s%N)
    printf '%s\t%s\t%s\n' "$start" "$before" started > "$stem.started"
    (
        cd "$fixture"
        exec setsid /usr/bin/time -q -f '%e\t%U\t%S\t%M' -o "$stem.time" \
            prlimit --as="$memory_bytes" -- timeout --foreground --kill-after=5s "$timeout_seconds" \
            "$@"
    ) > "$stem.out" 2> "$stem.err" &
    active_pid=$!
    wait "$active_pid" || result=$?
    # Bound cleanup of the private group, including ignored TERM. Zombies have
    # no running work or open descriptors; wait below already reaped our child.
    group_cleanup=0
    stop_group "$active_pid" || group_cleanup=$?
    printf '%s\n' "$group_cleanup" > "$stem.group-cleanup"
    if [[ $group_cleanup == 0 ]]; then
        active_pid=
    else
        stop_reason=undrained-owned-process-group
    fi
    end=$(date +%s%N)
    after=$(load)
    printf '%s\t%s\t%s\t%s\n' "$start" "$end" "$before" "$after" > "$stem.capture"
    printf '%s\n' "$result" > "$stem.exit"
    return "$result"
}
while IFS=$'\t' read -r ordinal revision parent root weight file_count rest; do
    [[ $ordinal == ordinal ]] && continue
    stem="$output.logs/$(printf '%03d' "$ordinal")-$revision"
    printf '%s\n' "$ordinal" > "$stem.ordinal"
    git -C "$repository" diff --name-only --no-renames -z "$parent" "$revision" > "$stem.all-changed-paths"
    perl -0ne 'print if m{^(src|library)/} || /\.(lkj|rs)\x00$/;' "$stem.all-changed-paths" > "$stem.changed-paths"
    if [[ $stop_reason != none ]]; then
        printf '%s\t%s\t%s\tnot-run\t-\t0\t0\t-\t-\t-\t-\t-\t-\t%s\tnot-loaded\t%s\n' \
            "$ordinal" "$revision" "$parent" "$stop_reason" "$root" >> "$output"
        ((completed += 1))
        continue
    fi
    case_dir="$fixture/case-$(printf '%03d' "$ordinal")"
    mkdir "$case_dir"
    printf '%s\n' "$case_dir/before" > "$stem.before"
    prep=0
    if ! free_space "$parent" "$revision"; then
        prep=20
    else
        materialize "$parent" "$case_dir/before" "$stem.before-archive-paths" || prep=$?
        if [[ $prep == 0 ]]; then
            materialize "$revision" "$case_dir/after" "$stem.after-archive-paths" || prep=$?
        fi
    fi
    if [[ $prep != 0 ]]; then
        stop_reason="setup-$prep"
        printf '%s\t%s\t%s\trefused\t-\t0\t0\t-\t-\t-\t-\t-\t-\t%s\tnot-loaded\t%s\n' \
            "$ordinal" "$revision" "$parent" "$stop_reason" "$root" >> "$output"
        ((completed += 1))
        continue
    fi
    args=("$compiler" edit "$root" --source-dir "$case_dir/before"
        --against "$case_dir/after/$root"
        --library-dir "$case_dir/before/library" --against-library-dir "$case_dir/after/library"
        --inline-limit 0 --store "$fixture/store" --store-bytes "$store_bytes" --store-key "$fixture/key" --stats)
    result=0
    ((launched += 1))
    measure "$stem" "${args[@]}" || result=$?
    metrics=$(tail -n 1 "$stem.time")
    [[ $(printf '%s' "$metrics" | awk -F '\t' '{print NF}') == 4 ]] || metrics=$'-\t-\t-\t-'
    IFS=$'\t' read -r start end before after < "$stem.capture"
    lines=$(wc -l < "$stem.out")
    operations=0
    status=refused
    reason="exit-$result"
    if [[ $result == 0 && -n $active_pid ]]; then
        status=stopped-failure
        reason=undrained-owned-process-group
    elif [[ $result == 0 ]]; then
        if perl bench/compiler-strict-recent-audit.pl --case "$repository" "$stem" \
            "$case_dir/before" "$revision" "$parent" > "$stem.files.tsv" 2> "$stem.scope"; then
            operations=$(perl -ne 'if (/^replay .* verified=true operations=([1-9][0-9]*)$/) {print $1; exit}' "$stem.out")
            status=passed
            reason=desired-checked-meaning-and-full-language-scope
            ((passed += 1))
        else
            status=implementation-failure
            reason=invalid-or-incomplete-replay-proof
            stop_reason=$reason
        fi
    else
        reason=$(perl -ne 'if (/\b((?:E_[A-Z0-9_]+)|R0006)\b/) {print $1; exit}' "$stem.err")
        [[ -n $reason ]] || reason="exit-$result"
        # Allocation refusal may follow an outer E_EDIT_* line.
        if rg -q '\bR0006\b' "$stem.err"; then reason=R0006; fi
        # Only a named ordinary edit/replay/admission refusal at exit 1 may
        # continue. Every other exit (including signals and wrapper failures)
        # stops; neither a retry nor a larger resource limit is permitted.
        ordinary=false
        case "$reason" in
            E_EDIT_*|E_REPLAY_*|E_LEX|E_PARSE|E_NAME|E_TYPE|E_UNSUPPORTED|E_VISIBILITY|E_IMPORT|E_TRAIT|E_BOUND|E_PATTERN|E_LIFETIME|E_OWNERSHIP)
                ordinary=true ;;
        esac
        if [[ $result != 1 || $ordinary != true ]]; then
            status=stopped-failure
            stop_reason=$reason
        fi
        if rg -q 'E_REPLAY_(SOURCE|SPAN).*(duplicate source path|conflicting|absent from snapshot|does not match source)' "$stem.err"; then
            status=implementation-failure
            stop_reason=$reason
        fi
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\thistorical-roots\t%s\n' \
        "$ordinal" "$revision" "$parent" "$status" "$result" "$operations" "$lines" "$metrics" \
        "$before" "$after" "$reason" "$root" >> "$output"
    ((completed += 1))
    printf 'strict-recent ordinal=%s revision=%s status=%s reason=%s\n' "$ordinal" "$revision" "$status" "$reason"
    if [[ -z $active_pid && -e "$case_dir/before/.lkjscript-edit.pending" ]]; then
        # Recovery is a separately retained cleanup action, never a replay retry.
        recovery=0
        measure "$stem.recovery" "$compiler" edit "$root" --source-dir "$case_dir/before" \
            --recover --no-store --store-key "$fixture/key" || recovery=$?
        if [[ $recovery != 0 || -e "$case_dir/before/.lkjscript-edit.pending" ]]; then
            stop_reason=unrecovered-source-intent
        fi
    fi
done < bench/compiler-strict-recent.candidates.tsv
printf 'expected_probes=6\n' >> "$output.meta"
[[ $completed == 6 && $stop_reason == none ]]
