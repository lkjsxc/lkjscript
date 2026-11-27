#!/usr/bin/env bash
# Exact import facts cannot authorize a refused source root.
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
work=$(mktemp -d "$PWD/target/partial-import.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for name in error verifier-error rust-error; do
            if [[ -s $work/$name ]]; then tail -n 35 "$work/$name" >&2; fi
        done
    fi
    if [[ ${LKJSCRIPT_PARTIAL_KEEP:-0} == 1 ]]; then printf 'retained_import_fixture=%s\n' "$work" >&2
    else rm -rf -- "$work"; fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'partial-import: %s\n' "$*" >&2; exit 1; }
cases=(import_replay import_ambiguity import_poison import_nonleak import_outer_local import_macro_local import_empty import_pattern import_pattern_hidden import_namespaces)
valids=(replay ambiguity poison nonleak pattern pattern-hidden namespaces)
mkdir "$work/cases"
for name in "${cases[@]}"; do cp "tests/fixtures/partial_check/$name.lkj" "$work/cases/"; done
verifier_compiler=${baseline:-$compiler}
"$verifier_compiler" build tests/fixtures/partial_import_verify.lkj --library-dir "$library" --no-store -o "$work/verify" > "$work/verifier-output" 2> "$work/verifier-error" || fail 'verifier build'
sed '/^fn refusal()/d' "$work/cases/import_replay.lkj" > "$work/replay-valid.lkj"
sed '/^fn refusal()/d' "$work/cases/import_pattern.lkj" > "$work/pattern-valid.lkj"
sed '/^fn refusal()/d' "$work/cases/import_namespaces.lkj" > "$work/namespaces-valid.lkj"
sed '/^fn refusal()/d' "$work/cases/import_pattern_hidden.lkj" > "$work/pattern-hidden-valid.lkj"
sed '/^    fn x()/d' "$work/pattern-hidden-valid.lkj" > "$work/pattern-projected-invalid.lkj"
sed '/^mod conflict {$/,/^}$/d; /^fn local_conflict()/,/^}$/d; /^fn refusal()/d' \
    "$work/cases/import_ambiguity.lkj" > "$work/ambiguity-valid.lkj"
sed '/^fn after()/,/^}$/d; /^fn sibling()/,/^}$/d; /^fn other()/d; /^fn refusal()/d' \
    "$work/cases/import_nonleak.lkj" > "$work/nonleak-valid.lkj"
sed 's/dyn Missing/i64/; /^fn probe()/d' "$work/cases/import_poison.lkj" > "$work/poison-valid.lkj"
cat >> "$work/poison-valid.lkj" <<'EOF'
mod absent { pub const thing: i64 = 0; }
fn probe() -> i64 {
    if local_bad() == 42 && local_glob() == 4 && missing_glob() == 42
        && missing_explicit() == 42 && contaminated::blocked() == 4
        && unrelated() == 42 { 42 } else { 0 }
}
EOF
for name in ambiguity poison nonleak; do
    sed '/^fn refusal()/d' "$work/cases/import_$name.lkj" > "$work/$name-invalid.lkj"
done
cp "$work/poison-invalid.lkj" "$work/poison-type-invalid.lkj"
printf '\nmod absent { pub const thing: i64 = 0; }\n' >> "$work/poison-type-invalid.lkj"
mkdir "$work/edit"
head -c 32 /dev/urandom > "$work/edit-key"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"probe","source":"{ 0i64 }"}]}' > "$work/edit-request"
checks=0
refusals=0
for mode in off on; do
    for name in "${valids[@]}"; do
        source="$work/$name-valid.lkj"
        expected=42
        "$compiler" ir "$source" --entry probe --library-dir "$library" --debug-assertions "$mode" --no-store -o "$work/$name-$mode.lkjir" > "$work/output" 2> "$work/error" || fail "$name valid $mode IR"
        for engine in native interpreter; do
            actual=$("$compiler" run "$source" --entry probe --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store 2> "$work/error") || fail "$name valid $mode/$engine"
            [[ $actual == "$expected" ]] || fail "$name valid $mode/$engine value"
            actual=$("$compiler" run "$work/$name-$mode.lkjir" --entry probe --engine "$engine" --no-store 2> "$work/error") || fail "$name IR $mode/$engine"
            [[ $actual == 42 ]] || fail "$name IR $mode/$engine value"
            checks=$((checks + 2))
        done
        if [[ -n $baseline ]]; then
            "$baseline" ir "$source" --entry probe --library-dir "$library" --debug-assertions "$mode" --no-store -o "$work/baseline.lkjir" > "$work/output" 2> "$work/error" || fail "$name baseline valid IR"
            cmp "$work/baseline.lkjir" "$work/$name-$mode.lkjir" || fail "$name $mode ordinary IR changed"
        fi
    done
    for name in "${cases[@]}"; do
        source="$work/cases/$name.lkj"
        status=0
        "$compiler" check "$source" --library-dir "$library" --debug-assertions "$mode" --no-store --stats > "$work/output" 2> "$work/error" || status=$?
        [[ $status == 1 && ! -s $work/output ]] || fail "$name $mode retained source refusal"
        "$work/verify" "$source" "$name" < "$work/error" > "$work/verified" 2> "$work/verifier-error" || fail "$name $mode exact item report"
        cat "$work/verified"
        if [[ $name == import_replay ]]; then
            sed 's/"status":"checked"/"status":"blocked"/' "$work/error" > "$work/mutated-report"
            if "$work/verify" "$source" "$name" < "$work/mutated-report" > "$work/mutant-output" 2> "$work/mutant-error"; then
                fail 'independent report verifier accepted a changed checked row'
            fi
        fi
        if [[ -n $baseline ]]; then
            status=0
            "$baseline" check "$source" --library-dir "$library" --debug-assertions "$mode" --no-store > "$work/baseline-output" 2> "$work/baseline-error" || status=$?
            [[ $status == 1 && ! -s $work/baseline-output ]] || fail "$name $mode baseline refusal"
            while IFS= read -r line; do
                [[ $line == *': E_'* && $line != *': E_BLOCKED '* ]] || continue
                grep -Fxq -- "$line" "$work/error" || fail "$name $mode lost an original diagnostic"
            done < "$work/baseline-error"
        fi
        for engine in native interpreter; do
            status=0
            "$compiler" run "$source" --entry probe --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store > "$work/output" 2> "$work/refused-error" || status=$?
            [[ $status == 1 && ! -s $work/output ]] || fail "$name $mode/$engine executed a refused root"
            ! grep -q '^LKJSCRIPT_ITEMS ' "$work/refused-error" || fail 'item report entered execution admission'
            refusals=$((refusals + 1))
        done
        for command in build ir; do
            printf 'unreplaced destination\n' > "$work/destination"
            cp "$work/destination" "$work/destination-before"
            status=0
            "$compiler" "$command" "$source" --entry probe --library-dir "$library" --debug-assertions "$mode" --no-store -o "$work/destination" > "$work/output" 2> "$work/refused-error" || status=$?
            [[ $status == 1 && ! -s $work/output ]] || fail "$name $mode published a refused root"
            cmp "$work/destination-before" "$work/destination" || fail "$name $mode changed refused destination"
            ! grep -q '^LKJSCRIPT_ITEMS ' "$work/refused-error" || fail 'item report entered artifact admission'
            refusals=$((refusals + 1))
        done
        cp "$source" "$work/edit/main.lkj"
        for edit_mode in dry-run apply; do
            edit_flags=()
            if [[ $edit_mode == dry-run ]]; then edit_flags=(--dry-run); fi
            status=0
            "$compiler" edit main.lkj --source-dir "$work/edit" --library-dir "$library" \
                --debug-assertions "$mode" --no-store --store-key "$work/edit-key" \
                --request "$work/edit-request" "${edit_flags[@]}" > "$work/output" 2> "$work/refused-error" || status=$?
            [[ $status == 1 && ! -s $work/output ]] || fail "$name $mode edit $edit_mode admitted a refused root"
            code=E_CAPABILITY
            if [[ $name == import_poison ]]; then code=E_UNSUPPORTED; fi
            grep -q ": $code " "$work/refused-error" || fail "$name $mode edit lost original refusal"
            cmp "$source" "$work/edit/main.lkj" || fail "$name $mode edit changed refused source"
            [[ ! -e $work/edit/.lkjscript-edit.pending ]] || fail "$name $mode edit left a pending transaction"
            refusals=$((refusals + 1))
        done
    done
    # Remove the unrelated capability refusal so the ordinary route must reach
    # ambiguity and absent lexical bindings themselves.
    for name in ambiguity nonleak poison pattern-projected; do
        code=E_NAME
        if [[ $name == poison ]]; then code=E_UNSUPPORTED; fi
        if [[ $name == pattern-projected ]]; then code=E_TYPE; fi
        for engine in native interpreter; do
            status=0
            "$compiler" run "$work/$name-invalid.lkj" --entry probe --library-dir "$library" --debug-assertions "$mode" --engine "$engine" --no-store > "$work/output" 2> "$work/refused-error" || status=$?
            [[ $status == 1 && ! -s $work/output ]] || fail "$name ordinary $mode/$engine accepted"
            grep -q "$code" "$work/refused-error" || fail "$name ordinary $mode/$engine wrong refusal"
            refusals=$((refusals + 1))
        done
    done
done
printf 'ok partial-import: both modes, exact slots/dependencies/causes/poison owners, %s source/IR value routes and %s execution/publication/edit/lookup refusals\n' "$checks" "$refusals"
if [[ -n $baseline ]]; then
    printf 'ok partial-import baseline: all valid twins preserve exact ordinary IR in both modes and every prior primary diagnostic remains\n'
fi
if [[ -n $witness ]]; then
    rust_values=0
    rust_verdicts=0
    rust_shadow_values=0
    rust_projection_values=0
    for mode in off on; do
        for level in 0 3; do
            for name in "${valids[@]}"; do
                expected=42
                cp "$work/$name-valid.lkj" "$work/rust.lkj"
                printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
                "$witness" --edition=2021 --crate-name partial_imports -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || fail "Rust $name valid $mode/$level"
                [[ $("$work/rust") == "$expected" ]] || fail "Rust $name valid $mode/$level value"
                rust_values=$((rust_values + 1))
            done
            for name in "${cases[@]}"; do
                cp "$work/cases/$name.lkj" "$work/rust.lkj"
                printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
                status=0
                "$witness" --edition=2021 --crate-name partial_imports -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || status=$?
                case "$name" in
                    import_replay|import_outer_local|import_macro_local|import_empty|import_pattern|import_pattern_hidden|import_namespaces)
                        [[ $status == 0 && $("$work/rust") == 42 ]] || fail "Rust capability-boundary $name $mode/$level"
                        ;;
                    import_ambiguity|import_nonleak|import_poison)
                        [[ $status != 0 ]] || fail "Rust accepted $name $mode/$level"
                        code=E0659
                        if [[ $name == import_nonleak ]]; then code=E0425; fi
                        if [[ $name == import_poison ]]; then code=E0432; fi
                        grep -Fq "error[$code]" "$work/rust-error" || fail "Rust wrong refusal for $name $mode/$level"
                        ;;
                esac
                rust_verdicts=$((rust_verdicts + 1))
            done
            for entry in macro_explicit macro_glob; do
                cp "$work/cases/import_macro_local.lkj" "$work/rust.lkj"
                printf '\nfn main() { println!("{}", %s()); }\n' "$entry" >> "$work/rust.lkj"
                "$witness" --edition=2021 --crate-name partial_imports -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || fail "Rust macro shadow boundary $entry $mode/$level"
                [[ $("$work/rust") == 10 ]] || fail "Rust macro shadow boundary $entry $mode/$level value"
                rust_shadow_values=$((rust_shadow_values + 1))
            done
            cp "$work/pattern-projected-invalid.lkj" "$work/rust.lkj"
            printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
            "$witness" --edition=2021 --crate-name partial_imports -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error" || fail "Rust changed pattern projection $mode/$level"
            [[ $("$work/rust") == 0 ]] || fail "Rust changed pattern projection $mode/$level value"
            rust_projection_values=$((rust_projection_values + 1))
            cp "$work/poison-type-invalid.lkj" "$work/rust.lkj"
            printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/rust.lkj"
            if "$witness" --edition=2021 --crate-name partial_imports -C "debug-assertions=$mode" -C "opt-level=$level" "$work/rust.lkj" -o "$work/rust" > "$work/rust-output" 2> "$work/rust-error"; then
                fail "Rust accepted isolated poisoned type $mode/$level"
            fi
            grep -Fq 'error[E0405]' "$work/rust-error" || fail "Rust isolated poison $mode/$level wrong refusal"
            rust_verdicts=$((rust_verdicts + 1))
        done
    done
    printf 'ok partial-import Rust: %s agreed values, %s context/boundary verdicts, %s macro-shadow and %s changed-projection boundary values; opt0/opt3 and both configurations\n' "$rust_values" "$rust_verdicts" "$rust_shadow_values" "$rust_projection_values"
fi
