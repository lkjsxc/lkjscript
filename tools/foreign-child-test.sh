#!/usr/bin/env bash
# Default language gate: no C or Rust compilation, no helper used to launch children.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        *) printf 'usage: %s [--compiler PATH]\n' "$0" >&2; exit 2 ;;
    esac
done
mkdir -p target
work=$(mktemp -d "$PWD/target/programs-foreign-gate.XXXXXX")
cleanup() {
    local code=$?
    trap - EXIT
    if ((code)); then
        for log in "$work/build.err" "$work/output" "$work/error"; do [[ ! -f "$log" ]] || cat "$log" >&2; done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
before=$(cat /proc/loadavg)
started=$(date +%s%N)
export LKJSCRIPT_FOREIGN_ROOT="$work/root"
root="$LKJSCRIPT_FOREIGN_ROOT"
mkdir -p "$root/.runtime" "$work/outside"
cp -L /lib64/ld-linux-x86-64.so.2 "$root/.runtime/ld.so"
cp -L /lib/x86_64-linux-gnu/libc.so.6 "$root/.runtime/libc.so.6"
cp -L /lib/x86_64-linux-gnu/libpcre2-8.so.0 "$root/.runtime/libpcre2-8.so.0"
cp -L /lib/x86_64-linux-gnu/libz.so.1 "$root/.runtime/libz.so.1"
for repository in "$root" "$work/outside"; do
    env -i PATH=/usr/bin:/bin GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null /usr/bin/git init -q "$repository"
done
printf 'inside\n' >"$root/inside.txt"
printf 'outside secret\n' >"$work/outside/secret.txt"
fixture=tests/fixtures/authority/foreign_git.lkj
grants=(--grant tool=/usr/bin/git --grant root=env:LKJSCRIPT_FOREIGN_ROOT --grant sink --grant arguments --grant output)
"$compiler" build "$fixture" --library-dir library --no-store "${grants[@]}" -o "$work/foreign" 2>"$work/build.err"
"$compiler" build examples/foreign-status --library-dir library --no-store -o "$work/git-status" 2>>"$work/build.err"
"$compiler" authority examples/foreign-status --library-dir library >"$work/authority"
for authority in ForeignCommand NullDevice ReadDirectory 'LD_LIBRARY_PATH=/proc/self/fd/5/.runtime'; do
    grep -Fq "$authority" "$work/authority"
done
unavailable=no
for engine in standalone native interpreter; do
    if [[ "$engine" = standalone ]]; then
        "$work/foreign" "$work/outside" >"$work/output" 2>"$work/error"
    else
        "$compiler" run "$fixture" --library-dir library --no-store "${grants[@]}" --engine "$engine" -- "$work/outside" >"$work/output" 2>"$work/error"
    fi
    test ! -s "$work/error"
    if grep -Eq '^ok foreign kernel unavailable, execution refused:' "$work/output"; then
        unavailable=yes
        cat "$work/output"
        continue
    fi
    grep -Eq '^ok foreign git read root, outside refusal, capture, identity, arguments$' "$work/output"
    if [[ "$engine" = standalone ]]; then "$work/git-status" >"$work/output" 2>"$work/error"
    else "$compiler" run examples/foreign-status --library-dir library --no-store --engine "$engine" >"$work/output" 2>"$work/error"
    fi
    grep -Eq '^\?\? inside.txt$' "$work/output"
    test ! -s "$work/error"
done
# A reviewed executable pathname is still required to resolve to a regular file.
mkfifo "$work/image-fifo"
"$compiler" build "$fixture" --entry acquire --library-dir library --no-store --grant "tool=$work/image-fifo" --grant _errors -o "$work/acquire" 2>>"$work/build.err"
for engine in standalone native interpreter; do
    if [[ "$engine" = standalone ]]; then command=("$work/acquire")
    else command=("$compiler" run "$fixture" --entry acquire --library-dir library --no-store --grant "tool=$work/image-fifo" --grant _errors --engine "$engine")
    fi
    if timeout --kill-after=1s 5s "${command[@]}" >"$work/output" 2>"$work/error"; then
        printf 'FIFO executable grant was admitted\n' >&2; exit 1
    fi
    test "$(wc -l < "$work/error")" == 1
    acquire_line=$(awk '/^pub fn acquire\(/ {print NR}' "$fixture")
    grep -Fxq "$fixture:$acquire_line: R0007 ForeignCommand $work/image-fifo validate-image: ENOEXEC (8)" "$work/error"
done
printf 'outside secret\n' >"$work/expected"
cmp "$work/expected" "$work/outside/secret.txt"
elapsed=$(($(date +%s%N)-started))
printf 'ok foreign-command gate (kernel unavailable=%s); seconds=' "$unavailable"
awk -v ns="$elapsed" 'BEGIN {printf "%.3f", ns/1000000000}'
printf '; load %s -> %s\n' "$before" "$(cat /proc/loadavg)"
