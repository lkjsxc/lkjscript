#!/usr/bin/env bash
# Focused authority/ownership gate, with a raw protocol peer independent of both engines.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        *) printf 'usage: %s [--compiler PATH]\n' "$0" >&2; exit 2 ;;
    esac
done
# Keep AF_UNIX's pathname bound independent of the checkout's absolute path length.
work=$(mktemp -d /tmp/lkjscript-unix.XXXXXX)
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if ((code)); then
        for file in "$work/error" "$work/witness.err" "$work/output"; do [[ ! -f "$file" ]] || tail -35 "$file" >&2; done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
cp "$compiler" "$work/compiler"
compiler="$work/compiler"
cc -std=c11 -O2 -Wall -Wextra -Werror tools/unix-stream-witness.c -o "$work/witness"
mkdir "$work/root" "$work/outside"
printf 'regular' >"$work/root/regular"
ln -s "$work/outside" "$work/root/escape"
ln -s "$work/outside/socket" "$work/root/socket-link"
source=tests/fixtures/unix/stream.lkj
for engine in native interpreter; do
    for entry in main directory world; do
        args=(run "$source" --library-dir library --no-store --entry "$entry" --engine "$engine")
        case "$entry" in
            main) args+=(--grant "socket=unix:$work/root/socket" --grant "root=$work/root" --grant clock --grant ready) ;;
            directory) args+=(--grant "root=$work/root" --grant clock --grant ready) ;;
            world) args+=(-- "$work/root") ;;
        esac
        timeout --kill-after=3s 60s "$work/witness" "$work/root" -- "$compiler" "${args[@]}" >"$work/output" 2>"$work/witness.err"
        grep -qx 42 "$work/output"
        printf 'ok Unix %s %s: Wayland byte wire, bidirectional owned rights, bounded cleanup\n' "$entry" "$engine"
    done
    timeout --kill-after=3s 60s "$work/witness" "$work/root" --close-stdin -- "$compiler" run "$source" --library-dir library --no-store --engine "$engine" --grant socket=unix:env:LKJSCRIPT_UNIX_TEST_SOCKET --grant root=env:LKJSCRIPT_UNIX_TEST_ROOT --grant clock --grant ready >"$work/output" 2>"$work/witness.err"
    grep -qx 42 "$work/output"
    printf 'ok Unix environment grants %s, closed stdin preserves descriptor sentinel\n' "$engine"
    mkdir "$work/replacement-$engine"
    timeout --kill-after=3s 60s "$work/witness" --replacement "$work/replacement-$engine" -- "$compiler" run "$source" --library-dir library --no-store --entry pinned --engine "$engine" --grant "root=$work/replacement-$engine" >"$work/output" 2>"$work/witness.err"
    grep -qx 42 "$work/output"
    timeout --kill-after=3s 60s "$work/witness" --tcp "$work/root" -- "$compiler" run "$source" --library-dir library --no-store --entry wrong_socket --engine "$engine" --grant @TCP@ --grant "root=$work/root" >"$work/output" 2>"$work/witness.err"
    grep -qx 42 "$work/output"
    printf 'ok Unix %s: live path replacement stays pinned; TCP rights refusal closes consumed owners\n' "$engine"
    "$compiler" run "$source" --library-dir library --no-store --entry confined --engine "$engine" --grant "root=$work/root" >"$work/output" 2>"$work/error"
    grep -qx 42 "$work/output"
done
"$compiler" build "$source" --library-dir library --no-store --grant socket=unix:env:LKJSCRIPT_UNIX_TEST_SOCKET --grant root=env:LKJSCRIPT_UNIX_TEST_ROOT --grant clock --grant ready -o "$work/standalone" >"$work/output" 2>"$work/error"
timeout --kill-after=3s 60s "$work/witness" "$work/root" --close-stdin -- "$work/standalone" >"$work/output" 2>"$work/witness.err"
[[ ! -s "$work/output" ]]
printf 'ok Unix standalone resolves only explicit environment selectors at startup\n'
"$compiler" authority "$source" --library-dir library --no-store --grant "socket=unix:$work/root/socket" --grant "root=$work/root" --grant clock --grant ready >"$work/authority" 2>"$work/error"
grep -q 'File Unix stream' "$work/authority"
grep -q "unix:$work/root/socket" "$work/authority"
grep -q 'file_send_message' "$work/authority"
grep -q 'file_receive_message' "$work/authority"
if grep -q 'world_unix_connect' "$work/authority"; then printf 'unreachable World effect entered the main report\n' >&2; exit 1; fi
"$compiler" check tests/fixtures/unix/raw_control.lkjir --no-library --no-store >"$work/output" 2>"$work/error"
for fixture in tests/fixtures/unix/raw_refuse_*.lkjir; do
    if "$compiler" check "$fixture" --no-library --no-store >"$work/output" 2>"$work/error"; then printf 'accepted raw IR ownership/signature mutant: %s\n' "$fixture" >&2; exit 1; fi
    grep -q E_TYPE "$work/error"
    if grep -Eq 'E_IR_TEXT|E_PARSE' "$work/error"; then exit 1; fi
done
if "$compiler" run tests/fixtures/unix/raw_refuse_integer_owner.lkjir --no-library --no-store --unchecked-ir --engine interpreter >"$work/output" 2>"$work/error"; then printf 'interpreter accepted descriptor integer forgery\n' >&2; exit 1; fi
for fixture in tests/fixtures/unix/refuse_*.lkj; do
    if "$compiler" check "$fixture" --library-dir library --no-store >"$work/output" 2>"$work/error"; then printf 'accepted refusal: %s\n' "$fixture" >&2; exit 1; fi
    grep -Eq 'E_(TYPE|BORROW|MOVE|OWNERSHIP|UNINITIALIZED|AFFINE|NAME|RESOLVE|FORM|UNSUPPORTED)' "$work/error"
done
for grant in "$work/root/socket" unix:relative unix:env:; do
    if "$compiler" build "$source" --library-dir library --no-store --grant "socket=$grant" --grant "root=$work/root" --grant clock --grant ready -o "$work/refused" >"$work/output" 2>"$work/error"; then printf 'accepted invalid socket grant: %s\n' "$grant" >&2; exit 1; fi
done
if "$compiler" build "$source" --library-dir library --no-store --grant "root=$work/root" --grant clock --grant ready -o "$work/refused" >"$work/output" 2>"$work/error"; then printf 'accepted missing socket grant\n' >&2; exit 1; fi
for engine in native interpreter; do
    if env -u LKJSCRIPT_UNIX_MISSING "$compiler" run "$source" --library-dir library --no-store --engine "$engine" --grant socket=unix:env:LKJSCRIPT_UNIX_MISSING --grant "root=$work/root" --grant clock --grant ready >"$work/output" 2>"$work/error"; then printf 'accepted missing environment selector\n' >&2; exit 1; fi
    grep -q 'File unix:env:LKJSCRIPT_UNIX_MISSING' "$work/error"
done
status=0
env -u LKJSCRIPT_UNIX_TEST_SOCKET "$work/standalone" >"$work/output" 2>"$work/error" || status=$?
[[ $status == 1 && ! -s "$work/output" && ! -s "$work/error" ]]
# A standalone image may report the acquisition failure only with output authority.
"$compiler" build tests/fixtures/unix/entry_diagnostic.lkj --library-dir library --no-store --grant socket=unix:env:LKJSCRIPT_UNIX_TEST_SOCKET --grant errors -o "$work/diagnostic" >"$work/output" 2>"$work/error"
status=0
env -u LKJSCRIPT_UNIX_TEST_SOCKET "$work/diagnostic" >"$work/output" 2>"$work/error" || status=$?
[[ $status == 1 && ! -s "$work/output" ]]
grep -Fxq 'tests/fixtures/unix/entry_diagnostic.lkj:1: R0007 File unix:env:LKJSCRIPT_UNIX_TEST_SOCKET lookup: ENOENT (2)' "$work/error"
[[ $(wc -l < "$work/error") == 1 ]]
# Runtime feature bit 18 must survive separate unit admission and linking.
mkdir "$work/units" "$work/store"
cp "$source" "$work/units/app.lkj"
cat >"$work/units/manifest.lkj" <<'MANIFEST'
const UNITS: &[&str] = &["app=app.lkj"];
const DEPENDS: &[&str] = &[];
const ENTRY: &str = "app::main";
MANIFEST
head -c 32 /dev/urandom >"$work/store-key"
for state in cold warm; do
    "$compiler" build-units manifest.lkj --source-dir "$work/units" --library-dir library --store "$work/store" --store-key "$work/store-key" --grant socket=unix:env:LKJSCRIPT_UNIX_TEST_SOCKET --grant root=env:LKJSCRIPT_UNIX_TEST_ROOT --grant clock --grant ready -o "$work/unit-$state" >"$work/output" 2>"$work/error"
    timeout --kill-after=3s 60s "$work/witness" "$work/root" -- "$work/unit-$state" >"$work/output" 2>"$work/witness.err"
    [[ ! -s "$work/output" ]]
done
cmp "$work/unit-cold" "$work/unit-warm"
printf 'ok Unix cold/warm unit-linked native images; no ambient grant, integer forge, read-only connect or sent-owner reuse\n'
