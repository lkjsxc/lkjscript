#!/bin/sh
# Live-origin parity by default; the regular gate uses frozen offline observations.
set -eu
cd "$(dirname "$0")/.."
compiler=target/lkjscript
offline=no
systemd=no
usage() {
    printf 'usage: sh tools/http-site-test.sh [--compiler PATH] [--offline | --systemd]\n' >&2
    exit 2
}
while [ "$#" -gt 0 ]; do
    case "$1" in
        --compiler) [ "$#" -ge 2 ] || usage; compiler=$2; shift 2 ;;
        --offline) offline=yes; shift ;;
        --systemd) systemd=yes; shift ;;
        *) usage ;;
    esac
done
if [ "$systemd" = yes ]; then
    [ "$offline" = no ] || usage
    exec sh tools/http-site-systemd.sh --compiler "$compiler"
fi
site_work=$(mktemp -d target/programs-site-test.XXXXXX)
server_pid=
client_pid=
started=
port=
cleanup() {
    status=$?
    verified=yes
    trap - EXIT HUP INT TERM
    # timeout forwards TERM to its client and reaps it; --kill-after is the fallback.
    if [ -n "$client_pid" ]; then
        kill -TERM "$client_pid" 2>/dev/null || true
        wait "$client_pid" 2>/dev/null || true
    fi
    if [ -n "$server_pid" ]; then
        kill -KILL "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    if [ "$status" -ne 0 ]; then
        for log in "$site_work/server.out" "$site_work/server.err" "$site_work/client.out" "$site_work/client.err"; do
            if [ -f "$log" ]; then cat "$log"; fi
        done
    fi
    for dead in $started; do
        if kill -0 "$dead" 2>/dev/null; then
            printf 'site test process remains: %s\n' "$dead" >&2
            status=1
            verified=no
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
            printf 'site test listener remains or cannot be checked: %s\n' "$port" >&2
            status=1
            verified=no
        fi
    fi
    rm -rf "$site_work"
    if [ "$verified" = yes ]; then
        printf 'Site cleanup: server/client-supervisor PIDs%s reaped; client completed; loopback port %s released; private files removed\n' "$started" "$port"
    else
        printf 'Site cleanup incomplete: PIDs%s; loopback port %s; see errors above; private files removed\n' "$started" "$port" >&2
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

site_load_before=$(cat /proc/loadavg)
site_started_ns=$(date +%s%N)
if [ "$offline" = yes ]; then
    "$compiler" test tools/http-site-parity.rs --library-dir library --jobs 1
fi
"$compiler" build examples/site.rs --library-dir library \
    --grant output --grant listener=127.0.0.1:0 \
    --grant clock --grant ready --grant stop -o "$site_work/server"
"$site_work/server" >"$site_work/server.out" 2>"$site_work/server.err" &
server_pid=$!
started="$started $server_pid"
attempt=0
while :; do
    bound_port=$(awk '$1 == "port" {print $2}' "$site_work/server.out")
    stop_target=$(awk '$1 == "stop-target" {print $2}' "$site_work/server.out")
    if [ -n "$bound_port" ] && [ -n "$stop_target" ]; then break; fi
    kill -0 "$server_pid" 2>/dev/null || exit 1
    attempt=$((attempt + 1))
    [ "$attempt" -lt 6000 ] || { printf 'site startup watchdog expired\n' >&2; exit 1; }
    sleep 0.01
done
case "$bound_port" in ''|*[!0-9]*) exit 1 ;; esac
[ "$bound_port" -gt 0 ] && [ "$bound_port" -le 65535 ]
port=$bound_port
# Only this native process may be signaled through its published endpoint.
test "$stop_target" = "$server_pid"

if [ "$offline" = yes ]; then
    "$compiler" build tools/http-site-cases.rs --library-dir library \
        --grant output --grant root=. --grant target=127.0.0.1:"$port" \
        -o "$site_work/client"
    mode=offline
else
    "$compiler" build tools/http-site-parity.rs --library-dir library \
        --grant output --grant root=examples/site-content --grant origin=127.0.0.1:8798 \
        --grant candidate=127.0.0.1:"$port" -o "$site_work/client"
    mode=live
fi
timeout --kill-after=5s 60s "$site_work/client" >"$site_work/client.out" 2>"$site_work/client.err" &
client_pid=$!
started="$started $client_pid"
wait "$client_pid"
client_pid=
cat "$site_work/client.out"

kill -TERM "$stop_target"
attempt=0
while kill -0 "$server_pid" 2>/dev/null; do
    attempt=$((attempt + 1))
    [ "$attempt" -lt 3000 ] || { printf 'site shutdown watchdog expired\n' >&2; exit 1; }
    sleep 0.01
done
wait "$server_pid"
server_pid=
grep -q '^stopped$' "$site_work/server.out"
site_elapsed_ms=$((($(date +%s%N) - site_started_ns) / 1000000))
printf 'Site %s check %s ms; load %s -> %s\n' "$mode" "$site_elapsed_ms" "$site_load_before" "$(cat /proc/loadavg)"
