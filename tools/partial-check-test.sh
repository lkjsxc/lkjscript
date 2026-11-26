#!/usr/bin/env bash
# Partial source diagnostics never authorize a source program or erase its refusals.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ $# -le 3 ]] || { printf 'usage: %s [compiler [rustc [baseline-compiler]]]\n' "$0" >&2; exit 2; }
compiler=$(realpath "${1:-target/lkjscript}")
witness=${2:-}
baseline=${3:-}
if [[ -n $witness ]]; then witness=$(command -v "$witness"); fi
if [[ -n $baseline ]]; then baseline=$(realpath "$baseline"); fi
library=$PWD/library
mkdir -p target
work=$(mktemp -d "$PWD/target/partial-check.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for name in error verifier-error rust-error; do
            if [[ -s $work/$name ]]; then tail -n 40 "$work/$name" >&2; fi
        done
    fi
    if [[ ${LKJSCRIPT_PARTIAL_KEEP:-0} == 1 ]]; then printf 'retained_fixture=%s\n' "$work" >&2
    else rm -rf -- "$work"; fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'partial-check: %s\n' "$*" >&2; exit 1; }
mkdir "$work/cases"
cp tests/fixtures/partial_check/*.lkj "$work/cases/"
verifier_compiler=${baseline:-$compiler}
"$verifier_compiler" build tests/fixtures/partial_check_verify.lkj --library-dir "$library" --no-store -o "$work/verify" > "$work/verifier-output" 2> "$work/verifier-error" || fail 'verifier build'
cases=(independent unsafe_binding local_import global_static mutable_static qualified_header global_macro glob_import trait_impl type_failure qualified_import normalized_self import_chain library_inputs)
checks=0
refusals=0
for mode in on off; do
    expected=42
    if [[ $mode == off ]]; then expected=17; fi
    "$compiler" check "$work/cases/valid.lkj" --library-dir "$library" --debug-assertions "$mode" --no-store > "$work/output" 2> "$work/error"
    [[ ! -s $work/output && ! -s $work/error ]] || fail "valid $mode check changed its silent contract"
    "$compiler" ir "$work/cases/valid.lkj" --library-dir "$library" --debug-assertions "$mode" --no-store -o "$work/valid-$mode.lkjir" > "$work/output" 2> "$work/error"
    for engine in native interpreter; do
        actual=$("$compiler" run "$work/cases/valid.lkj" --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store 2> "$work/error")
        [[ $actual == "$expected" ]] || fail "valid $mode/$engine value"
        actual=$("$compiler" run "$work/valid-$mode.lkjir" --entry probe --engine "$engine" --no-store 2> "$work/error")
        [[ $actual == "$expected" ]] || fail "valid serialized $mode/$engine value"
        checks=$((checks + 2))
    done
    if [[ -n $baseline ]]; then
        "$baseline" ir "$work/cases/valid.lkj" --library-dir "$library" --debug-assertions "$mode" --no-store -o "$work/baseline-$mode.lkjir" > "$work/output" 2> "$work/error"
        cmp "$work/baseline-$mode.lkjir" "$work/valid-$mode.lkjir" || fail "valid $mode IR changed"
        for engine in native interpreter; do
            actual=$("$baseline" run "$work/cases/valid.lkj" --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store 2> "$work/error")
            [[ $actual == "$expected" ]] || fail "baseline valid $mode/$engine value"
        done
    fi
    for name in "${cases[@]}"; do
        source="$work/cases/$name.lkj"
        status=0
        "$compiler" check "$source" --library-dir "$library" --debug-assertions "$mode" --no-store --stats > "$work/output" 2> "$work/error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "$name/$mode must keep the source refusal"
        "$work/verify" "$source" "$name" < "$work/error" > "$work/verified" 2> "$work/verifier-error" || fail "$name/$mode item inventory or upstream-cause mismatch"
        cat "$work/verified"
        if [[ -n $baseline ]]; then
            status=0
            "$baseline" check "$source" --library-dir "$library" --debug-assertions "$mode" --no-store > "$work/baseline-output" 2> "$work/baseline-error" || status=$?
            [[ $status == 1 && ! -s $work/baseline-output ]] || fail "baseline $name/$mode refusal"
            while IFS= read -r line; do
                [[ $line == *': E_'* && $line != *': E_BLOCKED '* ]] || continue
                grep -Fxq -- "$line" "$work/error" || fail "$name/$mode lost an original diagnostic"
            done < "$work/baseline-error"
        fi
        for engine in native interpreter; do
            status=0
            "$compiler" run "$source" --entry probe --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store > "$work/output" 2> "$work/refused-error" || status=$?
            [[ $status == 1 && ! -s $work/output ]] || fail "$name/$mode/$engine executed an invalid root"
            if grep -q '^LKJSCRIPT_ITEMS ' "$work/refused-error"; then fail 'item reporting leaked into execution admission'; fi
            refusals=$((refusals + 1))
        done
    done
    # Even when probe itself checks independently, the invalid source cannot publish an image or IR.
    for command in build ir; do
        printf 'unreplaced destination\n' > "$work/destination"
        cp "$work/destination" "$work/destination-before"
        status=0
        "$compiler" "$command" "$work/cases/independent.lkj" --entry probe --library-dir "$library" --debug-assertions "$mode" --no-store -o "$work/destination" > "$work/output" 2> "$work/refused-error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "$command published an invalid source root"
        cmp "$work/destination-before" "$work/destination" || fail "$command changed the destination after refusal"
        if grep -q '^LKJSCRIPT_ITEMS ' "$work/refused-error"; then fail 'item reporting leaked into artifact admission'; fi
        refusals=$((refusals + 1))
    done
done
printf 'ok partial-check: both configurations, authored item IDs/counts/dependencies/causes, %s source/IR value routes and %s retained execution/publication refusals\n' "$checks" "$refusals"
if [[ -n $baseline ]]; then
    printf 'ok partial-check baseline: valid IR byte identity, both-engine values, and every original primary refusal retained\n'
fi
if [[ -n $witness ]]; then
    rust_values=0
    rust_verdicts=0
    for mode in on off; do
        expected=42
        if [[ $mode == off ]]; then expected=17; fi
        for level in 0 3; do
            cp "$work/cases/valid.lkj" "$work/rust.lkj"
            printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
            "$witness" --edition=2021 --crate-name partial_valid -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || fail "Rust valid $mode/$level"
            [[ $("$work/rust") == "$expected" ]] || fail "Rust valid $mode/$level value"
            rust_values=$((rust_values + 1))
            for name in "${cases[@]}"; do
                cp "$work/cases/$name.lkj" "$work/rust.lkj"
                printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
                status=0
                "$witness" --edition=2021 --crate-name partial_case -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || status=$?
                case "$name" in
                    independent|unsafe_binding|library_inputs)
                        [[ $status == 0 && $("$work/rust") == 42 ]] || fail "Rust capability-boundary twin $name/$mode/$level"
                        ;;
                    local_import|global_static|mutable_static|qualified_header|global_macro|glob_import|trait_impl|type_failure|qualified_import|normalized_self|import_chain)
                        [[ $status != 0 ]] || fail "Rust accepted malformed context $name/$mode/$level"
                        ;;
                    *) fail "missing Rust oracle for $name" ;;
                esac
                rust_verdicts=$((rust_verdicts + 1))
            done
            cp "$work/cases/unsafe_binding.lkj" "$work/rust.lkj"
            printf '\nfn main() { println!("{}", unsafe_local()); }\n' >> "$work/rust.lkj"
            "$witness" --edition=2021 --crate-name partial_shadow -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error"
            [[ $("$work/rust") == 9 ]] || fail "Rust unsafe binding lost shadowing in $mode/$level"
            rust_values=$((rust_values + 1))
        done
    done
    printf 'ok partial-check Rust: %s values and %s context/boundary verdicts; opt0/opt3 and both configurations\n' "$rust_values" "$rust_verdicts"
fi
