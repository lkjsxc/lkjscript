#!/bin/sh
# Duration and watchdogs are test policy. The clients are native lkjscript.
set -eu
cd "$(dirname "$0")/.."
compiler=target/lkjscript
seconds=3600
engine=native
pipeline=32
load_path=language.md
verification=full
profile_directory=target/http-profiles
while [ "$#" -gt 0 ]; do
    case "$1" in
        --compiler|--seconds|--out|--engine|--pipeline|--path|--verification)
            [ "$#" -ge 2 ] || { echo "missing value for $1" >&2; exit 2; }
            case "$1" in
                --compiler) compiler=$2 ;;
                --seconds) seconds=$2 ;;
                --out) profile_directory=$2 ;;
                --engine) engine=$2 ;;
                --pipeline) pipeline=$2 ;;
                --path) load_path=$2 ;;
                --verification) verification=$2 ;;
            esac
            shift 2 ;;
        *) echo "usage: sh tools/http-test.sh [--compiler PATH] [--seconds N] [--out DIRECTORY] [--engine native|interpreter] [--pipeline N] [--path RELATIVE_PATH] [--verification full|framing]" >&2; exit 2 ;;
    esac
done
case "$seconds" in ''|*[!0-9]*) echo "seconds must be a nonnegative integer" >&2; exit 2;; esac
case "$pipeline" in ''|*[!0-9]*|0) echo "pipeline must be a positive integer" >&2; exit 2;; esac
case "$engine" in native|interpreter) ;; *) echo "engine must be native or interpreter" >&2; exit 2;; esac
case "$verification" in full|framing) ;; *) echo "verification must be full or framing" >&2; exit 2;; esac
# Reuse closed compiler inputs and pure case results only. Listener/file traffic
# below still executes against fresh state on every invocation.
compile_request() {
    if [ -n "${LKJSCRIPT_STORE_KEY:-}" ]; then
        "$compiler" "$@" --store "${LKJSCRIPT_STORE:-target/store}" --store-key "$LKJSCRIPT_STORE_KEY"
    else
        "$compiler" "$@"
    fi
}
# Background jobs must replace their shell so $! owns the real compiler/broker.
exec_request() {
    if [ -n "${LKJSCRIPT_STORE_KEY:-}" ]; then
        exec "$compiler" "$@" --store "${LKJSCRIPT_STORE:-target/store}" --store-key "$LKJSCRIPT_STORE_KEY"
    else
        exec "$compiler" "$@"
    fi
}
http_work=$(mktemp -d target/http-test.XXXXXX)
server_pid=
client_pid=
started_pids=
port=
cleanup() {
    trap - EXIT HUP INT TERM
    if [ -n "$client_pid" ]; then
        kill "$client_pid" 2>/dev/null || :
        wait "$client_pid" 2>/dev/null || :
    fi
    if [ -n "$server_pid" ]; then
        kill "$server_pid" 2>/dev/null || :
        wait "$server_pid" 2>/dev/null || :
    fi
    if [ -s "$http_work/server.err" ]; then cat "$http_work/server.err" >&2; fi
    if [ -s "$http_work/client.err" ]; then cat "$http_work/client.err" >&2; fi
    rm -rf "$http_work"
    gone=yes
    for pid in $started_pids; do
        if kill -0 "$pid" 2>/dev/null; then gone=no; fi
    done
    printf 'HTTP cleanup: started PIDs%s; all gone %s; temporary directory removed; port %s released\n' "$started_pids" "$gone" "$port"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM

mkdir "$http_work/docs"
cp docs/*.md "$http_work/docs/"
printf 'lkjscript\n' > "$http_work/docs/small.txt"
: > "$http_work/docs/empty.bin"
awk 'BEGIN { for (i=0; i<8192; i++) printf "%c", 33+i%94 }' > "$http_work/docs/chunk.bin"
cp "$http_work/docs/chunk.bin" "$http_work/docs/chunk-tail.bin"
printf '\000\377\n' >> "$http_work/docs/chunk-tail.bin"
printf 'outside the granted root\n' > "$http_work/outside.txt"
ln -s language.md "$http_work/docs/link-inside"
ln -s ../outside.txt "$http_work/docs/link-outside"
# Interpreting the large-header parser needs explicit application wall time.
# Native and adversarial runs retain the default 60/2/2-second deadlines.
if [ "$engine" = interpreter ]; then
    printf '60\n120\n120\n' > "$http_work/docs/.http-policy"
fi
bash tools/programs-unit-cases.sh "$compiler" library/http_protocol.lkj
bash tools/programs-unit-cases.sh "$compiler" library/http_policy.lkj
bash tools/programs-unit-cases.sh "$compiler" tools/http_client.lkj
if [ "$engine" = native ]; then
    compile_request build examples/http.lkj --profile \
        --grant output --grant "root=$http_work/docs" \
        --grant listener=127.0.0.1:0 --grant clock --grant ready --grant stop -o "$http_work/server"
fi
compile_request build tools/http-test.lkj -o "$http_work/client"
: > "$http_work/server.out"
: > "$http_work/server.err"
: > "$http_work/client.out"
: > "$http_work/client.err"
: > "$http_work/memory"
# A closed stdin forces accept to exercise promotion of descriptor zero.
if [ "$engine" = native ]; then
    "$http_work/server" <&- >"$http_work/server.out" 2>"$http_work/server.err" &
else
    exec_request run examples/http.lkj --engine interpreter \
        --grant output --grant "root=$http_work/docs" \
        --grant listener=127.0.0.1:0 --grant clock --grant ready --grant stop \
        <&- >"$http_work/server.out" 2>"$http_work/server.err" &
fi
server_pid=$!
started_pids="$started_pids $server_pid"
attempt=0
marker=
while ! read -r marker port < "$http_work/server.out" || [ "$marker" != port ]; do
    if ! kill -0 "$server_pid" 2>/dev/null; then
        echo "HTTP server exited before publishing its port" >&2; exit 1
    fi
    attempt=$((attempt+1))
    if [ "$attempt" -ge 1200 ]; then
        echo "HTTP startup watchdog expired" >&2; exit 1
    fi
    sleep 0.05
done
case "$port" in ''|*[!0-9]*) echo "invalid server port" >&2; exit 1;; esac
if [ "$engine" = interpreter ]; then application_policy=60/120/120; else application_policy=60/2/2; fi
printf 'HTTP application deadlines: idle/request/write %s seconds\n' "$application_policy"
printf 'HTTP run: supervisor %s, server %s, port %s, engine %s, load %s s, pipeline %s, path %s, load verification %s, UTC %s; loadavg %s\n' \
    "$$" "$server_pid" "$port" "$engine" "$seconds" "$pipeline" "$load_path" "$verification" "$(date -u +%FT%TZ)" "$(cat /proc/loadavg)"
started=$(date +%s)
next_sample=0
sample() {
    warm=0
    if grep -q '^warmup complete' "$http_work/client.out"; then warm=1; fi
    now=$(date +%s)
    elapsed=$((now-started))
    awk -v time="$elapsed" -v warm="$warm" '
        /^VmRSS:/ {rss=$2} /^VmSize:/ {virtual=$2} /^VmHWM:/ {peak=$2}
        END {print time, warm, rss, virtual, peak}
    ' "/proc/$server_pid/status" >> "$http_work/memory"
}
await_client() {
    client_started=$(date +%s)
    started_pids="$started_pids $client_pid"
    while kill -0 "$client_pid" 2>/dev/null; do
        if ! kill -0 "$server_pid" 2>/dev/null; then
            echo "HTTP server exited during traffic" >&2; exit 1
        fi
        now=$(date +%s)
        if [ "$((now-client_started))" -ge "$client_watchdog" ]; then
            echo "HTTP client watchdog expired" >&2; exit 1
        fi
        elapsed=$((now-started))
        if [ "$elapsed" -ge "$next_sample" ]; then sample; next_sample=$((elapsed+30)); fi
        sleep 1
    done
    if wait "$client_pid"; then client_status=0; else client_status=$?; fi
    client_pid=
    [ "$client_status" -eq 0 ] || exit "$client_status"
}
"$http_work/client" "$port" "$http_work/docs" >"$http_work/client.out" 2>"$http_work/client.err" &
client_pid=$!
client_watchdog=600
await_client
cat "$http_work/client.out"
: > "$http_work/client.out"
# Exercise the reusable functional client on both routes against the same server.
compile_request build tools/http-get.lkj --grant output --grant input --grant "target=127.0.0.1:$port" -o "$http_work/get"
printf 'HEAD /language.md HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n' > "$http_work/request"
"$http_work/get" <"$http_work/request" >"$http_work/client.out" 2>"$http_work/client.err" &
client_pid=$!
await_client
grep -q '^HTTP/1.1 200 ' "$http_work/client.out"
exec_request run tools/http-get.lkj --engine interpreter \
    --grant output --grant input --grant "target=127.0.0.1:$port" \
    <"$http_work/request" >"$http_work/client.out" 2>"$http_work/client.err" &
client_pid=$!
await_client
grep -q '^HTTP/1.1 200 ' "$http_work/client.out"
printf 'ok http-get native and interpreter\n'
: > "$http_work/client.out"
if [ "$seconds" -gt 0 ]; then
    compile_request build tools/http-load.lkj --grant output --grant input --grant clock \
        --grant "root=$http_work/docs" --grant "target=127.0.0.1:$port" -o "$http_work/load"
    printf '%s\n%s\n%s\n%s\n' "$seconds" "$pipeline" "$load_path" "$verification" > "$http_work/config"
    "$http_work/load" <"$http_work/config" >"$http_work/client.out" 2>"$http_work/client.err" &
    client_pid=$!
    client_watchdog=$((seconds+600))
    await_client
    cat "$http_work/client.out"
else
    printf 'warmup complete\n' > "$http_work/client.out"
fi
sample
printf 'HTTP measurement loadavg: %s\n' "$(cat /proc/loadavg)"
kill "$server_pid"
wait "$server_pid" 2>/dev/null || :
sample_pid=$(printf '%020d' "$server_pid")
server_pid=
awk '
    $2==1 {
        if (!n) {first=$3; min=$3; max=$3; first_virtual=$4}
        if ($3<min) min=$3
        if ($3>max) max=$3
        last=$3; last_virtual=$4; peak=$5; n++
    }
    END {
        printf "HTTP memory after warmup: %d samples; RSS first/last/min/max %d/%d/%d/%d KiB; virtual first/last %d/%d KiB; peak RSS %d KiB\n", n, first, last, min, max, first_virtual, last_virtual, peak
    }
' "$http_work/memory"
mkdir -p "$profile_directory"
measurements="$profile_directory/http.memory.$sample_pid"
cp "$http_work/memory" "$measurements"
printf 'HTTP memory samples: %s\n' "$measurements"
if [ "$engine" = native ]; then
    samples="$profile_directory/http.samples.$sample_pid"
    cp "$http_work/server.samples.$sample_pid" "$samples"
    "$compiler" profile "$samples"
    printf 'HTTP profile samples: %s\n' "$samples"
    # Its own server/client pair starts only after the profiled server has exited.
    sh tools/http-deadline.sh --compiler "$compiler"
fi
