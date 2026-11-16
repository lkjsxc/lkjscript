#!/usr/bin/env bash
# A live compiler producer receives framed IR after readiness, then admits only
# zero-authority entries. Each invocation ends before any next frame is consumed.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
reuse=()
if [[ -n "${LKJSCRIPT_STORE_KEY:-}" ]]; then
    reuse=(--store "${LKJSCRIPT_STORE:-target/store}" --store-key "$LKJSCRIPT_STORE_KEY")
fi
directory=$(mktemp -d target/programs-admission-test.XXXXXX)
host_pid=
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$host_pid" ]]; then
        kill "$host_pid" 2>/dev/null || true
        wait "$host_pid" 2>/dev/null || true
    fi
    rm -rf "$directory"
    exit "$status"
}
trap cleanup EXIT HUP INT TERM
read -r load_before _ < /proc/loadavg
started=$(date +%s%N)
if [[ -n "${LKJSCRIPT_ADMISSION_HOST:-}" ]]; then
    host=$LKJSCRIPT_ADMISSION_HOST
else
    "$compiler" build src/programs_admission_main.lkj --library-dir library --runtime-source library/runtime.lkj \
        -o "$directory/host" "${reuse[@]}" >"$directory/build.log" 2>&1 || { cat "$directory/build.log" >&2; exit 1; }
    host="$directory/host"
fi
cat >"$directory/header" <<'IR'
lkjscript-ir 1
type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
IR
cat "$directory/header" >"$directory/value.lkjir"
cat >>"$directory/value.lkjir" <<'IR'
source "runtime-input.lkjir"
fn "probe" 0 3
local "answer" 3 let
@21 int 0 42
@22 return 0
endfn
IR
cat "$directory/header" >"$directory/trap.lkjir"
cat >>"$directory/trap.lkjir" <<'IR'
source "runtime-trap.lkjir"
fn "probe" 0 3
@41 trap "admitted trap"
endfn
IR
cat "$directory/header" >"$directory/authority.lkjir"
cat >>"$directory/authority.lkjir" <<'IR'
type "Stdout" stdout
fn "probe" 1 3
local "output" 7 let
local "answer" 3 let
@1 int 1 42
@2 drop 0
@3 return 1
endfn
IR
cat "$directory/header" >"$directory/loop.lkjir"
cat >>"$directory/loop.lkjir" <<'IR'
fn "probe" 0 3
@1 loop
@2 end
endfn
IR
mkfifo "$directory/input" "$directory/output"
exec {input}<>"$directory/input"
exec {output}<>"$directory/output"
"$host" 500000000 <"$directory/input" >"$directory/output" 2>"$directory/errors" &
host_pid=$!
read -r -t 120 ready <&"$output" || { cat "$directory/errors" >&2; exit 1; }
[[ "$ready" == 'ready producer='*' authority=World admitted-authority=none execution=attached-process' ]]
# No payload existed on the input stream until the live producer announced ready.
frame() {
    local file=$1 length octet index
    length=$(wc -c <"$file")
    for ((index=0; index<8; index++)); do
        printf -v octet '%03o' "$(( (length >> (index * 8)) & 255 ))"
        printf '%b' "\\$octet" >&"$input"
    done
    cat "$file" >&"$input"
}
reply() {
    read -r -t 120 response <&"$output" || { cat "$directory/errors" >&2; exit 1; }
    printf '%s\n' "$response" >>"$directory/replies"
}
frame "$directory/value.lkjir"; reply
[[ "$response" == '"returned kind=i64 bits=42 image='* ]]
frame "$directory/authority.lkjir"; reply
[[ "$response" == '"refused E_ADMIT_AUTHORITY '* ]]
frame "$directory/trap.lkjir"; reply
[[ "$response" == '"trapped runtime-trap.lkjir:41: R'*'admitted trap image='* ]]
frame "$directory/value.lkjir"; reply
[[ "$response" == '"returned kind=i64 bits=42 image='* ]]
# The same build-time checker must refuse every direct IR mutation, then the
# already-running producer must independently refuse it without losing service.
mutations=0
for replacement in 'return 7' 'return 1' 'read 0 7' 'call 0 5000' 'intrinsic 0 "world_file" 0' 'int 0 18446744073709551616'; do
    cat "$directory/header" >"$directory/mutant.lkjir"
    printf 'fn "probe" 0 3\nlocal "answer" 3 let\nlocal "uninitialized" 3 let\n@1 %s\nendfn\n' "$replacement" >>"$directory/mutant.lkjir"
    if "$compiler" check "$directory/mutant.lkjir" >"$directory/check" 2>&1; then
        printf 'build-time checker admitted intended mutant: %s\n' "$replacement" >&2
        exit 1
    fi
    frame "$directory/mutant.lkjir"; reply
    [[ "$response" == '"refused '* ]]
    mutations=$((mutations + 1))
done
frame "$directory/loop.lkjir"; reply
[[ "$response" == '"deadline image='* ]]
frame "$directory/value.lkjir"; reply
[[ "$response" == '"returned kind=i64 bits=42 image='* ]]
printf '\0\0\0\0\0\0\0\0' >&"$input"
read -r -t 120 response <&"$output"
[[ "$response" == "finished requests=$((6 + mutations))" ]]
wait "$host_pid"
host_pid=
[[ ! -s "$directory/errors" ]]
read -r load_after _ < /proc/loadavg
elapsed=$(($(date +%s%N) - started))
awk -v ns="$elapsed" -v before="$load_before" -v after="$load_after" -v mutations="$mutations" \
    'BEGIN { printf "ok runtime IR admission: live input, empty authority, %d checker mutants, located trap, deadline, recovery (%.3f s, load %s -> %s)\n", mutations, ns/1000000000, before, after }'
