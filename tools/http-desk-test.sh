#!/usr/bin/env bash
# Isolated CLI and HTTP regression; --oracle adds the recorded Python session.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
oracle=
measure=no
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        --oracle) oracle=$2; shift 2 ;;
        --measure) measure=yes; shift ;;
        *) echo 'usage: tools/http-desk-test.sh [--compiler PATH] [--oracle PYTHON-DESK] [--measure]' >&2; exit 2 ;;
    esac
done
if [[ -n "$oracle" ]]; then test -x "$oracle"; command -v jq >/dev/null; fi
desk_work=$(mktemp -d target/programs-desk.XXXXXX)
desk_work=$(realpath "$desk_work")
server_pid=
writer_one=
writer_two=
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    for child in "$writer_one" "$writer_two"; do
        if [[ -n "$child" ]]; then kill -KILL "$child" 2>/dev/null || true; wait "$child" 2>/dev/null || true; fi
    done
    if [[ -n "$server_pid" ]]; then
        kill -KILL "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    if ((code)); then
        for log in "$desk_work/build.err" "$desk_work/service-build.err" "$desk_work/server.out" "$desk_work/server.err"; do
            [[ ! -f "$log" ]] || cat "$log" >&2
        done
    fi
    rm -rf "$desk_work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
desk_load_before=$(cat /proc/loadavg)
desk_started_ns=$(date +%s%N)
mkdir "$desk_work/native" "$desk_work/python"
bash tools/http-desk-capabilities.sh --compiler "$compiler"
"$compiler" build examples/desk.lkj --library-dir library --no-store \
    --grant args --grant state=env:DESK_STATE --grant state_env=DESK_STATE \
    --grant timezone=TZ --grant output --grant errors --grant input --grant clock \
    --grant entropy --grant exit --stats -o "$desk_work/desk" 2>"$desk_work/build.err"
"$compiler" build examples/desk.lkj --entry serve --library-dir library --no-store \
    --grant output --grant state=env:DESK_STATE --grant clock --grant id_clock \
    --grant entropy --grant listener=127.0.0.1:0 --grant ready --grant stop \
    --stats -o "$desk_work/server" 2>"$desk_work/service-build.err"
native() { env DESK_STATE="$desk_work/native" TZ=UTC0 "$desk_work/desk" "$@"; }
python_desk() { env DESK_STATE="$desk_work/python" TZ=UTC0 "$oracle" "$@"; }
id_check() { [[ "$1" =~ ^[0-9a-f]{32}$ ]]; }
seed_inbox() {
    local dir=$1 question=$2
    printf '{"id":"fixture-owner-a","t":0,"text":"返事です 😀","re":"%s"}\n{"id":"fixture-owner-b","t":60,"text":"next message"}\n' "$question" >"$dir/inbox.jsonl"
    printf '0' >"$dir/seen"
    printf '[answer to: 質問 alpha] 返事です 😀\nnext message\n' >"$dir/ping"
}
session() {
    local which=$1 dir=$2
    "$which" path >"$desk_work/$which.path"
    printf '%s\n' "$dir" >"$desk_work/path.expected"
    cmp "$desk_work/path.expected" "$desk_work/$which.path"
    local note ask
    note=$("$which" note "日本語の記録 😀" second --from agent-a)
    id_check "$note"
    ask=$("$which" ask "質問" alpha --options " A, ,Ｂ,C " --from agent-b)
    id_check "$ask"
    test "$note" != "$ask"
    seed_inbox "$dir" "$ask"
    "$which" inbox --peek >"$desk_work/$which.peek"
    printf '01-01 00:00 [answer to: 質問 alpha] 返事です 😀\n01-01 00:01 next message\n' >"$desk_work/peek.expected"
    cmp "$desk_work/peek.expected" "$desk_work/$which.peek"
    test "$(cat "$dir/seen")" = 0
    test -s "$dir/ping"
    "$which" inbox >"$desk_work/$which.inbox"
    cmp "$desk_work/peek.expected" "$desk_work/$which.inbox"
    test "$(cat "$dir/seen")" = 2
    test ! -e "$dir/ping"
    "$which" inbox --peek >"$desk_work/$which.empty"
    test ! -s "$desk_work/$which.empty"
    "$which" close "$ask" --from reviewer >"$desk_work/$which.close"
    test ! -s "$desk_work/$which.close"
    test -f "$dir/alive"
    test "$(wc -l <"$dir/notes.jsonl")" = 3
}
session native "$desk_work/native"
# Publication must not follow an in-root leftover symlink into acknowledged data.
mkdir "$desk_work/symlink-state"
printf 'protected contents\n' >"$desk_work/symlink-state/protected.txt"
cp "$desk_work/symlink-state/protected.txt" "$desk_work/protected.expected"
env DESK_STATE="$desk_work/symlink-state" TZ=UTC0 "$desk_work/desk" note first >/dev/null
cp "$desk_work/symlink-state/notes.jsonl" "$desk_work/first.expected"
ln -s protected.txt "$desk_work/symlink-state/.notes.jsonl-next"
env DESK_STATE="$desk_work/symlink-state" TZ=UTC0 "$desk_work/desk" note second >/dev/null
cmp "$desk_work/protected.expected" "$desk_work/symlink-state/protected.txt"
head -n 1 "$desk_work/symlink-state/notes.jsonl" >"$desk_work/first.actual"
cmp "$desk_work/first.expected" "$desk_work/first.actual"
cp "$desk_work/symlink-state/notes.jsonl" "$desk_work/two.expected"
ln -s notes.jsonl "$desk_work/symlink-state/.notes.jsonl-next"
env DESK_STATE="$desk_work/symlink-state" TZ=UTC0 "$desk_work/desk" note third >/dev/null
head -n 2 "$desk_work/symlink-state/notes.jsonl" >"$desk_work/two.actual"
cmp "$desk_work/two.expected" "$desk_work/two.actual"
test "$(wc -l <"$desk_work/symlink-state/notes.jsonl")" = 3
test ! -e "$desk_work/symlink-state/.notes.jsonl-next"
cp "$desk_work/symlink-state/notes.jsonl" "$desk_work/three.expected"
rm "$desk_work/symlink-state/alive"
ln -s notes.jsonl "$desk_work/symlink-state/alive"
env DESK_STATE="$desk_work/symlink-state" TZ=UTC0 "$desk_work/desk" note fourth >/dev/null
head -n 3 "$desk_work/symlink-state/notes.jsonl" >"$desk_work/three.actual"
cmp "$desk_work/three.expected" "$desk_work/three.actual"
test "$(wc -l <"$desk_work/symlink-state/notes.jsonl")" = 4
test ! -L "$desk_work/symlink-state/alive"
test ! -s "$desk_work/symlink-state/alive"
if [[ -n "$oracle" ]]; then
    session python_desk "$desk_work/python"
    # Preserve row order and map IDs/references to record ordinal. Only IDs,
    # times and insignificant JSON dictionary/whitespace ordering are relaxed.
    canonical() {
        jq -S -n --slurpfile notes "$1/notes.jsonl" --slurpfile inbox "$1/inbox.jsonl" '
          (reduce range(0; $notes|length) as $n ({}; .[$notes[$n].id] = ("note"+($n|tostring)))) as $ids |
          {notes: ($notes|map(.id=$ids[.id] | .t=0 | if has("ref") then .ref=($ids[.ref] // .ref) else . end)),
           inbox: ($inbox|map(del(.id,.t) | if has("re") then .re=($ids[.re] // .re) else . end))}'
    }
    canonical "$desk_work/native" >"$desk_work/native.json"
    canonical "$desk_work/python" >"$desk_work/python.json"
    cmp "$desk_work/native.json" "$desk_work/python.json"
    cmp "$desk_work/native/seen" "$desk_work/python/seen"
    mkdir "$desk_work/from-native" "$desk_work/from-python"
    cp "$desk_work/native/notes.jsonl" "$desk_work/native/inbox.jsonl" "$desk_work/from-native/"
    cp "$desk_work/python/notes.jsonl" "$desk_work/python/inbox.jsonl" "$desk_work/from-python/"
    env DESK_STATE="$desk_work/from-native" TZ=JST-9 "$oracle" inbox --peek >"$desk_work/read-native"
    env DESK_STATE="$desk_work/from-python" TZ=JST-9 "$desk_work/desk" inbox --peek >"$desk_work/read-python"
    cmp "$desk_work/read-native" "$desk_work/read-python"
    printf '01-01 09:00 [answer to: 質問 alpha] 返事です 😀\n01-01 09:01 next message\n' >"$desk_work/jst.expected"
    cmp "$desk_work/jst.expected" "$desk_work/read-native"
    printf 'ok desk oracle session: note/ask/close/inbox/peek/path, Japanese, UTC/JST, mutual file reading\n'
fi
env DESK_STATE="$desk_work/native" TZ=UTC0 "$desk_work/desk" note writer-one --from concurrent >"$desk_work/writer-one" &
writer_one=$!
env DESK_STATE="$desk_work/native" TZ=UTC0 "$desk_work/desk" note writer-two --from concurrent >"$desk_work/writer-two" &
writer_two=$!
wait "$writer_one"
writer_one=
wait "$writer_two"
writer_two=
test "$(wc -l <"$desk_work/native/notes.jsonl")" = 5
test "$(cat "$desk_work/writer-one")" != "$(cat "$desk_work/writer-two")"
if [[ -n "$oracle" ]]; then
    env DESK_STATE="$desk_work/python" TZ=UTC0 "$oracle" note writer-one --from concurrent >"$desk_work/python-one" &
    writer_one=$!
    env DESK_STATE="$desk_work/python" TZ=UTC0 "$oracle" note writer-two --from concurrent >"$desk_work/python-two" &
    writer_two=$!
    wait "$writer_one"
    writer_one=
    wait "$writer_two"
    writer_two=
    for implementation in native python; do
        jq -S -s 'map(select(.from=="concurrent")|del(.id,.t))|sort_by(.text)' \
            "$desk_work/$implementation/notes.jsonl" >"$desk_work/$implementation.concurrent"
    done
    cmp "$desk_work/native.concurrent" "$desk_work/python.concurrent"
fi
printf 'standard input 日本語\n' | native note --stdin --from stdin-case >"$desk_work/stdin.id"
id_check "$(cat "$desk_work/stdin.id")"
grep -Fq 'standard input 日本語' "$desk_work/native/notes.jsonl"
status=0
native unknown >"$desk_work/usage" || status=$?
test "$status" = 2
status=0
env -u DESK_STATE "$desk_work/desk" path >"$desk_work/missing.out" 2>"$desk_work/missing.err" || status=$?
test "$status" != 0
grep -Fq 'Directory' "$desk_work/missing.err"
question=$(native ask "page question 日本語" --from http)
env DESK_STATE="$desk_work/native" "$desk_work/server" >"$desk_work/server.out" 2>"$desk_work/server.err" &
server_pid=$!
for ((attempt=0;attempt<6000;attempt++)); do
    port=$(awk '$1=="port" {print $2}' "$desk_work/server.out")
    [[ -z "$port" ]] || break
    kill -0 "$server_pid"
    sleep 0.01
done
test -n "$port"
"$compiler" build tools/http-desk-cases.lkj --library-dir library --no-store \
    --grant output --grant input --grant target=127.0.0.1:"$port" \
    --grant page=examples/desk -o "$desk_work/client"
printf '%s\n' "$question" | timeout --kill-after=5s 60s "$desk_work/client"
native inbox >"$desk_work/acknowledged"
test ! -e "$desk_work/native/ping"
printf 'retry-read\n' | timeout --kill-after=5s 60s "$desk_work/client"
test ! -e "$desk_work/native/ping"
test "$(wc -l <"$desk_work/native/inbox.jsonl")" = 4
awk '/^VmRSS:/ {print "Desk service RSS: " $2 " KiB"}' "/proc/$server_pid/status"
kill -TERM "$server_pid"
for ((attempt=0;attempt<1500;attempt++)); do
    kill -0 "$server_pid" 2>/dev/null || break
    sleep 0.01
done
if kill -0 "$server_pid" 2>/dev/null; then echo 'desk did not drain' >&2; exit 1; fi
wait "$server_pid"
server_pid=
grep -q '^stopped$' "$desk_work/server.out"
if [[ "$measure" = yes ]]; then
    test -n "$oracle"
    for rows in 0 10000; do
        mkdir "$desk_work/bench-$rows"
        awk -v count="$rows" 'BEGIN { for(i=0;i<count;i++) printf "{\"id\":\"prefill-%08d\",\"t\":0,\"kind\":\"note\",\"from\":\"fixture\",\"text\":\"state size witness\"}\n",i }' \
            >"$desk_work/bench-$rows/notes.jsonl"
        for implementation in native python; do
            mkdir "$desk_work/bench-$rows-$implementation"
            cp "$desk_work/bench-$rows/notes.jsonl" "$desk_work/bench-$rows-$implementation/"
            if [[ "$implementation" = native ]]; then executable=$desk_work/desk; else executable=$oracle; fi
            before=$(cat /proc/loadavg)
            start=$(date +%s%N)
            for ((sample=0;sample<100;sample++)); do
                env DESK_STATE="$desk_work/bench-$rows-$implementation" TZ=UTC0 "$executable" note measurement --from parity >/dev/null
            done
            elapsed=$(($(date +%s%N)-start))
            printf 'Desk note %s, initial rows %s: 100 commands %s ns; load %s -> %s\n' \
                "$implementation" "$rows" "$elapsed" "$before" "$(cat /proc/loadavg)"
        done
    done
fi
printf 'Desk CLI bytes: %s; service bytes: %s\n' "$(stat -c %s "$desk_work/desk")" "$(stat -c %s "$desk_work/server")"
printf 'Desk command build stats: '
cat "$desk_work/build.err"
printf 'Desk service build stats: '
cat "$desk_work/service-build.err"
printf 'ok desk isolated CLI/concurrency/HTTP (%s ms, load %s -> %s)\n' \
    "$((($(date +%s%N)-desk_started_ns)/1000000))" "$desk_load_before" "$(cat /proc/loadavg)"
