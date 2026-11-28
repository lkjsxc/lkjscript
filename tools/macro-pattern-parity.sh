#!/usr/bin/env bash
# Pattern expansion uses ordinary lowering, with independently compiled Rust oracles.
set -euo pipefail
cd "$(dirname "$0")/.."
usage() { printf 'usage: %s [compiler [rustc]] | --rust-only rustc\n' "$0" >&2; exit 2; }
compiler=
witness=
if [[ ${1:-} == --rust-only ]]; then
    [[ $# == 2 ]] || usage
    witness=$2
else
    [[ $# -le 2 ]] || usage
    compiler=$(realpath "${1:-target/lkjscript}")
    witness=${2:-}
fi
if [[ -n $witness ]]; then witness=$(command -v "$witness"); fi
root=$PWD
jobs=${LKJSCRIPT_MACRO_PATTERN_JOBS:-2}
[[ $jobs =~ ^[1-4]$ ]] || usage
parent=${LKJSCRIPT_MACRO_PATTERN_OUTPUT:-$root/target}
mkdir -p "$parent"
work=$(mktemp -d "$parent/macro-patterns.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for file in "$work"/*.error "$work"/*.output; do
            [[ ! -s $file ]] || tail -n 35 "$file" >&2
        done
    fi
    if [[ -n ${LKJSCRIPT_MACRO_PATTERN_OUTPUT:-} ]]; then
        printf 'macro_pattern_results=%s\n' "$work"
    else
        rm -rf -- "$work"
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
fail() { printf 'macro-patterns: %s\n' "$*" >&2; exit 1; }
values=0
refusals=0
for source in tests/conformance/macro_patterns/*.lkj; do
    expectation=$(sed -n 's|^// expect: ||p' "$source")
    case "$expectation" in
        'value '*) values=$((values + 1)) ;;
        'reject '*) refusals=$((refusals + 1)) ;;
        *) fail "missing scalar/refusal expectation: $source" ;;
    esac
done
if [[ -n $compiler ]]; then
    for mode in off on; do
        for inline in 0 64; do
            "$compiler" test tests/conformance/macro_patterns --library-dir "$root/library" \
                --debug-assertions "$mode" --inline-limit "$inline" --jobs "$jobs" --no-store \
                > "$work/compiler-$mode-$inline.output" 2> "$work/compiler-$mode-$inline.error" \
                || fail "native/interpreter $mode inline=$inline"
            cat "$work/compiler-$mode-$inline.output"
        done
    done
    printf 'ok macro-patterns: %s value cases, %s refusal cases; native/interpreter in both debug configurations and inline limits 0/64\n' "$values" "$refusals"
fi
if [[ -n $witness ]]; then
    rust_values=0
    rust_refusals=0
    rust_boundaries=0
    printf 'case\tdebug_assertions\topt_level\tclassification\tresult\n' > "$work/rust-matrix.tsv"
    for mode in off on; do
        for level in 0 3; do
            for source in tests/conformance/macro_patterns/*.lkj; do
                name=${source##*/}
                expectation=$(sed -n 's|^// expect: ||p' "$source")
                cp "$source" "$work/rust.rs"
                printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.rs"
                status=0
                "$witness" --edition=2024 --crate-name macro_patterns -A warnings \
                    -C "debug-assertions=$mode" -C "opt-level=$level" -C overflow-checks=on \
                    "$work/rust.rs" -o "$work/rust" > "$work/rust.output" 2> "$work/rust.error" || status=$?
                if [[ $expectation == 'value '* ]]; then
                    [[ $status == 0 ]] || fail "Rust $name $mode/opt$level compile"
                    "$work/rust" > "$work/runtime.output" 2> "$work/runtime.error" || fail "Rust $name $mode/opt$level execution"
                    result=${expectation#value }
                    printf '%s\n' "$result" > "$work/expected"
                    cmp "$work/expected" "$work/runtime.output" || fail "Rust $name $mode/opt$level value"
                    [[ ! -s $work/runtime.error ]] || fail "Rust $name $mode/opt$level stderr"
                    classification=value
                    rust_values=$((rust_values + 1))
                elif grep -Eq '^// witness: .*"rustAccept":[[:space:]]*true' "$source"; then
                    # Explicit lkjscript boundaries remain separate from agreed refusals.
                    [[ $status == 0 ]] || fail "Rust $name $mode/opt$level expected accepted boundary"
                    result=$(sed -n 's|^// rust-value: ||p' "$source")
                    [[ -n $result ]] || fail "missing Rust boundary value: $source"
                    "$work/rust" > "$work/runtime.output" 2> "$work/runtime.error" || fail "Rust $name $mode/opt$level boundary execution"
                    printf '%s\n' "$result" > "$work/expected"
                    cmp "$work/expected" "$work/runtime.output" || fail "Rust $name $mode/opt$level boundary value"
                    [[ ! -s $work/runtime.error ]] || fail "Rust $name $mode/opt$level boundary stderr"
                    classification=boundary
                    rust_boundaries=$((rust_boundaries + 1))
                else
                    [[ $status == 1 ]] || fail "Rust $name $mode/opt$level expected refusal"
                    code=$(sed -n 's|^// witness: .*"rustCode":"\([^"]*\)".*|\1|p' "$source")
                    reason=$(sed -n 's|^// rust-error: ||p' "$source")
                    if [[ -n $code ]]; then
                        grep -Fq "error[$code]" "$work/rust.error" || fail "Rust $name $mode/opt$level wrong refusal"
                        result=$code
                    elif [[ -n $reason ]]; then
                        grep -Fq "$reason" "$work/rust.error" || fail "Rust $name $mode/opt$level wrong refusal"
                        result=$reason
                    else
                        grep -q '^error' "$work/rust.error" || fail "Rust $name $mode/opt$level missing diagnostic"
                        result=refused
                    fi
                    cp "$work/rust.error" "$work/$name-$mode-opt$level.diagnostic"
                    classification=refusal
                    rust_refusals=$((rust_refusals + 1))
                fi
                printf '%s\t%s\t%s\t%s\t%s\n' "$name" "$mode" "$level" "$classification" "$result" >> "$work/rust-matrix.tsv"
            done
        done
    done
    printf 'ok macro-patterns Rust: %s agreed values, %s agreed refusals and %s explicit lkjscript boundaries at opt0/opt3 in both debug configurations\n' "$rust_values" "$rust_refusals" "$rust_boundaries"
fi
