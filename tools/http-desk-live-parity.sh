#!/usr/bin/env bash
# Only GET touches the live desk. Every published fixture belongs to this test.
set -euo pipefail
cd "$(dirname "$0")/.."
port=$1
fixture=$2
[[ "$port" =~ ^[0-9]+$ && "$fixture" == "$PWD"/target/* ]] || exit 2
command -v jq >/dev/null
snapshot=$fixture/live-snapshot.json
stage_snapshot() {
    # Reconstruct exactly the visible state; older hidden rows are irrelevant to
    # GET parity. Closed asks need a synthetic close row outside the feed.
    jq -c '([.feed[]|select(.kind!="owner")]+.asks)|unique_by(.id)|.[]|del(.open,.read)' "$snapshot" >"$fixture/notes-next"
    jq -c '.feed[]|select(.kind=="ask" and .open==false)|{id:("parity-close-"+.id),t:0,kind:"close",ref:.id,from:"parity"}' "$snapshot" >>"$fixture/notes-next"
    jq -c '.feed|reverse|.[]|select(.kind=="owner")|del(.kind,.read,.open)' "$snapshot" >"$fixture/inbox-next"
    jq -jr '[.feed[]|select(.kind=="owner" and .read)]|length|tostring' "$snapshot" >"$fixture/seen-next"
    { printf 'desk-board 1 %s\n' "$(date +%s)"; jq -r '.jobs[]' "$snapshot"; } >"$fixture/board-next"
    jq -jr '.detail' "$snapshot" >"$fixture/handoff-next"
    exec {writer}>"$fixture/native/.lock"
    flock "$writer"
    mv "$fixture/notes-next" "$fixture/native/notes.jsonl"
    mv "$fixture/inbox-next" "$fixture/native/inbox.jsonl"
    mv "$fixture/seen-next" "$fixture/native/seen"
    mv "$fixture/board-next" "$fixture/board/board.txt"
    mv "$fixture/handoff-next" "$fixture/handoff/handoff-live.md"
    flock -u "$writer"
    exec {writer}>&-
}
reads=0
for path in /state /proxy/123/state///?query=1 /state?x=1; do
    curl --fail --silent --show-error --path-as-is --max-time 10 \
        "http://127.0.0.1:8911$path" -D "$fixture/origin.headers" -o "$snapshot"
    stage_snapshot
    curl --fail --silent --show-error --path-as-is --max-time 10 \
        "http://127.0.0.1:$port$path" -D "$fixture/candidate.headers" -o "$fixture/candidate.json"
    jq -S '{asks,feed,jobs,detail}' "$snapshot" >"$fixture/expected.json"
    jq -S '{asks,feed,jobs,detail}' "$fixture/candidate.json" >"$fixture/actual.json"
    cmp "$fixture/expected.json" "$fixture/actual.json"
    for headers in "$fixture/origin.headers" "$fixture/candidate.headers"; do
        grep -iq '^Content-Type: application/json' "$headers"
        grep -iq '^Cache-Control: no-store' "$headers"
    done
    reads=$((reads+1))
done
for path in / /proxy/123/ /unknown-read-path; do
    curl --fail --silent --show-error --path-as-is --max-time 10 \
        "http://127.0.0.1:8911$path" -D "$fixture/origin.headers" -o "$fixture/origin.html"
    curl --fail --silent --show-error --path-as-is --max-time 10 \
        "http://127.0.0.1:$port$path" -D "$fixture/candidate.headers" -o "$fixture/candidate.html"
    cmp examples/desk/index.html "$fixture/candidate.html"
    grep -Fq '<title>連絡帳</title>' "$fixture/origin.html"
    for headers in "$fixture/origin.headers" "$fixture/candidate.headers"; do
        grep -iq '^Content-Type: text/html; charset=utf-8' "$headers"
        grep -iq '^Cache-Control: no-store' "$headers"
    done
    reads=$((reads+1))
done
printf 'ok desk live GET parity: %s routes, complete visible feed/asks/board/handoff, exact reviewed candidate page; no live writes\n' "$reads"
