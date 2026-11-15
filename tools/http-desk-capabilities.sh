#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        *) printf 'usage: %s [--compiler PATH]\n' "$0" >&2; exit 2 ;;
    esac
done
[[ "$compiler" = /* ]] || compiler="$PWD/$compiler"
case_dir=$(mktemp -d "$PWD/target/desk-capabilities.XXXXXX")
child=
cleanup() {
    if [[ -n "$child" ]]; then kill "$child" 2>/dev/null || true; wait "$child" 2>/dev/null || true; fi
    rm -rf -- "$case_dir"
}
trap cleanup EXIT
fail() { printf 'desk capabilities: %s\n' "$*" >&2; exit 1; }
printf 'desk capabilities load_before '; cat /proc/loadavg
start=$(date +%s%N)
grants=(--grant args --grant value=DESK_CAPS_TEST --grant input --grant output --grant status)
"$compiler" build tests/fixtures/authority/command.rs --library-dir library --no-store \
    "${grants[@]}" -o "$case_dir/command"
printf 'stdin:入力\n' > "$case_dir/input"
for mode in absent empty utf8 binary; do
    printf 'arg:\narg:日本語 argument\n' > "$case_dir/expected"
    case "$mode" in
        absent) unset DESK_CAPS_TEST; printf 'absent\n' >> "$case_dir/expected" ;;
        empty) export DESK_CAPS_TEST=; printf 'value:\n' >> "$case_dir/expected" ;;
        utf8) export DESK_CAPS_TEST='環境 byte test'; printf 'value:環境 byte test\n' >> "$case_dir/expected" ;;
        binary) DESK_CAPS_TEST=$(printf '\377\376exact'); export DESK_CAPS_TEST
            printf 'value:\377\376exact\n' >> "$case_dir/expected" ;;
    esac
    export DESK_CAPS_TEST_MORE=must_not_match
    cat "$case_dir/input" >> "$case_dir/expected"
    printf 'cleanup\n' >> "$case_dir/expected"
    for engine in standalone native interpreter; do
        set +e
        if [[ "$engine" = standalone ]]; then
            "$case_dir/command" '' '日本語 argument' < "$case_dir/input" > "$case_dir/output"
        else
            "$compiler" run tests/fixtures/authority/command.rs --library-dir library --no-store \
                "${grants[@]}" --engine "$engine" -- '' '日本語 argument' \
                < "$case_dir/input" > "$case_dir/output"
        fi
        status=$?
        set -e
        [[ $status = 7 ]] || fail "$mode/$engine exited $status, expected 7"
        cmp "$case_dir/expected" "$case_dir/output" || fail "$mode/$engine changed bytes or skipped normal cleanup"
    done
done
mkdir "$case_dir/state"
export DESK_CAPS_ROOT="$case_dir/state"
lock_grants=(--grant state=env:DESK_CAPS_ROOT --grant clock --grant output --grant status)
"$compiler" build tests/fixtures/authority/command_lock.rs --library-dir library --no-store \
    "${lock_grants[@]}" -o "$case_dir/lock"
printf 'waiting\nlocked\n' > "$case_dir/lock-expected"
printf 'waiting\n' > "$case_dir/lock-waiting"
for engine in standalone native interpreter; do
    rm -f "$case_dir/lock-output"
    exec 9>"$case_dir/state/lock"
    flock -x 9
    if [[ "$engine" = standalone ]]; then
        "$case_dir/lock" 9>&- > "$case_dir/lock-output" &
    else
        "$compiler" run tests/fixtures/authority/command_lock.rs --library-dir library --no-store \
            "${lock_grants[@]}" --engine "$engine" 9>&- > "$case_dir/lock-output" &
    fi
    child=$!
    attempts=0
    while [[ ! -s "$case_dir/lock-output" ]]; do
        kill -0 "$child" 2>/dev/null || fail "$engine stopped before reaching lock_wait"
        attempts=$((attempts + 1))
        ((attempts < 500)) || fail "$engine did not reach lock_wait before deadline"
        sleep 0.01
    done
    sleep 0.02
    kill -0 "$child" 2>/dev/null || fail "$engine refused a held lock instead of waiting"
    cmp "$case_dir/lock-waiting" "$case_dir/lock-output" || fail "$engine passed a held lock"
    flock -u 9
    exec 9>&-
    set +e
    wait "$child"
    status=$?
    set -e
    child=
    [[ $status = 23 ]] || fail "$engine lock/ns exited $status, expected 23"
    cmp "$case_dir/lock-expected" "$case_dir/lock-output" || fail "$engine changed lock/ns output"
    flock -n "$case_dir/state/lock" true || fail "$engine left its state lock held"
done
end=$(date +%s%N)
awk -v elapsed="$((end - start))" 'BEGIN { printf "ok desk capabilities: exact env, argv/stdin/status cleanup, blocking lock, realtime ns; seconds=%.3f\n", elapsed / 1000000000 }'
printf 'desk capabilities load_after '; cat /proc/loadavg
