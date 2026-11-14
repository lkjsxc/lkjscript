#!/bin/sh
# The duration and request watchdogs are test policy; the server has no lifetime cap.
set -eu
cd "$(dirname "$0")/.."
compiler=target/lkjscript
seconds=3600
engine=native
profile_directory=target/http-profiles
curl_path=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --compiler|--seconds|--curl|--out|--engine)
            [ "$#" -ge 2 ] || { echo "missing value for $1" >&2; exit 2; }
            case "$1" in
                --compiler) compiler=$2 ;;
                --seconds) seconds=$2 ;;
                --curl) curl_path=$2 ;;
                --out) profile_directory=$2 ;;
                --engine) engine=$2 ;;
            esac
            shift 2 ;;
        *) echo "usage: sh tools/http-test.sh [--compiler PATH] [--seconds N] [--curl PATH] [--out DIRECTORY] [--engine native|interpreter]" >&2; exit 2 ;;
    esac
done
case "$seconds" in ''|*[!0-9]*) echo "seconds must be a nonnegative integer" >&2; exit 2;; esac
case "$engine" in native|interpreter) ;; *) echo "engine must be native or interpreter" >&2; exit 2;; esac
if [ -z "$curl_path" ]; then curl_path=$(command -v curl); fi
curl_path=$(readlink -f "$curl_path")
http_work=$(mktemp -d target/http-test.XXXXXX)
server_pid=
client_pid=
cleanup() {
    trap - EXIT HUP INT TERM
    if [ -n "$server_pid" ]; then
        kill "$server_pid" 2>/dev/null || :
        wait "$server_pid" 2>/dev/null || :
    fi
    # Closing the server makes the active request fail. Let the client wait for
    # its curl child and return, so an interrupted test cannot orphan curl.
    if [ -n "$client_pid" ]; then
        wait "$client_pid" 2>/dev/null || :
    fi
    if [ -s "$http_work/server.err" ]; then cat "$http_work/server.err" >&2; fi
    rm -rf "$http_work"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM

mkdir "$http_work/docs"
cp docs/*.md "$http_work/docs/"
printf 'outside the granted root\n' > "$http_work/outside.txt"
ln -s language.md "$http_work/docs/link-inside"
ln -s ../outside.txt "$http_work/docs/link-outside"

"$compiler" test examples/http_protocol.rs --jobs 1
if [ "$engine" = native ]; then
    "$compiler" build examples/http.rs --profile \
        --grant output --grant "root=$http_work/docs" \
        --grant listener=127.0.0.1:0 --grant clock -o "$http_work/server"
fi
"$compiler" build tools/http-test.rs -o "$http_work/client"
# Prepare logs before forking so the supervisor can read them immediately.
: > "$http_work/server.out"
: > "$http_work/server.err"
# A closed stdin forces accept4 to exercise promotion of descriptor zero.
if [ "$engine" = native ]; then
    "$http_work/server" <&- >"$http_work/server.out" 2>"$http_work/server.err" &
else
    "$compiler" run examples/http.rs --engine interpreter \
        --grant output --grant "root=$http_work/docs" \
        --grant listener=127.0.0.1:0 --grant clock \
        <&- >"$http_work/server.out" 2>"$http_work/server.err" &
fi
server_pid=$!
attempt=0
marker=
port=
while ! read -r marker port < "$http_work/server.out" || [ "$marker" != port ]; do
    if ! kill -0 "$server_pid" 2>/dev/null; then
        echo "HTTP server exited before publishing its port" >&2
        exit 1
    fi
    attempt=$((attempt+1))
    if [ "$attempt" -ge 1200 ]; then
        echo "HTTP server did not publish its port within the startup watchdog" >&2
        exit 1
    fi
    sleep 0.05
done
case "$port" in ''|*[!0-9]*) echo "invalid server port" >&2; exit 1;; esac
: > "$http_work/client.out"
: > "$http_work/client.err"
"$http_work/client" "$curl_path" "http://127.0.0.1:$port/" "$http_work/docs" "$seconds" \
    >"$http_work/client.out" 2>"$http_work/client.err" &
client_pid=$!
printf 'HTTP run: supervisor %s, server %s, client %s, port %s, engine %s, load %s s, work %s, UTC %s\n' \
    "$$" "$server_pid" "$client_pid" "$port" "$engine" "$seconds" "$http_work" "$(date -u +%FT%TZ)"
started=$(date +%s)
next_sample=0
sample() {
    warm=0
    if grep -q '^warmup complete$' "$http_work/client.out"; then warm=1; fi
    now=$(date +%s)
    elapsed=$((now-started))
    awk -v time="$elapsed" -v warm="$warm" '
        /^VmRSS:/ {rss=$2} /^VmSize:/ {virtual=$2} /^VmHWM:/ {peak=$2}
        END {print time, warm, rss, virtual, peak}
    ' "/proc/$server_pid/status" >> "$http_work/memory"
}
while kill -0 "$client_pid" 2>/dev/null; do
    if ! kill -0 "$server_pid" 2>/dev/null; then
        echo "HTTP server exited during generated traffic" >&2
        exit 1
    fi
    now=$(date +%s)
    elapsed=$((now-started))
    if [ "$elapsed" -ge "$next_sample" ]; then sample; next_sample=$((elapsed+30)); fi
    sleep 1
done
if wait "$client_pid"; then client_status=0; else client_status=$?; fi
client_pid=
sample
cat "$http_work/client.out"
cat "$http_work/client.err" >&2
[ "$client_status" -eq 0 ] || exit "$client_status"
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
fi
