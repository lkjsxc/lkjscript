#!/usr/bin/env bash
# Optional durable-publication campaign. Deliberately outside the ordinary gate.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
oracle=
wanted=96
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        --oracle) oracle=$2; shift 2 ;;
        --kills) wanted=$2; shift 2 ;;
        *) echo 'usage: tools/http-desk-crash.sh [--compiler PATH] [--oracle DESK] [--kills N]' >&2; exit 2 ;;
    esac
done
[[ "$wanted" =~ ^[1-9][0-9]*$ ]] || exit 2
command -v jq >/dev/null
[[ -z "$oracle" || -x "$oracle" ]]
umask 077
work=$(mktemp -d target/programs-desk-crash.XXXXXX)
work=$(realpath "$work")
child_pid=
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$child_pid" ]]; then
        kill -KILL "$child_pid" 2>/dev/null || true
        wait "$child_pid" 2>/dev/null || true
    fi
    if ((code)); then
        for log in "$work/build.err" "$work/child.err" "$work/recovery.err"; do
            [[ ! -f "$log" ]] || cat "$log" >&2
        done
    fi
    rm -rf "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
load_before=$(cat /proc/loadavg)
started=$(date +%s%N)
mkdir "$work/state"
awk 'BEGIN { for(i=0;i<10000;i++) printf "{\"id\":\"prefill-%08d\",\"t\":0,\"kind\":\"note\",\"from\":\"fixture\",\"text\":\"state size witness\"}\n",i }' >"$work/prefix"
cp "$work/prefix" "$work/state/notes.jsonl"
prefix_bytes=$(stat -c %s "$work/prefix")
: >"$work/acknowledged"
build_load=$(cat /proc/loadavg)
build_start=$(date +%s%N)
"$compiler" build examples/desk.lkj --library-dir library --no-store \
    --grant args --grant state=env:DESK_STATE --grant state_env=DESK_STATE \
    --grant timezone=TZ --grant output --grant errors --grant input --grant clock \
    --grant entropy --grant exit --stats -o "$work/desk" 2>"$work/build.err"
printf 'Desk crash build: %s ms; load %s -> %s\n' \
    "$((($(date +%s%N)-build_start)/1000000))" "$build_load" "$(cat /proc/loadavg)"
acknowledge() {
    local output=$1 id=
    [[ -f "$output" && $(stat -c %s "$output") = 33 ]] || return 1
    IFS= read -r id <"$output" || return 1
    [[ "$id" =~ ^[0-9a-f]{32}$ ]] || return 1
    printf '%s\n' "$id" >>"$work/acknowledged"
}
validate() {
    # This comparison is over original bytes, independently of JSON normalization.
    head -c "$prefix_bytes" "$work/state/notes.jsonl" >"$work/observed-prefix"
    cmp "$work/prefix" "$work/observed-prefix"
    jq -er '.id | strings' "$work/state/notes.jsonl" >"$work/published-ids"
    sort "$work/published-ids" >"$work/published-sorted"
    uniq -d "$work/published-sorted" >"$work/duplicates"
    test ! -s "$work/duplicates"
    sort -u "$work/acknowledged" >"$work/acknowledged-sorted"
    comm -23 "$work/acknowledged-sorted" "$work/published-sorted" >"$work/missing"
    if [[ -s "$work/missing" ]]; then
        echo 'desk crash lost an acknowledged id' >&2
        return 1
    fi
}
# Different windows include startup, copying, fsync, rename, acknowledgement,
# and the later alive publication. Count confirmed SIGKILL exits, not attempts.
delays=(0 0.0005 0.001 0.002 0.004 0.006 0.008 0.012 0.020)
killed=0
completed=0
attempt=0
acknowledged_kills=0
while ((killed < wanted)); do
    delay=${delays[$((attempt % ${#delays[@]}))]}
    attempt=$((attempt+1))
    # A zero-delay kill may precede the child shell opening its redirections.
    # Clear captures in the parent so no previous acknowledgement is counted.
    : >"$work/child.out"
    : >"$work/child.err"
    env DESK_STATE="$work/state" TZ=UTC0 "$work/desk" note "crash attempt $attempt" --from crash \
        >"$work/child.out" 2>"$work/child.err" &
    child_pid=$!
    if [[ "$delay" != 0 ]]; then sleep "$delay"; fi
    kill -KILL "$child_pid" 2>/dev/null || true
    status=0
    wait "$child_pid" 2>/dev/null || status=$?
    child_pid=
    if ((status == 137)); then
        killed=$((killed+1))
        if acknowledge "$work/child.out"; then acknowledged_kills=$((acknowledged_kills+1)); fi
    elif ((status == 0)); then
        completed=$((completed+1))
        acknowledge "$work/child.out"
    else
        printf 'unexpected desk child status %s at attempt %s\n' "$status" "$attempt" >&2
        exit 1
    fi
    validate
    env DESK_STATE="$work/state" TZ=UTC0 "$work/desk" note "recovery $attempt" --from recovery \
        >"$work/recovery.out" 2>"$work/recovery.err"
    acknowledge "$work/recovery.out"
done
validate
if [[ -n "$oracle" ]]; then
    IFS= read -r reference <"$work/acknowledged"
    jq -nc --arg re "$reference" '{id:"owner-oracle",t:0,text:"durability 日本語 😀",re:$re}' >"$work/state/inbox.jsonl"
    env DESK_STATE="$work/state" TZ=UTC0 "$work/desk" inbox --peek >"$work/native-inbox"
    env DESK_STATE="$work/state" TZ=UTC0 "$oracle" inbox --peek >"$work/oracle-inbox"
    cmp "$work/native-inbox" "$work/oracle-inbox"
    printf 'Desk crash oracle: original Python reads every surviving JSONL row and the same referenced answer\n'
fi
printf 'Desk crash: %s confirmed kills in %s attempts, %s completed attempts, %s killed after acknowledgement, %s acknowledged ids retained, %s published rows, %s recovery appends; prefix %s bytes unchanged\n' \
    "$killed" "$attempt" "$completed" "$acknowledged_kills" "$(wc -l <"$work/acknowledged-sorted")" \
    "$(wc -l <"$work/published-ids")" "$attempt" "$prefix_bytes"
printf 'Desk crash total: %s ms; load %s -> %s; all owned children reaped and private files removed\n' \
    "$((($(date +%s%N)-started)/1000000))" "$load_before" "$(cat /proc/loadavg)"
