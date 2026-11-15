#!/usr/bin/env bash
# Short replay regression; --campaign also proves 990 idle/10 active peers and slow-reader expiry.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
peers=20
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        --campaign) peers=1000; shift ;;
        *) echo 'usage: tools/http-desk-events.sh [--compiler PATH] [--campaign]' >&2; exit 2 ;;
    esac
done
umask 077
work=$(mktemp -d target/programs-desk-events.XXXXXX)
work=$(realpath "$work")
server=
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$server" ]]; then kill -KILL "$server" 2>/dev/null || true; wait "$server" 2>/dev/null || true; fi
    if ((status)); then
        for log in "$work/server-build.err" "$work/client-build.err" "$work/server.out" "$work/server.err" "$work/client.out" "$work/client.err"; do
            [[ ! -f "$log" ]] || cat "$log" >&2
        done
    fi
    rm -rf "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
load_before=$(cat /proc/loadavg)
started=$(date +%s%N)
mkdir "$work/state" "$work/board" "$work/handoff"
: >"$work/state/notes.jsonl"
: >"$work/state/inbox.jsonl"
printf 'desk-board 1 %s\nproof running\n' "$(date +%s)" >"$work/board/board.txt"
printf 'Disposable SSE handoff\n' >"$work/handoff/handoff-proof.md"
"$compiler" build examples/desk --library-dir library --no-store --stats -o "$work/server" 2>"$work/server-build.err"
# Readiness polling can run before the background child performs redirections.
: >"$work/server.out"
: >"$work/server.err"
env DESK_STATE="$work/state" DESK_BOARD="$work/board" DESK_HANDOFF="$work/handoff" \
    "$work/server" >"$work/server.out" 2>"$work/server.err" &
server=$!
for ((attempt=0;attempt<6000;attempt++)); do
    port=$(awk '$1=="port" {print $2}' "$work/server.out")
    [[ -z "$port" ]] || break
    kill -0 "$server"
    sleep 0.01
done
test -n "$port"
"$compiler" build tools/http-desk-events.lkj --library-dir library --no-store \
    --grant output --grant input --grant target=127.0.0.1:"$port" --grant state="$work/state" \
    --grant clock --grant ready -o "$work/client" 2>"$work/client-build.err"
proof_before=$(cat /proc/loadavg)
printf '%s\n' "$peers" | timeout --kill-after=5s 120s "$work/client" >"$work/client.out" 2>"$work/client.err"
proof_after=$(cat /proc/loadavg)
while IFS= read -r line; do
    printf '%s; proof load %s -> %s\n' "$line" "$proof_before" "$proof_after"
done <"$work/client.out"
awk '/^VmRSS:/ {print "Desk event service RSS: " $2 " KiB"}' "/proc/$server/status"
kill -TERM "$server"
for ((attempt=0;attempt<1000;attempt++)); do
    kill -0 "$server" 2>/dev/null || break
    sleep 0.01
done
if kill -0 "$server" 2>/dev/null; then echo 'desk events did not drain' >&2; exit 1; fi
wait "$server"
server=
grep -q '^stopped$' "$work/server.out"
printf 'ok desk events suite: %s ms; load %s -> %s; peers %s\n' \
    "$((($(date +%s%N)-started)/1000000))" "$load_before" "$(cat /proc/loadavg)" "$peers"
