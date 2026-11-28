#!/usr/bin/env bash
# Pattern refusals retain exact item ownership without authorizing the whole root.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ $# -le 1 ]] || { printf 'usage: %s [compiler]\n' "$0" >&2; exit 2; }
compiler=$(realpath "${1:-target/lkjscript}")
library=$PWD/library
mkdir -p target
work=$(mktemp -d "$PWD/target/macro-pattern-recovery.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for log in error verifier-error; do
            [[ ! -s $work/$log ]] || tail -n 50 "$work/$log" >&2
        done
    fi
    if [[ ${LKJSCRIPT_MACRO_KEEP:-0} == 1 ]]; then printf 'retained_fixture=%s\n' "$work" >&2
    else rm -rf -- "$work"; fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'macro-pattern-recovery: %s\n' "$*" >&2; exit 1; }
"$compiler" build tests/fixtures/macro_patterns_recovery_verify.lkj --library-dir "$library" \
    --no-store -o "$work/verify" > "$work/verifier-output" 2> "$work/verifier-error" \
    || fail 'verifier build'
checks=0
refusals=0
for mode in off on; do
    for specimen in header body nested_header bodyless; do
        source="$work/$specimen.lkj"
        cp "tests/fixtures/macro_patterns_recovery/$specimen.lkj" "$source"
        status=0
        "$compiler" check "$source" --library-dir "$library" --debug-assertions "$mode" --no-store \
            > "$work/output" 2> "$work/error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "$specimen/$mode lost whole-source refusal"
        "$work/verify" "$source" "$specimen" < "$work/error" > "$work/verified" 2> "$work/verifier-error" \
            || fail "$specimen/$mode item inventory, cause, or tombstone ownership"
        cat "$work/verified"
        checks=$((checks + 1))
        for engine in native interpreter; do
            status=0
            "$compiler" run "$source" --entry probe --library-dir "$library" --debug-assertions "$mode" \
                --engine "$engine" --no-store > "$work/output" 2> "$work/error" || status=$?
            [[ $status == 1 && ! -s $work/output ]] || fail "$specimen/$mode/$engine executed refused source"
            grep -Fq 'E_UNSUPPORTED_MACRO unsupported macro: missing!' "$work/error" || fail 'execution lost primary refusal'
            if grep -q '^LKJSCRIPT_ITEMS ' "$work/error"; then fail 'partial item facts leaked into execution admission'; fi
            refusals=$((refusals + 1))
        done
        for command in build ir; do
            printf 'unchanged destination\n' > "$work/destination"
            cp "$work/destination" "$work/destination-before"
            status=0
            "$compiler" "$command" "$source" --entry probe --library-dir "$library" --debug-assertions "$mode" \
                --no-store -o "$work/destination" > "$work/output" 2> "$work/error" || status=$?
            [[ $status == 1 && ! -s $work/output ]] || fail "$specimen/$mode/$command admitted refused source"
            grep -Fq 'E_UNSUPPORTED_MACRO unsupported macro: missing!' "$work/error" || fail 'publication lost primary refusal'
            cmp "$work/destination-before" "$work/destination" || fail "$command replaced destination after refusal"
            if grep -q '^LKJSCRIPT_ITEMS ' "$work/error"; then fail 'partial item facts leaked into publication admission'; fi
            refusals=$((refusals + 1))
        done
    done
done
printf 'ok macro-pattern recovery: %s exact partial inventories in both debug configurations; %s native/interpreter execution and publication refusals; independent siblings retained\n' "$checks" "$refusals"
