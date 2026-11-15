#!/usr/bin/env bash
# Independent direct-IR constructors exercise the checker, never frontend verdicts.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
if [[ ${1:-} == --compiler ]]; then compiler=$2; shift 2; fi
(($# == 0)) || exit 2
work=$(mktemp -d "$PWD/target/programs-process-mutate.XXXXXX")
before=$(cut -d' ' -f1 /proc/loadavg)
started=$(date +%s%N)
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if ((code)); then for file in "$work/error" "$work/output"; do [[ ! -f $file ]] || cat "$file" >&2; done; fi
    local elapsed after
    elapsed=$(($(date +%s%N)-started));after=$(cut -d' ' -f1 /proc/loadavg)
    awk -v ns="$elapsed" -v before="$before" -v after="$after" -v code="$code" 'BEGIN {printf "process mutation gate %.3f s load %s -> %s status=%s\n",ns/1000000000,before,after,code}'
    rm -rf -- "$work";exit "$code"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
"$compiler" build src/programs_process_mutation.lkj --library-dir library --no-store --grant output -o "$work/mutation" >"$work/output" 2>"$work/error"
timeout --kill-after=2s 60s "$work/mutation" >"$work/output" 2>"$work/error"
cat "$work/output"
for file in tests/fixtures/process/region_ir_*control.lkjir; do
    "$compiler" check "$file" --no-library --no-store >"$work/output" 2>"$work/error"
done
for file in tests/fixtures/process/region_fail_*; do
    if "$compiler" check "$file" --library-dir library --no-store >"$work/output" 2>"$work/error"; then printf 'unexpected region mutant acceptance: %s\n' "$file" >&2;exit 1;fi
    rg -q 'E_BORROW|E_TYPE' "$work/error"
    if rg -q 'E_IR_TEXT|E_PARSE' "$work/error"; then exit 1;fi
done
printf 'ok Region source/IR boundary: two text controls, eight owner/mutability/return/heap rejections\n'
