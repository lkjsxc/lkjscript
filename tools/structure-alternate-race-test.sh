#!/usr/bin/env bash
# One syscall-boundary mutation after the alternate module probe returns ENOENT.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
mkdir -p target
fixture=$(mktemp -d "$PWD/target/compiler-alternate-race.XXXXXX")
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if ((code)); then
        for file in "$fixture/build.err" "$fixture/run.out" "$fixture/run.err"; do
            if test -f "$file"; then cat "$file" >&2; fi
        done
    fi
    rm -rf -- "$fixture"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
bounded() { prlimit --as=4294967296 -- timeout --kill-after=5s 900s "$@"; }
started=$(date +%s%N)
load_before=$(cat /proc/loadavg)
bounded cc -std=c11 -O2 -Wall -Wextra -Werror tools/structure-alternate-race.c -o "$fixture/race" > "$fixture/build.out" 2> "$fixture/build.err"
mkdir "$fixture/source" "$fixture/source/added"
# A failing child without the exact syscall trigger must never satisfy the witness.
if bounded "$fixture/race" "$fixture/source/added" /bin/false > "$fixture/control.out" 2> "$fixture/control.err"; then exit 1; fi
grep -q 'the exact negative alternate probe was never exercised' "$fixture/control.err"
test ! -e "$fixture/source/added/mod.lkj"
printf '%s\n' 'mod keep;' 'fn main() -> i64 { keep::value() }' > "$fixture/source/main.lkj"
printf '%s\n' '// untouched alternate-race witness' 'pub fn value() -> i64 { 1i64 }' > "$fixture/source/keep.lkj"
cp "$fixture/source/main.lkj" "$fixture/main-before"
cp "$fixture/source/keep.lkj" "$fixture/keep-before"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/request.json" <<'JSON'
{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub mod added;","projection":{"path":"added.lkj","source":"pub fn value() -> i64 { 42i64 }\n"}},{"op":"replace-body","id":"main","source":"{ added::value() }"}]}
JSON
bounded "$fixture/race" "$fixture/source/added" "$compiler" edit main.lkj --source-dir "$fixture/source" --no-library --no-store --request "$fixture/request.json" --store-key "$fixture/key" > "$fixture/run.out" 2> "$fixture/run.err"
grep -Fxq 'ok alternate race: triggers=1 result=-ENOENT preserved=true compiler_status=1' "$fixture/run.out"
grep -q 'E_EDIT_STALE source dependency changed during checking' "$fixture/run.err"
if grep -q 'mode=apply' "$fixture/run.out"; then exit 1; fi
cmp "$fixture/main-before" "$fixture/source/main.lkj"
cmp "$fixture/keep-before" "$fixture/source/keep.lkj"
test ! -e "$fixture/source/added.lkj"
test ! -L "$fixture/source/added.lkj"
test ! -e "$fixture/source/.lkjscript-edit.pending"
test ! -e "$fixture/source/.lkjscript-edit.generation"
test -z "$(find "$fixture/source" -name '*.lkjscript-next' -print -quit)"
printf '%s\n' 'pub fn value() -> i64 { 99i64 }' > "$fixture/competitor-expected"
cmp "$fixture/competitor-expected" "$fixture/source/added/mod.lkj"
# Retain both expected refusals in the caller log before removing the fixture.
cat "$fixture/control.err" "$fixture/run.err" "$fixture/run.out"
ended=$(date +%s%N)
load_after=$(cat /proc/loadavg)
awk -v elapsed="$((ended-started))" -v before="$load_before" -v after="$load_after" 'BEGIN { printf "ok alternate dependency race: %.3f s, load %s -> %s\n", elapsed/1000000000, before, after }'
