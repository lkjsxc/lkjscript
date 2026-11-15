#!/bin/sh
# The native lkjscript supervisor owns and reaps every journal process.
# One final campaign uses 1,000 random write kills and 1,000 recovery kills.
set -eu
cd "$(dirname "$0")/.."
compiler=target/lkjscript
cycles=1000
seed=6840227782638526189
while [ "$#" -gt 0 ]; do
    case "$1" in
        --compiler|--cycles|--seed)
            [ "$#" -ge 2 ] || { echo "missing value for $1" >&2; exit 2; }
            case "$1" in
                --compiler) compiler=$2 ;;
                --cycles) cycles=$2 ;;
                --seed) seed=$2 ;;
            esac
            shift 2 ;;
        *) echo "usage: sh tools/http-journal-test.sh [--compiler PATH] [--cycles N] [--seed N]" >&2; exit 2 ;;
    esac
done
case "$cycles" in ''|*[!0-9]*|0) echo "cycles must be positive" >&2; exit 2;; esac
case "$seed" in ''|*[!0-9]*|0) echo "seed must be positive" >&2; exit 2;; esac
journal_work=$(mktemp -d target/http-journal.XXXXXX)
journal_absolute="$(pwd)/$journal_work"
supervisor=
started=
cleanup() {
    trap - EXIT HUP INT TERM
    if [ -n "$supervisor" ]; then
        kill "$supervisor" 2>/dev/null || :
        wait "$supervisor" 2>/dev/null || :
    fi
    if [ -s "$journal_work/errors" ]; then cat "$journal_work/errors" >&2; fi
    retained=
    for entry in /proc/[0-9]*/exe; do
        executable=$(readlink "$entry" 2>/dev/null || :)
        if [ "$executable" = "$journal_absolute/server" ] || [ "$executable" = "$journal_absolute/supervisor" ]; then
            retained="$retained $entry"
        fi
    done
    if [ -n "$retained" ]; then
        echo "journal cleanup found a retained test executable:$retained" >&2
        exit 1
    fi
    rm -rf "$journal_work"
    printf 'Journal cleanup: supervisor PID %s gone; no journal test executable retained; all child stops/reaps counted above; temporary directory removed\n' "$started"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
mkdir "$journal_work/data"
"$compiler" build examples/journal.rs --library-dir library \
    --grant output --grant "root=$journal_absolute/data" \
    --grant listener=127.0.0.1:0 --grant clock --grant ready -o "$journal_work/server"
"$compiler" build tools/http-journal-test.rs --library-dir library -o "$journal_work/supervisor"
printf 'Journal campaign: %s write kills + %s recovery kills; seed %s; UTC %s; loadavg %s\n' \
    "$cycles" "$cycles" "$seed" "$(date -u +%FT%TZ)" "$(cat /proc/loadavg)"
"$journal_work/supervisor" "$journal_absolute" "$cycles" "$seed" >"$journal_work/output" 2>"$journal_work/errors" &
supervisor=$!
started=$supervisor
begin=$(date +%s)
last=
while kill -0 "$supervisor" 2>/dev/null; do
    progress=$(awk '/^journal progress / {line=$0} END {print line}' "$journal_work/output")
    if [ "$progress" != "$last" ]; then
        printf '%s; loadavg %s\n' "$progress" "$(cat /proc/loadavg)"
        last=$progress
    fi
    now=$(date +%s)
    [ "$((now-begin))" -lt "$((cycles*30+120))" ] || {
        echo "journal supervisor watchdog expired" >&2; exit 1
    }
    sleep 1
done
if wait "$supervisor"; then status=0; else status=$?; fi
supervisor=
cat "$journal_work/output"
printf 'Journal final loadavg: %s\n' "$(cat /proc/loadavg)"
exit "$status"
