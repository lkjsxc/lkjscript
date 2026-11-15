#!/bin/sh
# One native lkjscript client interleaves ten active peers and the adversaries.
# Sampling and watchdogs are test policy; no benchmark threshold gates the language.
set -eu
cd "$(dirname "$0")/.."
compiler=target/lkjscript
seconds=10
idle=990
while [ "$#" -gt 0 ]; do
    case "$1" in
        --compiler|--seconds|--idle)
            [ "$#" -ge 2 ] || { echo "missing value for $1" >&2; exit 2; }
            case "$1" in
                --compiler) compiler=$2 ;;
                --seconds) seconds=$2 ;;
                --idle) idle=$2 ;;
            esac
            shift 2 ;;
        *) echo "usage: sh tools/http-concurrency.sh [--compiler PATH] [--seconds 1..44] [--idle N]" >&2; exit 2 ;;
    esac
done
case "$seconds" in ''|*[!0-9]*) echo "invalid seconds" >&2; exit 2;; esac
case "$idle" in ''|*[!0-9]*) echo "invalid silent peer count" >&2; exit 2;; esac
[ "$seconds" -gt 0 ] && [ "$seconds" -lt 45 ] || { echo "seconds must be 1..44" >&2; exit 2; }
http_work=$(mktemp -d target/http-concurrency.XXXXXX)
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
    for log in "$http_work/server.err" "$http_work/client.err"; do
        if [ -s "$log" ]; then cat "$log" >&2; fi
    done
    rm -rf "$http_work"
    gone=yes
    for pid in $started_pids; do
        if kill -0 "$pid" 2>/dev/null; then gone=no; fi
    done
    listening=no
    if [ -n "$port" ] && ss -ltnH "sport = :$port" | grep -q .; then listening=yes; fi
    printf 'Concurrency cleanup: started PIDs%s; all gone %s; port %s listening %s; temporary directory removed\n' \
        "$started_pids" "$gone" "$port" "$listening"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
mkdir "$http_work/root"
dd if=/dev/zero of="$http_work/root/load.bin" bs=40108 count=1 status=none
dd if=/dev/zero of="$http_work/root/blocked.bin" bs=1048576 count=32 status=none
"$compiler" build examples/http.rs \
    --grant output --grant "root=$http_work/root" \
    --grant listener=127.0.0.1:0 --grant clock --grant ready -o "$http_work/server"
: > "$http_work/server.out"
: > "$http_work/server.err"
: > "$http_work/client.out"
: > "$http_work/client.err"
: > "$http_work/memory"
"$http_work/server" <&- >"$http_work/server.out" 2>"$http_work/server.err" &
server_pid=$!
started_pids="$started_pids $server_pid"
attempt=0
marker=
while ! read -r marker port < "$http_work/server.out" || [ "$marker" != port ]; do
    if ! kill -0 "$server_pid" 2>/dev/null; then
        echo "concurrency server exited before publishing a port" >&2; exit 1
    fi
    attempt=$((attempt+1))
    [ "$attempt" -lt 1200 ] || { echo "concurrency startup watchdog expired" >&2; exit 1; }
    sleep 0.05
done
case "$port" in ''|*[!0-9]*) echo "invalid server port" >&2; exit 1;; esac
"$compiler" build tools/http-concurrency.rs \
    --grant output --grant input --grant "target=127.0.0.1:$port" \
    --grant clock --grant ready --grant "inspect=/proc/$server_pid/fd" -o "$http_work/client"
printf '%s\n%s\n' "$seconds" "$idle" > "$http_work/config"
printf 'Concurrency run: server %s, port %s, %s seconds per measurement, %s silent peers; UTC %s; loadavg %s\n' \
    "$server_pid" "$port" "$seconds" "$idle" "$(date -u +%FT%TZ)" "$(cat /proc/loadavg)"
"$http_work/client" <"$http_work/config" >"$http_work/client.out" 2>"$http_work/client.err" &
client_pid=$!
started_pids="$started_pids $client_pid"
started=$(date +%s)
last_phase=
sample() {
    phase=$(awk '/^phase / {phase=$2} END {print phase}' "$http_work/client.out")
    [ -n "$phase" ] || phase=startup
    if [ "$phase" != "$last_phase" ]; then
        printf 'Concurrency phase %s: loadavg %s\n' "$phase" "$(cat /proc/loadavg)"
        last_phase=$phase
    fi
    descriptors=$(ls -U "/proc/$server_pid/fd" | wc -l)
    awk -v phase="$phase" -v descriptors="$descriptors" '
        /^VmRSS:/ {rss=$2} /^VmSize:/ {virtual=$2} /^VmHWM:/ {peak=$2}
        END {print phase, rss, virtual, peak, descriptors}
    ' "/proc/$server_pid/status" >> "$http_work/memory"
}
while kill -0 "$client_pid" 2>/dev/null; do
    kill -0 "$server_pid" 2>/dev/null || { echo "server exited under concurrent traffic" >&2; exit 1; }
    now=$(date +%s)
    [ "$((now-started))" -lt "$((seconds*3+90))" ] || {
        echo "concurrency client watchdog expired" >&2; exit 1
    }
    sample
    sleep 0.2
done
if wait "$client_pid"; then client_status=0; else client_status=$?; fi
client_pid=
cat "$http_work/client.out"
awk '
    /^ok concurrent baseline-before:/ {before=$8}
    /^ok concurrent idle-loaded:/ {loaded=$8}
    /^ok concurrent baseline-after:/ {after=$8}
    END {
        if (before && after) printf "Silent-peer throughput versus bracketing unloaded mean: %.2f%% (%d / %.1f requests/s)\n", loaded*200/(before+after),loaded,(before+after)/2
    }
' "$http_work/client.out"
[ "$client_status" -eq 0 ] || exit "$client_status"
# Allow the server's next readiness dispatch to observe all final peer closes.
sleep 0.2
sample
awk -v idle="$idle" '
    $1=="baseline-before" || $1=="idle-loaded" || $1=="baseline-after" || $1=="hostile" {
        name=$1
        if (!n[name]) {min[name]=$2; max[name]=$2; fdmin[name]=$5; fdmax[name]=$5}
        if ($2<min[name]) min[name]=$2
        if ($2>max[name]) max[name]=$2
        if ($5<fdmin[name]) fdmin[name]=$5
        if ($5>fdmax[name]) fdmax[name]=$5
        sum[name]+=$2; n[name]++
    }
    $1=="closed" {closed=$5; saw_closed=1}
    END {
        names[1]="baseline-before"; names[2]="idle-loaded"; names[3]="baseline-after"; names[4]="hostile"
        for (i=1; i<=4; i++) {
            name=names[i]
            printf "Concurrency memory %s: %d samples, RSS min/max %d/%d KiB, mean %.1f KiB, owned descriptors min/max %d/%d\n",
                name,n[name],min[name],max[name],n[name]?sum[name]/n[name]:0,fdmin[name],fdmax[name]
        }
        if (idle && n["baseline-before"] && n["idle-loaded"]) {
            delta=sum["idle-loaded"]/n["idle-loaded"]-sum["baseline-before"]/n["baseline-before"]
            printf "Incremental process RSS per silent peer: %.1f bytes (%d peers); includes slot/timer/wait-set growth, excludes kernel socket memory\n", delta*1024/idle,idle
        }
        printf "Owned descriptors after all clients close: %d\n",closed
        if (fdmin["idle-loaded"] < fdmin["baseline-before"] + idle) {
            print "server did not own every silent connection during measurement" > "/dev/stderr"
            exit 1
        }
        if (!saw_closed || closed>fdmin["baseline-before"]-10) {
            print "server retained a client descriptor after cleanup" > "/dev/stderr"
            exit 1
        }
    }
' "$http_work/memory"
printf 'Concurrency final loadavg: %s\n' "$(cat /proc/loadavg)"
kill "$server_pid"
wait "$server_pid" 2>/dev/null || :
server_pid=
