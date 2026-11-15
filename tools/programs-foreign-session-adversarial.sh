#!/usr/bin/env bash
# Optional independent OS adversaries/supervisor. No C code is a runtime host.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
if [[ ${1:-} == --compiler ]]; then compiler=$2; shift 2; fi
[[ $# == 0 ]] || exit 2
[[ $compiler == /* ]] || compiler="$PWD/$compiler"
work=$(mktemp -d "$PWD/target/programs-foreign-session-adversarial.XXXXXX")
cleanup() { local status=$?; trap - EXIT; if ((status)); then for log in "$work/build.err" "$work/output" "$work/error"; do [[ ! -f "$log" ]] || cat "$log" >&2; done; fi; rm -rf -- "$work"; exit "$status"; }
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
read -r before rest < /proc/loadavg
started=$(date +%s%N)
export LKJSCRIPT_FOREIGN_ROOT="$work/root"
mkdir -p "$LKJSCRIPT_FOREIGN_ROOT"
cc -std=c11 -static -O2 -Wall -Wextra -Werror tools/programs-foreign-session-adversary.c -o "$work/probe"
cc -std=c11 -O2 -Wall -Wextra -Werror tools/programs-foreign-session-lifecycle.c -o "$work/lifecycle"
cp "$work/probe" "$LKJSCRIPT_FOREIGN_ROOT/reexec"
printf inside > "$LKJSCRIPT_FOREIGN_ROOT/inside"
printf secret > "$work/outside"
ln -s "$work/outside" "$LKJSCRIPT_FOREIGN_ROOT/escape"
fixture=tests/fixtures/authority/foreign_session_proof.lkj
base=(--grant "tool=$work/probe" --grant root=env:LKJSCRIPT_FOREIGN_ROOT --grant sink)
for entry in main owner unavailable; do
    grants=("${base[@]}")
    if [[ $entry == main ]]; then grants+=(--grant arguments --grant output)
    elif [[ $entry == owner ]]; then grants+=(--grant clock)
    else grants+=(--grant output); fi
    "$compiler" build "$fixture" --entry "$entry" --library-dir library --no-store "${grants[@]}" -o "$work/$entry" 2>>"$work/build.err"
done
exec 100<"$work/outside"
export LKJSCRIPT_FOREIGN_UNGRANTED_SECRET=must-not-arrive
for engine in standalone native interpreter; do
    read -r engine_before rest < /proc/loadavg
    if [[ $engine == standalone ]]; then command=("$work/main" "$work/outside")
    else command=("$compiler" run "$fixture" --library-dir library --no-store --engine "$engine" "${base[@]}" --grant arguments --grant output -- "$work/outside"); fi
    timeout --kill-after=2s 60s "${command[@]}" >"$work/output" 2>"$work/error"
    test ! -s "$work/error"; rg -Fq 'ok persistent confinement:' "$work/output"; cat "$work/output"
    if [[ $engine == standalone ]]; then command=("$work/owner")
    else command=("$compiler" run "$fixture" --entry owner --library-dir library --no-store --engine "$engine" "${base[@]}" --grant clock); fi
    for round in 1 2 3 4; do "$work/lifecycle" --parent-death "${command[@]}" >"$work/output" 2>"$work/error"; test ! -s "$work/error"; done
    if [[ $engine == standalone ]]; then command=("$work/unavailable")
    else command=("$compiler" run "$fixture" --entry unavailable --library-dir library --no-store --engine "$engine" "${base[@]}" --grant output); fi
    "$work/lifecycle" --unavailable "${command[@]}" >"$work/output" 2>"$work/error"
    test ! -s "$work/error"; rg -Fq 'unsupported kernel refuses' "$work/output"
    read -r engine_after rest < /proc/loadavg
    printf 'ok %s: confinement, four owner kills/reaps, fail-closed; load %s -> %s\n' "$engine" "$engine_before" "$engine_after"
done
exec 100<&-
read -r after rest < /proc/loadavg
elapsed=$(($(date +%s%N)-started))
awk -v elapsed="$elapsed" -v before="$before" -v after="$after" 'BEGIN{printf "ok foreign-session adversarial seconds=%.3f; load %s -> %s\n",elapsed/1000000000,before,after}'
