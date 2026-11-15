#!/bin/sh
# Fixed deadlines and watchdogs here are explicit consumer/test policy.
set -eu
cd "$(dirname "$0")/.."
compiler=target/lkjscript
engine=native
while [ "$#" -gt 0 ]; do
    case "$1" in
        --compiler) [ "$#" -ge 2 ] || exit 2; compiler=$2; shift 2 ;;
        --engine) [ "$#" -ge 2 ] || exit 2; engine=$2; shift 2 ;;
        *) printf 'usage: sh tools/http-stop.sh [--compiler PATH] [--engine native|interpreter]\n' >&2; exit 2 ;;
    esac
done
case "$engine" in native|interpreter) ;; *) exit 2 ;; esac
stop_work=$(mktemp -d target/http-stop.XXXXXX)
server_pid=
client_pid=
started=
port=
cleanup() {
    status=$?
    trap - EXIT HUP INT TERM
    exec 9>&-
    if [ -n "$client_pid" ]; then kill -KILL "$client_pid" 2>/dev/null || true; wait "$client_pid" 2>/dev/null || true; fi
    if [ -n "$server_pid" ]; then kill -KILL "$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true; fi
    if [ "$status" -ne 0 ]; then
        for log in "$stop_work/server.out" "$stop_work/server.err" "$stop_work/client.out" "$stop_work/client.err"; do
            if [ -f "$log" ]; then cat "$log"; fi
        done
    fi
    for dead in $started; do if kill -0 "$dead" 2>/dev/null; then status=1; fi; done
    if [ -n "$port" ]; then
        port_hex=$(printf '%04X' "$port")
        # Linux state 0A is LISTEN; TIME_WAIT does not retain a listener.
        if [ -r /proc/net/tcp6 ]; then
            set -- /proc/net/tcp /proc/net/tcp6
        else
            set -- /proc/net/tcp
        fi
        if ! awk -v wanted="$port_hex" '
            $4 == "0A" { split($2, address, ":"); if (address[2] == wanted) occupied=1 }
            END { exit occupied ? 1 : 0 }
        ' "$@"; then status=1; fi
    fi
    rm -rf "$stop_work"
    printf 'Shutdown cleanup: PIDs%s reaped; loopback port %s released; temporary directory removed\n' "$started" "$port"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir "$stop_work/root"
printf 'ok\n' > "$stop_work/root/ok.txt"
if [ "$engine" = native ]; then
    printf '60\n60\n60\n2\n' > "$stop_work/root/.http-policy"
    dd if=/dev/zero of="$stop_work/root/large.bin" bs=1048576 count=32 status=none
    "$compiler" build examples/http.rs --library-dir library --grant output --grant root="$stop_work/root" \
        --grant listener=127.0.0.1:0 --grant clock --grant ready --grant stop -o "$stop_work/server"
else
    # A tiny fixture with the compatible three-line policy and default 5s drain.
    printf '60\n30\n30\n' > "$stop_work/root/.http-policy"
fi
"$compiler" build tools/http-stop.rs --library-dir library -o "$stop_work/client"
if [ "$engine" = native ]; then
    "$stop_work/server" >"$stop_work/server.out" 2>"$stop_work/server.err" &
else
    "$compiler" run examples/http.rs --engine interpreter --library-dir library \
        --grant output --grant root="$stop_work/root" --grant listener=127.0.0.1:0 \
        --grant clock --grant ready --grant stop >"$stop_work/server.out" 2>"$stop_work/server.err" &
fi
server_pid=$!
started="$started $server_pid"
attempt=0
while :; do
    port=$(awk '$1 == "port" {print $2}' "$stop_work/server.out")
    target=$(awk '$1 == "stop-target" {print $2}' "$stop_work/server.out")
    if [ -n "$port" ] && [ -n "$target" ]; then break; fi
    kill -0 "$server_pid" 2>/dev/null || exit 1
    attempt=$((attempt + 1)); [ "$attempt" -lt 6000 ] || exit 1
    sleep 0.01
done
# Confirm that the endpoint belongs to this execution before signaling it.
case "$target" in ''|*[!0-9]*) exit 1 ;; esac
if [ "$engine" = native ]; then
    test "$target" = "$server_pid"
else
    test "$target" != "$server_pid"
    test "$(awk '$1 == "PPid:" {print $2}' "/proc/$target/status")" = "$server_pid"
    started="$started $target"
fi
mkfifo "$stop_work/control"
exec 9<>"$stop_work/control"
if [ "$engine" = native ]; then
    "$stop_work/client" "$port" <"$stop_work/control" >"$stop_work/client.out" 2>"$stop_work/client.err" &
else
    "$stop_work/client" "$port" smoke <"$stop_work/control" >"$stop_work/client.out" 2>"$stop_work/client.err" &
fi
client_pid=$!
started="$started $client_pid"
attempt=0
while ! grep -q '^armed$' "$stop_work/client.out"; do
    kill -0 "$client_pid" 2>/dev/null || { wait "$client_pid"; exit 1; }
    kill -0 "$server_pid" 2>/dev/null || exit 1
    attempt=$((attempt + 1)); [ "$attempt" -lt 6000 ] || exit 1
    sleep 0.01
done
load_before=$(cat /proc/loadavg)
stop_start=$(date +%s%N)
kill -TERM "$target"
attempt=0
while ! grep -q '^stopping ' "$stop_work/server.out"; do
    kill -0 "$server_pid" 2>/dev/null || exit 1
    attempt=$((attempt + 1)); [ "$attempt" -lt 6000 ] || exit 1
    sleep 0.01
done
# A repeat does not reset the deadline or turn successful cleanup into signal exit.
kill -TERM "$target"
printf g >&9
attempt=0
while ! grep -q '^drained$' "$stop_work/client.out"; do
    kill -0 "$client_pid" 2>/dev/null || { wait "$client_pid"; exit 1; }
    attempt=$((attempt + 1)); [ "$attempt" -lt 6000 ] || exit 1
    sleep 0.01
done
attempt=0
while kill -0 "$server_pid" 2>/dev/null; do
    attempt=$((attempt + 1)); [ "$attempt" -lt 3000 ] || exit 1
    sleep 0.01
done
wait "$server_pid"
server_pid=
stop_end=$(date +%s%N)
elapsed_ms=$(((stop_end - stop_start) / 1000000))
if [ "$engine" = native ]; then
    # The unfinished request keeps draining until the configured two-second deadline.
    [ "$elapsed_ms" -ge 1500 ] && [ "$elapsed_ms" -lt 10000 ]
else
    [ "$elapsed_ms" -lt 10000 ]
fi
grep -q '^stopped$' "$stop_work/server.out"
if [ "$engine" = native ]; then printf g >&9; fi
wait "$client_pid"
client_pid=
cat "$stop_work/client.out"
printf 'Shutdown %s elapsed %s ms; load %s -> %s\n' "$engine" "$elapsed_ms" "$load_before" "$(cat /proc/loadavg)"
