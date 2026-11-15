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
issuer=$probe/issuer
pid=
started=
ports=
cleanup() {
    status=$?
    verified=yes
    trap - EXIT HUP INT TERM
    if [ -n "$pid" ]; then kill -KILL "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; pid=; fi
    if [ "$status" -ne 0 ]; then
        for output in "$probe/startup" "$probe/stderr" "$probe/failure-out" "$probe/failure-err"; do
            if [ -f "$output" ]; then cat "$output"; fi
        done
    fi
    for dead in $started; do
        if kill -0 "$dead" 2>/dev/null; then printf 'journal case server remains: %s\n' "$dead" >&2; status=1; verified=no; fi
    done
    for released in $ports; do
        # Every fixture listener is IPv4. Inspect the kernel table directly;
        # the seed-only Debian gate deliberately has no iproute2 dependency.
        port_status=0
        awk -v suffix="$(printf ':%04X' "$released")" '
            $4 == "0A" && substr($2, length($2) - 4) == suffix { found=1 }
            END { exit !found }
        ' < /proc/net/tcp || port_status=$?
        case "$port_status" in
            0) printf 'journal case port remains: %s\n' "$released" >&2; status=1; verified=no ;;
            1) ;;
            *) printf 'cannot verify journal case port: %s\n' "$released" >&2; status=1; verified=no ;;
        esac
    done
    rm -rf "$probe"
    if [ "$verified" = yes ]; then
        printf 'Journal cases cleanup: server PIDs%s; all reaped; loopback ports%s released; client processes reaped and temporary directory removed\n' "$started" "$ports"
    else
        printf 'Journal cases cleanup incomplete: server PIDs%s; loopback ports%s; see errors above; temporary directory removed\n' "$started" "$ports" >&2
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir "$probe/data"
printf 'abandoned unpublished key' >"$probe/data/.session-key-next"
if [ "$engine" = interpreter ]; then printf '60\n120\n120\n' >"$probe/data/.http-policy"; fi
"$compiler" build tools/http-journal-cases.lkj --library-dir library -o "$client"
"$compiler" build examples/journal/token.lkj --library-dir library \
    --grant output --grant errors --grant input --grant root="$probe/data" \
    --grant entropy --grant clock -o "$issuer"
if [ "$engine" = native ]; then
    "$compiler" build examples/journal/main.lkj --library-dir library \
        --grant output --grant root="$probe/data" --grant listener=127.0.0.1:0 \
        --grant clock --grant ready --grant stop --grant entropy -o "$server"
fi
start() {
    # Clear the preceding launch before polling; child redirections run later.
    : >"$probe/startup"
    : >"$probe/stderr"
    if [ "$engine" = native ]; then
        "$server" >"$probe/startup" 2>"$probe/stderr" &
    else
        "$compiler" run examples/journal/main.lkj --engine interpreter --library-dir library \
            --grant output --grant root="$probe/data" --grant listener=127.0.0.1:0 \
            --grant clock --grant ready --grant stop --grant entropy >"$probe/startup" 2>"$probe/stderr" &
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
graceful_stop() {
    endpoint=$(awk '$1 == "stop-target" {print $2}' "$probe/startup")
    test -n "$endpoint"
    if [ "$engine" = native ]; then
        test "$endpoint" = "$pid"
    else
        test "$(awk '$1 == "PPid:" {print $2}' "/proc/$endpoint/status")" = "$pid"
    fi
    kill -TERM "$endpoint"
    attempt=0
    while kill -0 "$pid" 2>/dev/null; do
        attempt=$((attempt + 1))
        test "$attempt" -lt 12000
        sleep 0.01
    done
    wait "$pid"
    pid=
    grep -Fx 'stopped' "$probe/startup"
}
request() { timeout --kill-after=5s 120s "$client" "$port" "$1" "$probe/data" test-token; }
issue() {
    # Credentials stay in private files, never command arguments or test output.
    (umask 077; printf '3600\n' | "$issuer" >"$probe/data/test-token")
    (umask 077; printf '3600\n' | "$issuer" >"$probe/second-token")
    if cmp -s "$probe/data/test-token" "$probe/second-token"; then
        printf 'session issuer repeated a random nonce\n' >&2
        exit 1
    fi
    if printf '0\n' | "$issuer" >"$probe/rejected-token" 2>"$probe/issuer-error"; then
        printf 'session issuer accepted zero lifetime\n' >&2
        exit 1
    fi
    test ! -s "$probe/rejected-token"
    grep -Fq 'session lifetime must be a positive' "$probe/issuer-error"
    test "$(stat -c %a "$probe/data/session.key")" = 600
    test ! -e "$probe/data/.session-key-next"
}
fail_start() {
    failure_status=0
    if [ "$engine" = native ]; then
        timeout --kill-after=5s 120s "$server" >"$probe/failure-out" 2>"$probe/failure-err" || failure_status=$?
    else
        timeout --kill-after=5s 120s "$compiler" run examples/journal/main.lkj --engine interpreter --library-dir library \
            --grant output --grant root="$probe/data" --grant listener=127.0.0.1:0 \
            --grant clock --grant ready --grant stop --grant entropy >"$probe/failure-out" 2>"$probe/failure-err" || failure_status=$?
    fi
    test "$failure_status" -ne 0
    test "$failure_status" -ne 124
    test "$failure_status" -ne 137
    grep -F "$1" "$probe/failure-out" "$probe/failure-err"
}
if [ "$engine" = interpreter ]; then
    start
    issue
    request init
    stop
    start
    request verify
    graceful_stop
    printf 'ok journal interpreted CRUD, pipeline and recovery smoke\n'
    exit 0
fi
start
issue
request init
before=$(sha256sum "$probe/data/journal.log")
fail_start "lock journal: errno 11"
test "$before" = "$(sha256sum "$probe/data/journal.log")"
test "$(stat -c %s "$probe/data/head")" -eq 96
test "$(stat -c %a "$probe/data/session.key")" = 600
test ! -e "$probe/data/.head-next"
stop
cp "$probe/data/journal.log" "$probe/base-log"
cp "$probe/data/head" "$probe/base-head"
base=$(stat -c %s "$probe/base-log")
cp "$probe/data/session.key" "$probe/base-key"
chmod 644 "$probe/data/session.key"
printf 'abandoned replacement key' >"$probe/data/.session-key-next"
start
test "$(stat -c %a "$probe/data/session.key")" = 600
test ! -e "$probe/data/.session-key-next"
cmp "$probe/base-key" "$probe/data/session.key"
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
    grep -F "recovery repaired $base" "$probe/startup"
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
# Malformed signing keys fail without silent replacement or token invalidation.
for length in 31 33; do
    cp "$probe/base-key" "$probe/data/session.key"
    truncate -s "$length" "$probe/data/session.key"
    before=$(sha256sum "$probe/data/session.key")
    fail_start "session.key must contain exactly 32 bytes"
    test "$before" = "$(sha256sum "$probe/data/session.key")"
done
cp "$probe/base-key" "$probe/data/session.key"
start
request verify
graceful_stop
printf 'ok journal durable store (%s): creation, append/replace, pipelining, restart, incomplete header/body/checksum, stale head-next, complete unacknowledged record, durable-boundary refusal, log/head corruption, single-writer lock, signed/expired/forged sessions, private durable key and separate issuer\n' "$engine"
