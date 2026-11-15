#!/usr/bin/env bash
# Full language proof. C files are optional test adversaries/supervisors, never runtime helpers.
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
ulimit -c 0
mkdir -p target
work=$(mktemp -d "$PWD/target/programs-foreign.XXXXXX")
child=
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$child" ]]; then kill -KILL "$child" 2>/dev/null || true; wait "$child" 2>/dev/null || true; fi
    if ((code)); then
        for log in "$work/build.err" "$work/output" "$work/error" "$work/proof"; do [[ ! -f "$log" ]] || cat "$log" >&2; done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
before=$(cat /proc/loadavg)
started=$(date +%s%N)
export LKJSCRIPT_FOREIGN_ROOT="$work/root"
root="$LKJSCRIPT_FOREIGN_ROOT"
mkdir -p "$root/.runtime" "$work/no-runtime"
cc -std=c11 -O2 -Wall -Wextra -Werror tools/foreign-child-proof.c -o "$work/probe-original"
cc -std=c11 -O2 -Wall -Wextra -Werror tools/foreign-child-lifecycle.c -o "$work/lifecycle"
cp "$work/probe-original" "$work/probe"
cp -L /lib64/ld-linux-x86-64.so.2 "$root/.runtime/ld.so"
cp -L /lib/x86_64-linux-gnu/libc.so.6 "$root/.runtime/libc.so.6"
cp -L /lib/x86_64-linux-gnu/libpcre2-8.so.0 "$root/.runtime/libpcre2-8.so.0"
cp -L /lib/x86_64-linux-gnu/libz.so.1 "$root/.runtime/libz.so.1"
env -i PATH=/usr/bin:/bin GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null /usr/bin/git init -q "$root"
printf 'inside\n' >"$root/inside.txt"
printf 'outside secret\n' >"$work/outside.txt"
ln -s "$work/outside.txt" "$root/escape"
# Also leaves a rebased, permitted reexec adversary inside the fresh root.
"$work/probe-original" "$root" "$work/outside.txt" "$work/probe-original" >"$work/proof"
fixture=tests/fixtures/authority/foreign_command.lkj
base=(--grant "tool=$work/probe" --grant root=env:LKJSCRIPT_FOREIGN_ROOT --grant sink)
grants_for() {
    grants=("${base[@]}")
    case "$1" in
        main) grants+=(--grant arguments --grant output --grant clock) ;;
        snapshot) grants+=(--grant input --grant output) ;;
        owner|owner_marked) ;;
        *) grants+=(--grant output) ;;
    esac
}
for entry in main missing_runtime unsupported snapshot owner owner_marked; do
    grants_for "$entry"
    "$compiler" build "$fixture" --entry "$entry" --library-dir library --no-store "${grants[@]}" -o "$work/$entry" 2>>"$work/build.err"
done
"$compiler" build examples/foreign-status --library-dir library --no-store -o "$work/git-status" 2>>"$work/build.err"
"$compiler" authority examples/foreign-status --library-dir library >"$work/authority"
grep -Eq 'ForeignCommand' "$work/authority"
grep -Eq 'NullDevice' "$work/authority"
grep -Eq 'ReadDirectory' "$work/authority"
grep -Eq 'LD_LIBRARY_PATH=/proc/self/fd/5/.runtime' "$work/authority"
run_entry() {
    local entry=$1; shift
    grants_for "$entry"
    if [[ "$engine" = standalone ]]; then
        "$work/$entry" "$@"
    else
        "$compiler" run "$fixture" --entry "$entry" --library-dir library --no-store "${grants[@]}" --engine "$engine" -- "$@"
    fi
}
command_for() {
    local entry=$1
    grants_for "$entry"
    if [[ "$engine" = standalone ]]; then command=("$work/$entry")
    else command=("$compiler" run "$fixture" --entry "$entry" --library-dir library --no-store "${grants[@]}" --engine "$engine")
    fi
}
for engine in standalone native interpreter; do
    cp "$work/probe-original" "$work/probe"
    export LKJSCRIPT_FOREIGN_ROOT="$root"
    # Deliberately inherited input FD and environment must not enter foreign code.
    exec 100<"$work/outside.txt"
    export LKJSCRIPT_FOREIGN_UNGRANTED_SECRET=must-not-arrive
    run_entry main "$root" "$work/outside.txt" >"$work/output" 2>"$work/error"
    exec 100<&-
    grep -Eq '^ok foreign capture, exit, signal, deadline, confined reexec, identity$' "$work/output"
    test ! -s "$work/error"
    if [[ "$engine" = standalone ]]; then "$work/git-status" >"$work/output" 2>"$work/error"
    else "$compiler" run examples/foreign-status --library-dir library --no-store --engine "$engine" >"$work/output" 2>"$work/error"
    fi
    grep -Eq '^\?\? inside.txt$' "$work/output"
    test ! -s "$work/error"
    export LKJSCRIPT_FOREIGN_ROOT="$work/no-runtime"
    run_entry missing_runtime >"$work/output" 2>"$work/error"
    grep -Eq '^ok launch errno and reap$' "$work/output"
    test ! -s "$work/error"
    export LKJSCRIPT_FOREIGN_ROOT="$root"
    command_for unsupported
    "$work/lifecycle" --unavailable "${command[@]}" >"$work/output" 2>"$work/error"
    grep -Eq '^ok unavailable kernel fails closed$' "$work/output"
    test ! -s "$work/error"
    mkfifo "$work/control"
    exec 9<>"$work/control"
    command_for snapshot
    "${command[@]}" <"$work/control" 9>&- >"$work/output" 2>"$work/error" &
    child=$!
    ready=no
    for ((attempt=0;attempt<1000;attempt++)); do
        if grep -Eq '^snapshot acquired$' "$work/output"; then ready=yes; break; fi
        kill -0 "$child" 2>/dev/null || break
        sleep 0.01
    done
    [[ "$ready" = yes ]]
    cp /bin/false "$work/probe"
    printf 'x' >&9
    wait "$child"
    child=
    exec 9>&-
    rm "$work/control"
    printf 'snapshot acquired\nok immutable initial image\n' >"$work/expected"
    cmp "$work/expected" "$work/output"
    test ! -s "$work/error"
    cp "$work/probe-original" "$work/probe"
    command_for owner
    "$work/lifecycle" --parent-death "${command[@]}" >"$work/output" 2>"$work/error"
    grep -Eq '^ok language owner death and descendant reap$' "$work/output"
    test ! -s "$work/error"
    chmod 6755 "$root/second-probe"
    [[ "$(stat -c %a "$root/second-probe")" = 6755 ]]
    command_for owner_marked
    "$work/lifecycle" --parent-death "${command[@]}" >"$work/output" 2>"$work/error"
    grep -Eq '^ok language owner death and descendant reap$' "$work/output"
    test ! -s "$work/error"
    chmod 755 "$root/second-probe"
done
elapsed=$(($(date +%s%N)-started))
printf 'ok foreign command: git, confinement/reexec, captures/status, deadline/reap, parent death, immutable image, fail-closed host; seconds='
awk -v ns="$elapsed" 'BEGIN {printf "%.3f", ns/1000000000}'
printf '; load %s -> %s\n' "$before" "$(cat /proc/loadavg)"
