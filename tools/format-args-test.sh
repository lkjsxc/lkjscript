#!/usr/bin/env bash
# Borrowed format arguments retain checked loans, source identities and IR meaning.
set -euo pipefail
cd "$(dirname "$0")/.."
if (($# > 2)); then
    printf 'usage: %s [COMPILER [RUSTC]]\n' "$0" >&2
    exit 2
fi
compiler=$(realpath "${1:-target/lkjscript}")
witness=${2:-}
root=$PWD
library=$root/library
mkdir -p target
work=$(mktemp -d "$root/target/format-args.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for log in "$work/error" "$work/rust-error" "$work/plan"; do
            [[ ! -f $log ]] || sed -n '1,100p' "$log" >&2
        done
    fi
    if [[ ${LKJSCRIPT_FORMAT_ARGS_KEEP:-0} == 1 ]]; then
        rm -f -- "$work/key"
        printf 'retained_fixture=%s\n' "$work" >&2
    else
        rm -rf -- "$work"
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
umask 077
fail() { printf 'format-args: %s\n' "$*" >&2; exit 1; }

run_value() {
    local source=$1 expected=$2 engine=$3 limit=$4
    "$compiler" run "$source" --library-dir "$library" --entry probe --no-store \
        --engine "$engine" --inline-limit "$limit" > "$work/output" 2> "$work/error" \
        || fail "$source on $engine inline=$limit"
    [[ $(cat "$work/output") == "$expected" && ! -s $work/error ]] \
        || fail "$source returned unexpected output on $engine inline=$limit"
}

shopt -s nullglob
passes=(tests/conformance/pass/format_args*.lkj)
refusals=(tests/conformance/fail/format_args*.lkj)
(("${#passes[@]}" > 0 && "${#refusals[@]}" > 0)) || fail 'missing source fixtures'
routes=0
for limit in 0 64; do
    for engine in native interpreter; do
        for source in "${passes[@]}"; do
            run_value "$source" 42 "$engine" "$limit"
            routes=$((routes + 1))
        done
        for source in "${refusals[@]}"; do
            diagnostic=$(sed -n 's/^\/\/ expect: reject \(E_[A-Z_]*\)$/\1/p' "$source")
            [[ -n $diagnostic ]] || fail "$source lacks an exact refusal code"
            if "$compiler" run "$source" --library-dir "$library" --entry probe --no-store \
                --engine "$engine" --inline-limit "$limit" > "$work/output" 2> "$work/error"; then
                fail "accepted $source on $engine inline=$limit"
            fi
            [[ ! -s $work/output ]] && grep -Fq ": $diagnostic " "$work/error" \
                || fail "$source did not report $diagnostic on $engine inline=$limit"
            routes=$((routes + 1))
        done
    done
done
printf 'ok borrowed format_args source: %s pass cases, %s refusal cases, %s native/interpreter normal/inline routes\n' \
    "${#passes[@]}" "${#refusals[@]}" "$routes"

# Serialization must retain fat reference metadata and every private thunk edge.
for name in format_args_reuse format_args_nested format_args_errors; do
    for limit in 0 64; do
        "$compiler" ir "tests/conformance/pass/$name.lkj" --library-dir "$library" \
            --entry probe --no-store --inline-limit "$limit" -o "$work/$name.lkjir" \
            > "$work/output" 2> "$work/error" || fail "$name IR emission"
        for engine in native interpreter; do
            run_value "$work/$name.lkjir" 42 "$engine" "$limit"
        done
    done
done
printf 'ok borrowed format_args serialized IR: copies, distinct environments, nested invocations and partial failure\n'

# A longer name shifts the authored positions around the generated helper.
# Checked publication must keep helper identity structural and preserve references.
mkdir "$work/project" "$work/store"
project=$work/project
head -c 32 /dev/urandom > "$work/key"
cat > "$project/main.lkj" <<'SOURCE'
fn render(value: &i64) -> String {
    std::fmt::format(format_args!("value={}", value))
}
pub fn probe() -> i64 {
    let number = 42i64;
    let output = render(&number);
    if output == "value=42" { 42i64 }
    else if output == "changed=42" { 43i64 }
    else { 0i64 }
}
SOURCE
common=(main.lkj --source-dir "$project" --library-dir "$library" --entry probe
    --store "$work/store" --store-key "$work/key")
publish() {
    "$compiler" build "${common[@]}" --index --stats -o "$work/image" \
        > "$work/index" 2> "$work/error" || fail 'indexed build'
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$work/index")
    [[ ${#snapshot} == 64 ]] || fail 'missing indexed snapshot'
}
edit() {
    "$compiler" edit "${common[@]}" --request "$work/request" \
        > "$work/plan" 2> "$work/error" || fail 'checked edit'
    grep -Fq 'mode=apply' "$work/plan" || fail 'edit did not apply'
}
project_value() {
    local expected=$1
    for limit in 0 64; do
        for engine in native interpreter; do
            run_value "$project/main.lkj" "$expected" "$engine" "$limit"
        done
    done
}
refuse_edit() {
    local diagnostic=$1
    cp "$project/main.lkj" "$work/unchanged"
    cp "$project/.lkjscript-edit.generation" "$work/generation"
    if "$compiler" edit "${common[@]}" --request "$work/request" \
        > "$work/plan" 2> "$work/error"; then
        fail "published a transaction requiring $diagnostic"
    fi
    grep -Fq "$diagnostic" "$work/error" || fail "missing transaction diagnostic $diagnostic"
    cmp "$work/unchanged" "$project/main.lkj" || fail 'refused edit changed source'
    cmp "$work/generation" "$project/.lkjscript-edit.generation" \
        || fail 'refused edit changed generation'
    [[ ! -e $project/.lkjscript-edit.pending ]] || fail 'refused edit left pending state'
}

publish
project_value 42
cat > "$work/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"render","name":"render_deferred_arguments"}]}
REQUEST
edit
grep -Fq 'render_deferred_arguments(&number)' "$project/main.lkj" \
    || fail 'longer function rename missed its caller'
project_value 42
publish
cat > "$work/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"render_deferred_arguments::param:value","name":"borrowed_input_value"}]}
REQUEST
edit
grep -Fq 'format_args!("value={}", borrowed_input_value)' "$project/main.lkj" \
    || fail 'longer parameter rename missed its formatting argument'
project_value 42
publish
cat > "$work/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"render_deferred_arguments","source":"{ std::fmt::format(format_args!(\"changed={}\", borrowed_input_value)) }"}]}
REQUEST
edit
project_value 43
publish
accepted_snapshot=$snapshot
# A nested scope may not lend its owned argument or generated stack environment.
cat > "$work/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"render_deferred_arguments","source":"{ let arguments = { let short = String::from(\"short\"); format_args!(\"{}\", short) }; std::fmt::format(arguments) }"}]}
REQUEST
refuse_edit E_BORROW
publish
[[ $snapshot == "$accepted_snapshot" ]] || fail 'borrow rollback changed the snapshot'
project_value 43
cat > "$work/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"render_deferred_arguments","source":"{ std::fmt::format(format_args!(\"{}\"; borrowed_input_value)) }"}]}
REQUEST
refuse_edit E_EDIT_SYNTAX
publish
[[ $snapshot == "$accepted_snapshot" ]] || fail 'syntax rollback changed the snapshot'
project_value 43

# Implicit captures retain the established incomplete-source-reference refusal.
cat > "$work/request" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-body","id":"render_deferred_arguments","source":"{ std::fmt::format(format_args!(\"changed={borrowed_input_value}\")) }"}]}
REQUEST
edit
publish
"$compiler" query "${common[@]}" --snapshot "$snapshot" --kind meta \
    --id value@render_deferred_arguments --limit 32 --bytes 65536 \
    > "$work/capture-meta" 2> "$work/error"
grep -Eq '^"value@render_deferred_arguments".*references-complete=false' "$work/capture-meta" \
    || fail 'implicit capture owner claims complete editable references'
cat > "$work/request" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"render_deferred_arguments","name":"renamed_implicit_capture"}]}
REQUEST
refuse_edit E_EDIT_REFERENCES
project_value 43
printf 'ok borrowed format_args checked edits: longer function/parameter names, formatting body replacement, borrow/syntax rollback and implicit-capture refusal\n'

if [[ -n $witness ]]; then
    witness=$(command -v "$witness") || fail 'Rust witness compiler is unavailable'
    for source in "${passes[@]}" "${refusals[@]}"; do
        name=$(basename "$source" .lkj)
        if grep -Fq '"rustAccept":false' "$source"; then
            if "$witness" --edition=2024 --crate-type lib --crate-name "$name" \
                -A warnings "$source" -o "$work/refused.rlib" \
                > "$work/output" 2> "$work/rust-error"; then
                fail "Rust accepted refusal $name"
            fi
            grep -q '^error' "$work/rust-error" || fail "Rust failed without a source diagnostic for $name"
        else
            {
                cat "$source"
                printf '\nfn main() { assert_eq!(probe(), 42); }\n'
            } > "$work/witness.rs"
            for level in 0 3; do
                "$witness" --edition=2024 --crate-name "$name" -A warnings \
                    -C "opt-level=$level" -C overflow-checks=yes "$work/witness.rs" \
                    -o "$work/witness" > "$work/output" 2> "$work/rust-error" \
                    || fail "Rust compilation of $name opt=$level"
                "$work/witness" > "$work/output" 2> "$work/rust-error" \
                    || fail "Rust execution of $name opt=$level"
                [[ ! -s $work/output && ! -s $work/rust-error ]] \
                    || fail "Rust output for $name opt=$level"
            done
        fi
    done
    printf 'ok borrowed format_args Rust witness: opt0/opt3 values, source refusals and explicit as_str static-return boundary\n'
else
    printf 'format_args Rust witness skipped: pass RUSTC as the second argument\n'
fi
