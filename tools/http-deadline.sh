#!/bin/sh
# One private native server/client pair; durations are regression-test policy.
set -eu
cd "$(dirname "$0")/.."
compiler=target/lkjscript
usage() {
    printf 'usage: sh tools/http-deadline.sh [--compiler PATH]\n' >&2
    exit 2
}
while [ "$#" -gt 0 ]; do
    case "$1" in
        --compiler) [ "$#" -ge 2 ] || usage; compiler=$2; shift 2 ;;
        *) usage ;;
    esac
done
deadline_work=$(mktemp -d target/programs-http-deadline.XXXXXX)
server_pid=
client_pid=
started=
port=
process_done() {
    if ! kill -0 "$1" 2>/dev/null; then return 0; fi
    # Reap exited children even if /proc still records a zombie.
    awk '$1 == "State:" && $2 == "Z" {done=1} END {exit !done}' "/proc/$1/status" 2>/dev/null
}
await_exit() {
    exit_attempt=0
    while ! process_done "$1"; do
        exit_attempt=$((exit_attempt + 1))
        [ "$exit_attempt" -lt "$2" ] || return 1
        sleep 0.01
    done
}
stop_server() {
    forced=no
    kill -TERM "$server_pid" 2>/dev/null || true
    if ! await_exit "$server_pid" 1000; then
        printf 'deadline server shutdown watchdog expired; killing owned PID %s\n' "$server_pid" >&2
        forced=yes
        kill -KILL "$server_pid" 2>/dev/null || true
        await_exit "$server_pid" 200 || return 124
    fi
    server_status=0
    wait "$server_pid" || server_status=$?
    server_pid=
    [ "$forced" = no ] || return 124
    return "$server_status"
}
cleanup_failed() {
    printf '%s\n' "$1" >&2
    [ "$status" -ne 0 ] || status=1
    verified=no
}
cleanup() {
    status=$?
    verified=yes
    trap - EXIT HUP INT TERM
    set +e
    if [ -n "$client_pid" ]; then
        # timeout forwards TERM and reaps its native client; its KILL fallback is 5s.
        kill -TERM "$client_pid" 2>/dev/null || true
        if ! await_exit "$client_pid" 700; then
            kill -KILL "$client_pid" 2>/dev/null || true
        fi
        if await_exit "$client_pid" 200; then
            wait "$client_pid" 2>/dev/null || true
            client_pid=
        else
            cleanup_failed "deadline client supervisor did not exit within its cleanup bound"
        fi
    fi
    if [ -n "$server_pid" ]; then
        stop_server || cleanup_failed "deadline server did not stop successfully within its cleanup bound"
    fi
    if [ "$status" -ne 0 ]; then
        for log in "$deadline_work/server.out" "$deadline_work/server.err" "$deadline_work/client.out" "$deadline_work/client.err"; do
            if [ -f "$log" ]; then cat "$log"; fi
        done
    fi
    for dead in $started; do
        if kill -0 "$dead" 2>/dev/null; then
            cleanup_failed "deadline process remains: $dead"
        fi
    done
    if [ -n "$port" ]; then
        port_hex=$(printf '%04X' "$port")
        if [ -r /proc/net/tcp6 ]; then
            set -- /proc/net/tcp /proc/net/tcp6
        else
            set -- /proc/net/tcp
        fi
        if ! awk -v wanted="$port_hex" '
            $4 == "0A" { split($2, address, ":"); if (address[2] == wanted) occupied=1 }
            END { exit occupied ? 1 : 0 }
        ' "$@"; then
            cleanup_failed "deadline listener remains or cannot be checked: $port"
        fi
    fi
    if ! rm -rf -- "$deadline_work" || [ -e "$deadline_work" ]; then
        cleanup_failed "deadline private directory was not removed: $deadline_work"
    fi
    if [ "$verified" = yes ]; then
        printf 'HTTP deadline cleanup complete: server/client-supervisor PIDs%s reaped; port %s released; private files removed\n' "$started" "$port"
    else
        printf 'HTTP deadline cleanup incomplete: PIDs%s; port %s; directory %s; see errors above\n' "$started" "$port" "$deadline_work" >&2
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

deadline_load_before=$(cut -d ' ' -f 1-3 /proc/loadavg)
deadline_started_ns=$(date +%s%N)
mkdir "$deadline_work/root"
printf 'ok\n' > "$deadline_work/root/ok.txt"
dd if=/dev/zero of="$deadline_work/root/large.bin" bs=1048576 count=32 status=none
printf '60\n1\n10\n' > "$deadline_work/root/.http-policy"
"$compiler" build examples/http.rs --library-dir library \
    --grant output --grant "root=$deadline_work/root" \
    --grant listener=127.0.0.1:0 --grant clock --grant ready --grant stop \
    -o "$deadline_work/server"
"$deadline_work/server" >"$deadline_work/server.out" 2>"$deadline_work/server.err" &
server_pid=$!
started="$started $server_pid"
attempt=0
while :; do
    bound_port=$(awk '$1 == "port" {print $2}' "$deadline_work/server.out")
    stop_target=$(awk '$1 == "stop-target" {print $2}' "$deadline_work/server.out")
    if [ -n "$bound_port" ] && [ -n "$stop_target" ]; then break; fi
    if process_done "$server_pid"; then
        printf 'deadline server exited before publishing its port and stop target\n' >&2
        exit 1
    fi
    attempt=$((attempt + 1))
    [ "$attempt" -lt 6000 ] || { printf 'deadline startup watchdog expired\n' >&2; exit 1; }
    sleep 0.01
done
case "$bound_port" in ''|*[!0-9]*) printf 'invalid deadline server port\n' >&2; exit 1 ;; esac
[ "$bound_port" -gt 0 ] && [ "$bound_port" -le 65535 ]
port=$bound_port
test "$stop_target" = "$server_pid"
"$compiler" build tools/http-deadline.rs --library-dir library \
    --grant output --grant "target=127.0.0.1:$port" --grant clock --grant ready \
    -o "$deadline_work/client"
timeout --kill-after=5s 60s "$deadline_work/client" >"$deadline_work/client.out" 2>"$deadline_work/client.err" &
client_pid=$!
started="$started $client_pid"
client_status=0
wait "$client_pid" || client_status=$?
client_pid=
[ "$client_status" -eq 0 ] || exit "$client_status"
cat "$deadline_work/client.out"
stop_server
grep -q '^stopped$' "$deadline_work/server.out"
deadline_elapsed_ms=$((($(date +%s%N) - deadline_started_ns) / 1000000))
printf 'HTTP deadline check %s ms; load %s -> %s\n' "$deadline_elapsed_ms" "$deadline_load_before" "$(cut -d ' ' -f 1-3 /proc/loadavg)"
