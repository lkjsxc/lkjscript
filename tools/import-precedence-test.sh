#!/usr/bin/env bash
# Lexical import/local precedence shares outcomes with the independent Rust compiler.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ $# -le 2 ]] || { printf 'usage: %s [compiler [rustc]]\n' "$0" >&2; exit 2; }
compiler=$(realpath "${1:-target/lkjscript}")
witness=${2:-}
if [[ -n $witness ]]; then witness=$(command -v "$witness"); fi
root=$PWD
mkdir -p target
work=$(mktemp -d "$root/target/import-precedence.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for name in output error rust-error rust-runtime-error; do
            if [[ -s $work/$name ]]; then tail -n 35 "$work/$name" >&2; fi
        done
    fi
    if [[ ${LKJSCRIPT_IMPORT_PRECEDENCE_KEEP:-0} == 1 ]]; then
        printf 'retained_import_precedence_fixture=%s\n' "$work" >&2
    else
        rm -rf -- "$work"
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'import-precedence: %s\n' "$*" >&2; exit 1; }
values=0
refusals=0
boundaries=0
for source in tests/conformance/import_precedence/*.lkj; do
    expectation=$(sed -n 's|^// expect: ||p' "$source")
    case "$expectation" in
        'value '*) values=$((values + 1)) ;;
        'reject '*)
            if grep -Fq '"rustAccept":true' "$source"; then
                boundaries=$((boundaries + 1))
            else
                refusals=$((refusals + 1))
            fi
            ;;
        *) fail "missing scalar/refusal expectation: $source" ;;
    esac
done
for mode in off on; do
    "$compiler" test tests/conformance/import_precedence --library-dir "$root/library" \
        --debug-assertions "$mode" --jobs 2 --no-store > "$work/output" 2> "$work/error" || fail "native/interpreter $mode"
    cat "$work/output"
    cp "$work/output" "$work/lkjscript-$mode.output"
    cp "$work/error" "$work/lkjscript-$mode.error"
    for source in tests/conformance/import_precedence/*.lkj; do
        grep -Fq '"rustAccept":true' "$source" || continue
        reason=$(sed -n 's|^// boundary: ||p' "$source")
        [[ -n $reason ]] || fail "missing boundary cause: $source"
        status=0
        "$compiler" check "$source" --library-dir "$root/library" --debug-assertions "$mode" \
            --no-store > "$work/output" 2> "$work/error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "boundary $source $mode"
        grep -Fq ": $reason" "$work/error" || fail "wrong boundary cause: $source $mode"
    done
done
printf 'ok import-precedence: %s value cases and %s refusal cases, native/interpreter and source/IR checks in both configurations\n' "$values" "$refusals"
printf 'ok import-precedence frontend boundaries: %s cases with exact causes, counted separately from Rust agreement\n' "$boundaries"
if [[ -n $witness ]]; then
    rust_values=0
    rust_refusals=0
    rust_boundaries=0
    printf 'case\tdebug_assertions\topt_level\tclassification\tresult\n' > "$work/rust-matrix.tsv"
    for mode in off on; do
        for level in 0 3; do
            for source in tests/conformance/import_precedence/*.lkj; do
                name=${source##*/}
                expectation=$(sed -n 's|^// expect: ||p' "$source")
                boundary=0
                if grep -Fq '"rustAccept":true' "$source"; then boundary=1; fi
                cp "$source" "$work/rust.rs"
                printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.rs"
                status=0
                "$witness" --edition=2021 --crate-name import_precedence -A warnings \
                    -C "debug-assertions=$mode" -C "opt-level=$level" -C overflow-checks=on \
                    "$work/rust.rs" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || status=$?
                if [[ $expectation == 'value '* || $boundary == 1 ]]; then
                    [[ $status == 0 ]] || fail "Rust $name $mode/opt$level compile"
                    "$work/rust" > "$work/rust-output" 2> "$work/rust-runtime-error" || fail "Rust $name $mode/opt$level execution"
                    if ((boundary)); then
                        expected=$(sed -n 's|^// rust-value: ||p' "$source")
                        [[ -n $expected ]] || fail "missing Rust boundary value: $name"
                        classification=boundary
                        rust_boundaries=$((rust_boundaries + 1))
                    else
                        expected=${expectation#value }
                        classification=value
                        rust_values=$((rust_values + 1))
                    fi
                    printf '%s\n' "$expected" > "$work/expected"
                    cmp "$work/expected" "$work/rust-output" || fail "Rust $name $mode/opt$level value"
                    [[ ! -s $work/rust-runtime-error ]] || fail "Rust $name $mode/opt$level stderr"
                    result=$expected
                else
                    [[ $status == 1 ]] || fail "Rust $name $mode/opt$level expected refusal"
                    code=$(sed -n 's|^// witness: .*"rustCode":"\([^"]*\)".*|\1|p' "$source")
                    [[ -n $code ]] || fail "missing Rust diagnostic code: $name"
                    grep -Fq "error[$code]" "$work/rust-error" || fail "Rust $name $mode/opt$level wrong refusal"
                    classification=refusal
                    result=$code
                    rust_refusals=$((rust_refusals + 1))
                fi
                printf '%s\t%s\t%s\t%s\t%s\n' "$name" "$mode" "$level" "$classification" "$result" >> "$work/rust-matrix.tsv"
            done
        done
    done
    printf 'ok import-precedence Rust: %s agreed values and %s matching refusals, opt0/opt3 and both configurations\n' "$rust_values" "$rust_refusals"
    printf 'ok import-precedence Rust frontend boundaries: %s accepted values, counted separately from agreement\n' "$rust_boundaries"
fi
