#!/bin/sh
# Focused HTTP journal framing, durability, corruption and single-writer cases.
set -eu
compiler=target/lkjscript
engine=native
while [ "$#" -gt 0 ]; do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        --engine) engine=$2; shift 2 ;;
        *) printf 'usage: sh tools/http-journal-cases.sh [--compiler PATH] [--engine native|interpreter]\n' >&2; exit 2 ;;
    esac
done
case "$engine" in native|interpreter) ;; *) printf 'invalid engine\n' >&2; exit 2 ;; esac
probe=$(mktemp -d target/http-journal-cases.XXXXXX)
server=$probe/journal
client=$probe/client
pid=
started=
ports=
cleanup() {
    status=$?
    trap - EXIT HUP INT TERM
    if [ -n "$pid" ]; then kill -KILL "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; pid=; fi
    if [ "$status" -ne 0 ]; then
        for output in "$probe/startup" "$probe/stderr" "$probe/failure-out" "$probe/failure-err"; do
            if [ -f "$output" ]; then cat "$output"; fi
        done
    fi
    for dead in $started; do
        if kill -0 "$dead" 2>/dev/null; then printf 'journal case server remains: %s\n' "$dead" >&2; status=1; fi
    done
    for released in $ports; do
        if ss -ltn | awk '{print $4}' | rg -q ":$released$"; then
            printf 'journal case port remains: %s\n' "$released" >&2
            status=1
        fi
    done
    rm -rf "$probe"
    printf 'Journal cases cleanup: server PIDs%s; all reaped; loopback ports%s released; client processes reaped and temporary directory removed\n' "$started" "$ports"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir "$probe/data"
if [ "$engine" = interpreter ]; then printf '60\n120\n120\n' >"$probe/data/.http-policy"; fi
"$compiler" build tools/http-journal-cases.rs --library-dir library -o "$client"
if [ "$engine" = native ]; then
    "$compiler" build examples/journal.rs --library-dir library \
        --grant output --grant root="$probe/data" --grant listener=127.0.0.1:0 \
        --grant clock --grant ready -o "$server"
fi
start() {
    if [ "$engine" = native ]; then
        "$server" >"$probe/startup" 2>"$probe/stderr" &
    else
        "$compiler" run examples/journal.rs --engine interpreter --library-dir library \
            --grant output --grant root="$probe/data" --grant listener=127.0.0.1:0 \
            --grant clock --grant ready >"$probe/startup" 2>"$probe/stderr" &
    fi
    pid=$!
    started="$started $pid"
    attempt=0
    port=
    while [ "$attempt" -lt 6000 ]; do
        port=$(awk '$1 == "port" {print $2}' "$probe/startup")
        if [ -n "$port" ]; then break; fi
        if ! kill -0 "$pid" 2>/dev/null; then wait "$pid" 2>/dev/null || true; pid=; exit 1; fi
        attempt=$((attempt + 1))
        sleep 0.01
    done
    test -n "$port"
    ports="$ports $port"
}
stop() {
    kill -KILL "$pid"
    wait "$pid" 2>/dev/null || true
    pid=
}
request() { timeout --kill-after=5s 120s "$client" "$port" "$1"; }
fail_start() {
    failure_status=0
    if [ "$engine" = native ]; then
        timeout --kill-after=5s 120s "$server" >"$probe/failure-out" 2>"$probe/failure-err" || failure_status=$?
    else
        timeout --kill-after=5s 120s "$compiler" run examples/journal.rs --engine interpreter --library-dir library \
            --grant output --grant root="$probe/data" --grant listener=127.0.0.1:0 \
            --grant clock --grant ready >"$probe/failure-out" 2>"$probe/failure-err" || failure_status=$?
    fi
    test "$failure_status" -ne 0
    test "$failure_status" -ne 124
    test "$failure_status" -ne 137
    rg -F "$1" "$probe/failure-out" "$probe/failure-err"
}
if [ "$engine" = interpreter ]; then
    start
    request init
    stop
    start
    request verify
    stop
    printf 'ok journal interpreted CRUD, pipeline and recovery smoke\n'
    exit 0
fi
start
request init
before=$(sha256sum "$probe/data/journal.log")
fail_start "lock journal: errno 11"
test "$before" = "$(sha256sum "$probe/data/journal.log")"
test "$(stat -c %s "$probe/data/head")" -eq 96
test ! -e "$probe/data/.head-next"
stop
cp "$probe/data/journal.log" "$probe/base-log"
cp "$probe/data/head" "$probe/base-head"
base=$(stat -c %s "$probe/base-log")
start
request verify
request add
stop
cp "$probe/data/journal.log" "$probe/full-log"
full=$(stat -c %s "$probe/full-log")
for length in $((base + 7)) $((base + 46)) $((full - 1)); do
    cp "$probe/full-log" "$probe/data/journal.log"
    cp "$probe/base-head" "$probe/data/head"
    truncate -s "$length" "$probe/data/journal.log"
    printf 'abandoned partial manifest' >"$probe/data/.head-next"
    start
    request verify
    test "$(stat -c %s "$probe/data/journal.log")" -eq "$base"
    test ! -e "$probe/data/.head-next"
    rg -F "recovery repaired $base" "$probe/startup"
    stop
done
cp "$probe/full-log" "$probe/data/journal.log"
cp "$probe/base-head" "$probe/data/head"
start
request extra
test "$(stat -c %s "$probe/data/journal.log")" -eq "$full"
stop
cp "$probe/base-log" "$probe/data/journal.log"
cp "$probe/base-head" "$probe/data/head"
truncate -s $((base - 1)) "$probe/data/journal.log"
before=$(sha256sum "$probe/data/journal.log")
fail_start "head points beyond the complete log prefix; no repair performed"
test "$before" = "$(sha256sum "$probe/data/journal.log")"
cp "$probe/base-log" "$probe/data/journal.log"
printf z | dd of="$probe/data/journal.log" bs=1 seek=40 conv=notrunc status=none
before=$(sha256sum "$probe/data/journal.log")
fail_start "record checksum mismatch"
test "$before" = "$(sha256sum "$probe/data/journal.log")"
cp "$probe/base-log" "$probe/data/journal.log"
printf z | dd of="$probe/data/head" bs=1 seek=16 conv=notrunc status=none
before=$(sha256sum "$probe/data/journal.log")
fail_start "head checksum mismatch"
test "$before" = "$(sha256sum "$probe/data/journal.log")"
cp "$probe/base-head" "$probe/data/head"
start
request verify
stop
printf 'ok journal durable store (%s): creation, append/replace, pipelining, restart, incomplete header/body/checksum, stale head-next, complete unacknowledged record, durable-boundary refusal, log/head corruption and single-writer lock\n' "$engine"
