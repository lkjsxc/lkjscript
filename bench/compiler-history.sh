#!/usr/bin/env bash
# Replay/check complete compiler roots from recorded history, one bounded worker.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# -lt 5 ]]; then
    printf 'usage: %s replay|compare REPOSITORY BINARY PRIVATE_KEY OUTPUT [COUNT] [SCAN]\n' "$0" >&2
    exit 2
fi
mode=$1
repository=$(realpath "$2")
compiler=$(realpath "$3")
private_key=$(realpath "$4")
output=$(realpath -m "$5")
count=${6:-30}
scan=${7:-300}
[[ $mode == replay || $mode == compare ]] || exit 2
[[ $count =~ ^[1-9][0-9]*$ && $scan =~ ^[1-9][0-9]*$ ]] || exit 2
memory_bytes=${LKJSCRIPT_HISTORY_MEMORY_BYTES:-4294967296}
timeout_seconds=${LKJSCRIPT_HISTORY_TIMEOUT_SECONDS:-240}
store_bytes=${LKJSCRIPT_HISTORY_STORE_BYTES:-2147483648}
history_head=$(git -C "$repository" rev-parse --verify "${LKJSCRIPT_HISTORY_HEAD:-HEAD}^{commit}")
compiler_bytes=$(stat -c %s "$compiler")
archive_blocks() {
    git -C "$repository" ls-tree -rl "$1" -- src library tools tests |
        awk '$4 ~ /^[0-9]+$/ { bytes += int(($4 + 4095) / 4096) * 4096 } END { printf "%.0f\n", bytes }'
}
free_space() {
    local before_bytes after_bytes allowance required available
    before_bytes=$(archive_blocks "$1") || return 1
    after_bytes=$(archive_blocks "$2") || return 1
    # Two measured trees, doubled for retained .rs plus .lkj copies; a 128 MiB
    # floor covers directory metadata and small revisions. Reserve 2x the store
    # budget for authenticated envelopes/block overhead, plus binary and logs.
    allowance=$((2 * (before_bytes + after_bytes)))
    if (( allowance < 134217728 )); then allowance=134217728; fi
    required=$((2 * store_bytes + allowance + compiler_bytes + 134217728))
    available=$(df -B1 --output=avail "$PWD" | tail -n 1)
    printf '%s\t%s\t%s\t%s\t%s\n' "$available" "$required" "$allowance" "$1" "$2" >> "$output.space"
    if (( available < required )); then
        printf 'E_HISTORY_SPACE available=%s required=%s measured_tree_allowance=%s\n' "$available" "$required" "$allowance" >&2
        return 1
    fi
}
: > "$output.space"
free_space "$history_head" "$history_head" || exit 2
fixture=$(mktemp -d "$PWD/target/compiler-history.XXXXXX")
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
mkdir "$fixture/store" "$output.logs"
compiler_origin=$compiler
cp -- "$compiler" "$fixture/compiler"
compiler="$fixture/compiler"
git -C "$repository" log -n "$scan" --format='%H %P' "$history_head" > "$fixture/revisions"
printf 'ordinal\trevision\tparent\tstatus\tprojection\toperations\tstdout_lines\twall_s\tuser_s\tsys_s\tpeak_kib\tload_before\tload_after\treason\n' > "$output"
printf 'mode=%s root=src/main.lkj scan=%s wanted=%s memory_bytes=%s timeout_seconds=%s store_bytes=%s\n' \
    "$mode" "$scan" "$count" "$memory_bytes" "$timeout_seconds" "$store_bytes" > "$output.meta"
printf 'history_head=%s\ncompiler_origin=%s\n' "$history_head" "$compiler_origin" >> "$output.meta"
sha256sum "$compiler" >> "$output.meta"

# Listing/archiving is restricted to these four trees. Never copy repository
# metadata, generation-one, data/services, releases or unrelated projects.
materialize() {
    local revision=$1 destination=$2
    local areas=() area file projected
    mkdir "$destination" || return 16
    for area in src library tools tests; do
        if git -C "$repository" cat-file -e "$revision:$area" 2>/dev/null; then areas+=("$area"); fi
    done
    [[ ${#areas[@]} -gt 0 ]] || return 10
    git -C "$repository" ls-tree -r --name-only "$revision" -- "${areas[@]}" > "$fixture/archive-paths" || return 15
    if rg -q '(^|/)\.generation-one(/|$)' "$fixture/archive-paths"; then
        return 11
    fi
    git -C "$repository" archive --format=tar "$revision" -- "${areas[@]}" | tar -xf - -C "$destination" || return 15
    if [[ -n $(find "$destination" -type l -print -quit) ]]; then return 12; fi
    while IFS= read -r -d '' file; do
        projected="${file%.rs}.lkj"
        if [[ -e $projected ]]; then
            cmp -s "$file" "$projected" || return 13
        else
            cp -- "$file" "$projected" || return 17
            projection=legacy-rs-copies
        fi
    done < <(rg --files --no-ignore -0 -g '*.rs' "$destination" || true)
    [[ -f "$destination/src/main.lkj" && -f "$destination/library/lib.lkj" ]] || return 14
}
if [[ $mode == replay ]]; then
    # Prefer bounded historical changes, without pretending ineligible revisions
    # were reproduced. The compiler/desired-image check makes the actual choice.
    : > "$fixture/candidates"
    while read -r revision parent rest; do
        [[ -n ${parent:-} ]] || continue
        changes=$(git -C "$repository" diff --numstat "$parent" "$revision" -- src library |
            awk '{ if ($1 ~ /^[0-9]+$/ && $2 ~ /^[0-9]+$/) total += $1 + $2; else total += 1000000 } END { print total + 0 }')
        [[ $changes != 0 ]] || continue
        printf '%012d\t%s\t%s\n' "$changes" "$revision" "$parent" >> "$fixture/candidates"
    done < "$fixture/revisions"
    sort -n "$fixture/candidates" > "$fixture/ordered"
else
    awk -v wanted="$count" 'NF >= 2 { print NR "\t" $1 "\t" $2; if (++seen == wanted) exit; }' "$fixture/revisions" > "$fixture/ordered"
fi
passed=0
attempted=0
while IFS=$'\t' read -r weight revision parent; do
    ((attempted += 1))
    rm -rf "$fixture/before" "$fixture/after"
    projection=native-lkj
    if ! free_space "$parent" "$revision"; then
        printf '%s\t%s\t%s\trefused\t%s\t0\t0\t-\t-\t-\t-\t-\t-\tE_HISTORY_SPACE\n' \
            "$attempted" "$revision" "$parent" "$projection" >> "$output"
        break
    fi
    prep=0
    materialize "$parent" "$fixture/before" || prep=$?
    if [[ $prep == 0 ]]; then materialize "$revision" "$fixture/after" || prep=$?; fi
    stem="$output.logs/$(printf '%03d' "$attempted")-$revision"
    if [[ $prep != 0 ]]; then
        printf '%s\t%s\t%s\tineligible\t%s\t0\t0\t-\t-\t-\t-\t-\t-\tarchive-or-projection-%s\n' \
            "$attempted" "$revision" "$parent" "$projection" "$prep" >> "$output"
        continue
    fi
    load_before=$(cut -d ' ' -f 1 /proc/loadavg)
    if [[ $mode == replay ]]; then
        args=(edit src/main.lkj --source-dir "$fixture/before" --library-dir "$fixture/before/library"
            --against "$fixture/after/src/main.lkj" --against-library-dir "$fixture/after/library")
    else
        args=(compare "$fixture/after/src/main.lkj" --library-dir "$fixture/after/library"
            --against "$fixture/before/src/main.lkj" --against-library-dir "$fixture/before/library" --limit 20)
    fi
    args+=(--store "$fixture/store" --store-bytes "$store_bytes" --store-key "$private_key" --stats)
    result=0
    (
        cd "$fixture"
        /usr/bin/time -q -f '%e\t%U\t%S\t%M' -o "$stem.time" \
            prlimit --as="$memory_bytes" -- timeout --kill-after=5s "$timeout_seconds" \
            "$compiler" "${args[@]}" > "$stem.out" 2> "$stem.err"
    ) || result=$?
    load_after=$(cut -d ' ' -f 1 /proc/loadavg)
    lines=$(wc -l < "$stem.out")
    status=ineligible
    operations=0
    reason="exit-$result"
    if [[ $result == 0 ]]; then
        if [[ $mode == replay ]]; then
            if operations=$(perl -ne 'if (/^replay desired=([0-9a-f]{64}) candidate=([0-9a-f]{64}) verified=true operations=([0-9]+)$/ && $1 eq $2) { $count=$3; } END { if ($count) { print $count; } else { exit 1; } }' "$stem.out"); then
                status=passed
                reason=desired-checked-meaning-identical
            else
                reason=missing-replay-equality-proof
            fi
        elif [[ $lines -le 30 ]]; then
            status=passed
            reason=bounded-comparison
        else
            reason=comparison-exceeded-30-lines
        fi
    else
        reason=$(perl -ne 'if (/(E_[A-Z0-9_]+)/) { print $1; exit; }' "$stem.err")
        [[ -n $reason ]] || reason="exit-$result"
        if [[ -e "$fixture/before/.lkjscript-edit.pending" ]]; then
            (
                cd "$fixture"
                "$compiler" edit src/main.lkj --source-dir "$fixture/before" --library-dir "$fixture/before/library" \
                    --recover --no-store --store-key "$private_key"
            ) > "$stem.recovery" 2>&1
        fi
    fi
    metrics=$(tail -n 1 "$stem.time")
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$attempted" "$revision" "$parent" "$status" "$projection" "$operations" "$lines" "$metrics" \
        "$load_before" "$load_after" "$reason" >> "$output"
    if [[ $status == passed ]]; then ((passed += 1)); fi
    printf 'history mode=%s attempted=%s passed=%s revision=%s result=%s reason=%s\n' \
        "$mode" "$attempted" "$passed" "$revision" "$status" "$reason"
    if [[ $mode == replay && $passed -ge $count ]]; then break; fi
done < "$fixture/ordered"
printf 'attempted=%s passed=%s wanted=%s\n' "$attempted" "$passed" "$count" >> "$output.meta"
[[ $passed -ge $count ]]
